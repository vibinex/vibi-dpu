use serde_json::Value;
use sled::IVec;
use std::error::Error;
use crate::db::config::get_db;
use crate::utils::pr_info::PrInfo;


pub async fn save_pr_info_to_db(workspace_slug: &str,repo_slug: &str,pr_info: PrInfo, pr_number: &str) {
    let db = get_db();
    let key = format!("{}/{}/{}/{}", "bitbucket", workspace_slug, repo_slug, pr_number);

    let pr_info_bytes = serde_json::to_vec(&pr_info);
    if pr_info_bytes.is_err() {
        let e = pr_info_bytes.expect_err("Empty error in pr_info_bytes");
		eprintln!("Unable to serialize pr info: {:?}, error: {:?}", pr_info, e);
		return;
    }
    let pr_info_json = pr_info_bytes.expect("Uncaught error in parse_res repo");

    let insert_result = db.insert(key.as_bytes(), IVec::from(pr_info_json)); 
    if insert_result.is_err() {
        let e = insert_result.expect_err("No error in inserting pr_info");
        eprintln!("Failed to insert PR info into the database. {:?}", e);
        return;
    }

    println!("PR succesfully upserted: {:?} {:?}", key, pr_info);
}


pub async fn update_pr_info_in_db(workspace_slug: &str, repo_slug: &str, pr_info: &PrInfo, pr_number: &str) {
    let key = format!("{}/{}/{}/{}", "bitbucket", workspace_slug, repo_slug, pr_number);
    let db = get_db();

    let pr_info_json_result = serde_json::to_vec(&pr_info);

    if pr_info_json_result.is_err() {
        let e = pr_info_json_result.expect_err("Empty error in pr_info_bytes");
        eprintln!("Failed to serialize PR info: {:?}", e);
        return;
    }

    let pr_info_bytes = pr_info_json_result.expect("empty pr_info_json_result");

    // Update the entry in the database. It will create a new entry if the key does not exist.
    let update_result = db.insert(IVec::from(key.as_bytes()), IVec::from(pr_info_bytes));

    if update_result.is_err() {
        let e = update_result.expect_err("No error in updating pr_info");
        eprintln!("Failed to update PR info in the database: {:?}", e);
        return;
    }

    println!("PR info updated successfully in the database. {:?} {:?}", key, pr_info);
}

pub async fn process_and_update_pr_if_different(webhook_data: &Value, workspace_slug: &str, repo_slug: &str, pr_number: &str, repo_provider: &str) -> bool {
    let pr_info_parsed_opt = parse_webhook_data(webhook_data);
    if pr_info_parsed_opt.is_none() {
        eprintln!("[process_and_update_pr_if_different] Unable to parse webhook data");
        return false;
    }
    let pr_info_parsed = pr_info_parsed_opt.expect("Empty pr_info_parsed_opt");
    // Retrieve the existing pr_head_commit from the database
    let pr_info_db_opt = get_pr_info(workspace_slug, repo_slug, pr_number, repo_provider, &pr_info_parsed).await;
    if pr_info_db_opt.is_none() {
        eprintln!("[process_and_update_pr_if_different] No pr_info in db, parsed: {:?}", pr_info_parsed);
        return false;
    }
    let pr_info_db = pr_info_db_opt.expect("Empty pr_info_db_opt");
    if pr_info_db.pr_head_commit().to_string().eq_ignore_ascii_case(pr_info_parsed.pr_head_commit()){
        return false; // commits are the same
    } else {
        update_pr_info_in_db(&workspace_slug, &repo_slug, &pr_info_parsed, &pr_number).await;
        return true; // commits are different, and PR info should be updated
    }
}

fn parse_webhook_data(webhook_data: &Value) -> Option<PrInfo> {
    let pr_head_commit_opt = webhook_data
        .get("pull_request")
        .and_then(|src| src.get("source"))//branch, commit, hash
        .and_then(|branch| branch.get("branch"))
        .and_then(|commit| commit.get("commit"))
        .and_then(|hash_obj| hash_obj.get("hash"))
        .and_then(|sha| sha.as_str());
    if pr_head_commit_opt.is_none() {
        eprintln!("[parse_webhook_data] pr_head_commit_opt not found: {:?}", webhook_data);
        return None;
    }
    let pr_head_commit = pr_head_commit_opt.expect("Empty pr_head_commit_opt");

    let base_head_commit_opt = webhook_data
        .get("pull_request")
        .and_then(|dest| dest.get("destination"))//branch, commit, hash
        .and_then(|branch| branch.get("branch"))
        .and_then(|commit| commit.get("commit"))
        .and_then(|hash_obj| hash_obj.get("hash"))
        .and_then(|sha| sha.as_str());
    if base_head_commit_opt.is_none() {
        eprintln!("[parse_webhook_data] base_head_commit_opt not found: {:?}", webhook_data);
        return None;
    }
    let base_head_commit = base_head_commit_opt.expect("Empty base_head_commit_opt");

    let pr_state_opt = webhook_data
        .get("pull_request")
        .and_then(|pr| pr.get("state"))
        .and_then(|state| state.as_str());
    if pr_state_opt.is_none() {
        eprintln!("[parse_webhook_data] pr_state_opt not found: {:?}", webhook_data);
        return None;
    }
    let pr_state = pr_state_opt.expect("Empty pr_state_opt");
    let pr_branch_opt = webhook_data
        .get("pull_request")
        .and_then(|src| src.get("source"))//branch, commit, hash
        .and_then(|branch| branch.get("branch"))
        .and_then(|commit| commit.get("name"))
        .and_then(|branch_name| branch_name.as_str());
    if pr_branch_opt.is_none() {
        eprintln!("[parse_webhook_data] pr_branch_opt not found: {:?}", webhook_data);
        return None;
    }
    let pr_branch = pr_branch_opt.expect("Empty pr_branch_opt");
    let pr_info = PrInfo { base_head_commit: base_head_commit.to_string(),
        pr_head_commit: pr_head_commit.to_string(),
        state: pr_state.to_string(),
        pr_branch: pr_branch.to_string()
    };
    println!("[parse_webhook_data] pr_info :{:?}", &pr_info);
    return Some(pr_info);
}

pub async fn get_pr_info(workspace_slug: &str, repo_slug: &str, pr_number: &str, repo_provider: &str, pr_info_parsed: &PrInfo) -> Option<PrInfo> {
    let db = get_db();
    let db_pr_key = format!("{}/{}/{}/{}", repo_provider, workspace_slug, repo_slug, pr_number);
    let pr_info_res = db.get(IVec::from(db_pr_key.as_bytes()));

    if pr_info_res.is_err() {
        let e = pr_info_res.expect_err("No error in pr_info_res");
        eprintln!("Unable to get bb pr info from db: {:?}", e);
        return None;
    };

    let pr_info_opt = pr_info_res.expect("Uncaught error in pr_info res");
    if pr_info_opt.is_none() {
        eprintln!("No bitbucket pr info in db");
        update_pr_info_in_db(&workspace_slug, &repo_slug, pr_info_parsed, &pr_number).await;
        return None; //If no info in db then it will be considered as new commit
    }

    let pr_info_ivec = pr_info_opt.expect("Empty pr_info_opt");
    let pr_info_parse = serde_json::from_slice(&pr_info_ivec);
    if pr_info_parse.is_err() {
        let e = pr_info_parse.expect_err("No error in pr_info_parse");
        eprintln!("Unable to deserialize pr_Info: {:?}", e);
        return None;
    }
    let pr_info: PrInfo = pr_info_parse.expect("Failed to deserialize PR info");
    return Some(pr_info);
}