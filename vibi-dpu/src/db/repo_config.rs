use sled::IVec;

use crate::db::config::get_db;
use crate::utils::repo_config::RepoConfig;

pub fn save_repo_config_to_db(repo_config: &RepoConfig, 
    repo_name: &str, repo_owner: &str, repo_provider: &str) {
    let db = get_db();
    let config_key = format!("{}/{}/{}/config", repo_provider, repo_owner, repo_name);
    println!("config_key = {}", &config_key);
    
    // Serialize repo struct to JSON 
    let parse_res = serde_json::to_vec(repo_config);
    if parse_res.is_err() {
        let e = parse_res.expect_err("Empty error in parse_res in save_repo_config_to_db");
        eprintln!("Unable to serialize repo in save_repo_config_to_db: {:?}, error: {:?}", &repo_config, e);
        return;
    }
    let config_json = parse_res.expect("Uncaught error in parse_res save_repo_config_to_db");
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(config_key.as_bytes()), config_json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res save_repo_config_to_db");
        eprintln!("Failed to upsert repo config into sled DB: {:?}", e);
        return;
    }
    println!("Repo Config succesfully upserted: {:?}", repo_config);
}