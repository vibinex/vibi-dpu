use sled::IVec;
use std::error::Error;
use serde_json::Value;
use crate::db::config::get_db;
use crate::utils::prInfo::prInfo;


pub async fn save_pr_info_to_db(workspace_slug: &str,repo_slug: &str, pr_info: prInfo, pr_number: &str) {
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


pub async fn update_pr_info_in_db(workspace_slug: String, repo_slug: String, pr_info: prInfo, pr_number: String) {
    let key = format!("{}/{}/{}/{}", "bitbucket", workspace_slug, repo_slug, pr_number);
    let db = get_db();

    let pr_info_json_result = serde_json::to_vec(&pr_info);

    if pr_info_json_result.is_err() {
        let e = pr_info_json_result.expect_err("Empty error in pr_info_bytes");
        eprintln!("Failed to serialize PR info: {:?}", e);
        return;
    }

    let pr_info_bytes = pr_info_json_result.unwrap();

    // Update the entry in the database. It will create a new entry if the key does not exist.
    let update_result = db.insert(IVec::from(key.as_bytes()), IVec::from(pr_info_bytes));

    if update_result.is_err() {
        let e = update_result.expect_err("No error in updating pr_info");
        eprintln!("Failed to update PR info in the database: {:?}", e);
        return;
    }

    println!("PR info updated successfully in the database. {:?} {:?}", key, pr_info);
}

pub async fn process_and_update_pr_if_different(webhook_data: &Value, workspace_slug: String, repo_slug: String, pr_number: String, repo_provider: String) -> Result<bool, String> {
    let pr_head_commit = webhook_data
        .get("pull_request")
        .and_then(|pr| pr.get("head"))
        .and_then(|head| head.get("sha"))
        .and_then(|sha| sha.as_str())
        .ok_or("Missing pr_head_commit")?
        .to_string();
    
    let base_head_commit = webhook_data
        .get("pull_request")
        .and_then(|pr| pr.get("base"))
        .and_then(|base| base.get("sha"))
        .and_then(|sha| sha.as_str())
        .ok_or("Missing base_head commit")?
        .to_string();

    let pr_state = webhook_data
        .get("pull_request")
        .and_then(|pr| pr.get("state"))
        .and_then(|state| state.as_str())
        .ok_or("Missing pr_state")?
        .to_string();

    let pr_branch = webhook_data
        .get("pull_request")
        .and_then(|pr| pr.get("head"))
        .and_then(|head| head.get("ref"))
        .and_then(|ref_val| ref_val.as_str())
        .ok_or("Missing pr_branch")?
        .to_string();

    let updated_pr_info = prInfo { base_head_commit: base_head_commit,
        pr_head_commit: pr_head_commit.clone(),
        state: pr_state,
        pr_branch: pr_branch 
    };

    // Retrieve the existing pr_head_commit from the database
    let db = get_db();
    let db_pr_key = format!("{}/{}/{}/{}", repo_provider, workspace_slug, repo_slug, pr_number);
    let pr_info_res = db.get(IVec::from(db_pr_key.as_bytes()));
    
    if pr_info_res.is_err() {
        let e = pr_info_res.expect_err("No error in pr_info_res");
        eprintln!("Unable to get bb pr info from db: {:?}", e);
        return Err("Database retrieval failed".to_string());
    };

    let pr_info_opt = pr_info_res.expect("Uncaught error in pr_info res");
    if pr_info_opt.is_none() {
        eprintln!("No bitbucket pr info in db");
        update_pr_info_in_db(workspace_slug, repo_slug, updated_pr_info, pr_number).await;
        return Ok(true); //If no info in db then it will be considered as new commit
    }
    
    let pr_info_ivec = pr_info_opt.expect("Empty pr_info_opt");
    let pr_info_parse = serde_json::from_slice(&pr_info_ivec);
    if pr_info_parse.is_err() {
        let e = pr_info_parse.expect_err("No error in pr_info_parse");
        eprintln!("Unable to deserialize pr_Info: {:?}", e);
        return Err("Failed to deserialize PR info".to_string());
    }
    let pr_info: prInfo = pr_info_parse.expect("Failed to deserialize PR info");
    let stored_pr_head_commit_str = pr_info.pr_head_commit;
    // Compare with the one in webhook data
    if pr_head_commit == stored_pr_head_commit_str{
        Ok(false) // commits are the same
    } else {
        update_pr_info_in_db(workspace_slug, repo_slug, updated_pr_info, pr_number).await;
        Ok(true) // commits are different, and PR info should be updated
    }
}