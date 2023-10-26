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


pub async fn update_pr_info_in_db(workspace_slug: &str, repo_slug: &str, pr_info: PrInfo, pr_number: &str) {
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