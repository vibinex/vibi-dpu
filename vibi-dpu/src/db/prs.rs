use serde_json::Value;
use sled::IVec;
use crate::core::approval::process_approval;
use crate::db::config::get_db;
use crate::utils::pr_info::PrInfo;

pub async fn update_pr_info_in_db(workspace_slug: &str, repo_slug: &str, pr_info: &PrInfo, pr_number: &str, repo_provider: &str) {
    let key = format!("pr_info/{}/{}/{}/{}", repo_provider, workspace_slug, repo_slug, pr_number);
    let db = get_db();

    let pr_info_json_result = serde_json::to_vec(&pr_info);

    if pr_info_json_result.is_err() {
        let e = pr_info_json_result.expect_err("Empty error in pr_info_bytes");
        eprintln!("Failed to serialize PR info: {:?}", e);
        return;
    }

    let pr_info_bytes = pr_info_json_result.expect("empty pr_info_json_result");

    // Update the entry in the database. It will create a new entry if the key does not exist.
    let update_result = db.insert(IVec::from(key.as_bytes()), pr_info_bytes);

    if update_result.is_err() {
        let e = update_result.expect_err("No error in updating pr_info");
        log::error!("[update_pr_info_in_db] Failed to update PR info in the database: {:?}", e);
        return;
    }

    log::debug!("[update_pr_info_in_db] PR info updated successfully in the database. {:?} {:?}", key, pr_info);
}

pub async fn bitbucket_process_and_update_pr_if_different(webhook_data: &Value, workspace_slug: &str, repo_slug: &str, pr_number: &str, repo_provider: &str) -> bool {
    log::debug!("[bitbucket_process_and_update_pr_if_different] {:?}, {:?}, {:?}, {:?}", workspace_slug, repo_slug, pr_number, repo_provider);
    let pr_info_parsed_opt = parse_bitbucket_webhook_data(webhook_data);
    if pr_info_parsed_opt.is_none() {
        log::error!("[bitbucket_process_and_update_pr_if_different] Unable to parse webhook data");
        return false;
    }
    let pr_info_parsed = pr_info_parsed_opt.expect("Empty pr_info_parsed_opt");
    // Retrieve the existing pr_head_commit from the database
    log::debug!("[process_and_update_pr_if_different|get_pr_info_from_db] workspace_slug: {}, repo_slug: {},  pr_number: {}, pr_info_parsed: {:?}", &workspace_slug, &repo_slug,  &pr_number, &pr_info_parsed); // todo: remove
    let pr_info_db_opt = get_pr_info_from_db(workspace_slug, repo_slug, pr_number, repo_provider, &pr_info_parsed).await;
    if pr_info_db_opt.is_none() {
        log::error!("[bitbucket_process_and_update_pr_if_different] No pr_info in db, parsed: {:?}", pr_info_parsed);
        return true; // new pr
    }
    let pr_info_db = pr_info_db_opt.expect("Empty pr_info_db_opt");
    if pr_info_db.pr_head_commit().to_string().eq_ignore_ascii_case(pr_info_parsed.pr_head_commit()){
        return false; // commits are the same
    } else {
        log::debug!("[process_and_update_pr_if_different|update_pr_info_in_db] workspace_slug: {}, repo_slug: {}, pr_info_parsed: {:?}, pr_number: {}", &workspace_slug, &repo_slug, &pr_info_parsed, &pr_number);
        update_pr_info_in_db(&workspace_slug, &repo_slug, &pr_info_parsed, &pr_number, repo_provider).await;
        return true; // commits are different, and PR info should be updated
    }
}

fn parse_bitbucket_webhook_data(webhook_data: &Value) -> Option<PrInfo> {
    println!("[parse_bitbucket_webhook_data] webhook_data: {:?}", &webhook_data);
    let pr_head_commit_raw = webhook_data["pullrequest"]["source"]["commit"]["hash"].to_string();
    let pr_head_commit = pr_head_commit_raw.trim_matches('"');
    let base_head_commit_raw = webhook_data["pullrequest"]["destination"]["commit"]["hash"].to_string();
    let base_head_commit = base_head_commit_raw.trim_matches('"');
    let pr_state_raw = webhook_data["pullrequest"]["state"].to_string();
    let pr_state = pr_state_raw.trim_matches('"');
    let pr_branch_raw = webhook_data["pullrequest"]["source"]["branch"]["name"].to_string();
    let pr_branch = pr_branch_raw.trim_matches('"');
    let pr_info = PrInfo { base_head_commit: base_head_commit.to_string(),
        pr_head_commit: pr_head_commit.to_string(),
        state: pr_state.to_string(),
        pr_branch: pr_branch.to_string()
    };
    println!("[parse_webhook_data] pr_info :{:?}", &pr_info);
    return Some(pr_info);
}

pub async fn get_pr_info_from_db(workspace_slug: &str, repo_slug: &str, pr_number: &str, repo_provider: &str, pr_info_parsed: &PrInfo) -> Option<PrInfo> {
    let db = get_db();
    let db_pr_key = format!("pr_info/{}/{}/{}/{}", repo_provider, workspace_slug, repo_slug, pr_number);
    let pr_info_res = db.get(IVec::from(db_pr_key.as_bytes()));

    if pr_info_res.is_err() {
        let e = pr_info_res.expect_err("No error in pr_info_res");
        eprintln!("Unable to get bb pr info from db: {:?}", e);
        return None;
    };

    let pr_info_opt = pr_info_res.expect("Uncaught error in pr_info res");
    if pr_info_opt.is_none() {
        eprintln!("No {} pr info in db", repo_provider);
        update_pr_info_in_db(&workspace_slug, &repo_slug, pr_info_parsed, &pr_number, repo_provider).await;
        return None; //If no info in db then it will be considered as new commit
    }

    let pr_info_ivec = pr_info_opt.expect("Empty pr_info_opt");
    println!("[get_pr_info_from_db] pr_info_ivec = {:?}", &pr_info_ivec);

    let pr_info_parse = serde_json::from_slice(&pr_info_ivec);
    println!("[get_pr_info_from_db] pr_info_parse = {:?}", &pr_info_parse);

    if pr_info_parse.is_err() {
        let e = pr_info_parse.expect_err("No error in pr_info_parse");
        eprintln!("Unable to deserialize pr_Info: {:?}", e);
        return None;
    }
    let pr_info: PrInfo = pr_info_parse.expect("Failed to deserialize PR info");
    return Some(pr_info);
}

pub async fn github_process_and_update_pr_if_different(webhook_data: &Value, repo_config: &Value, repo_owner: &str, repo_name: &str, pr_number: &str, repo_provider: &str) -> bool {
    println!("[github_process_and_update_pr_if_different] {:?}, {:?}, {:?}, {:?}", repo_owner, repo_name, pr_number, repo_provider);
    let event_action = webhook_data["action"].to_string().trim_matches('"').to_string();

    let pr_info_parsed_opt = parse_github_webhook_data(webhook_data);
    if pr_info_parsed_opt.is_none() {
        eprintln!("[github_process_and_update_pr_if_different] Unable to parse webhook data");
        return false;
    }
    let pr_info_parsed = pr_info_parsed_opt.expect("Empty pr_info_parsed_opt");

    if event_action == "opened" {
        // new PR opened
        println!("[github_process_and_update_pr_if_different|new_pr_opened] {:?}", pr_info_parsed);
        update_pr_info_in_db(&repo_owner, &repo_owner, &pr_info_parsed, &pr_number, repo_provider).await;
        return true;
    }
    if event_action == "synchronize" {
        println!("[github_process_and_update_pr_if_different| event_action synchronise] pr_info_parsed: {:?}", &pr_info_parsed);
        update_pr_info_in_db(&repo_owner, &repo_name, &pr_info_parsed, &pr_number, repo_provider).await;
        return true // commits are different, and PR info should be updated
    } 
    if event_action == "submitted" {
        let event_review_status = webhook_data["review"]["state"].to_string().trim_matches('"').to_string();
        if event_review_status == "approved" {
            println!("[github_process_and_update_pr_if_different| pr has been approved] webhook data for pr {:?}", &webhook_data);
            update_pr_info_in_db(&repo_owner, &repo_name, &pr_info_parsed, &pr_number, repo_provider).await;
            process_approval(webhook_data, repo_config, repo_owner, repo_name, pr_number, repo_provider).await;
            return false;
        } else {
            println!("[github_process_and_update_pr_if_different|no_update_needed] event is not approved");
            return false; // event is not open or synchronize
        }
    } else {
        println!("[github_process_and_update_pr_if_different | no update needed] event is not opened or synchronise or approved");
        return false;
    }

}

fn parse_github_webhook_data(webhook_data: &Value) -> Option<PrInfo> {
    println!("[parse_github_webhook_data] webhook_data: {:?}", &webhook_data);
    let pr_head_commit = webhook_data["pull_request"]["head"]["sha"].to_string().trim_matches('"').to_string();
    let base_head_commit = webhook_data["pull_request"]["base"]["sha"].to_string().trim_matches('"').to_string();
    let pr_state = webhook_data["pull_request"]["state"].to_string().trim_matches('"').to_string();
    let pr_branch = webhook_data["pull_request"]["head"]["ref"].to_string().trim_matches('"').to_string();
    let pr_info = PrInfo { base_head_commit: base_head_commit,
        pr_head_commit: pr_head_commit,
        state: pr_state,
        pr_branch: pr_branch,
    };
    println!("[parse_github_webhook_data] pr_info :{:?}", &pr_info);
    return Some(pr_info)
}
