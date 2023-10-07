use sled::IVec;

use crate::db::config::get_db;
use crate::utils::repo::Repository;

pub fn save_repo_to_db(repo: &Repository) {
    let db = get_db();
    let repo_key = format!("{}/{}/{}", repo.provider(), repo.workspace(), repo.name());
    println!("repo_key = {}", &repo_key);
  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(repo).expect("Failed to serialize repo");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(repo_key.as_bytes()), json).expect("Failed to upsert repo into sled DB");
}

pub fn get_clone_url_clone_dir(repo_provider: &str, workspace_name: &str, repo_name: &str) -> (String, String) {
	let db = get_db();
	let key = format!("{}/{}/{}", repo_provider, workspace_name, repo_name);
	let repo_opt = db.get(IVec::from(key.as_bytes())).expect("Unable to get repo from db");
	let repo_ivec = repo_opt.expect("Empty value");
	let repo: Repository = serde_json::from_slice::<Repository>(&repo_ivec).unwrap();
	println!("repo = {:?}", &repo);
	let clone_dir = repo.local_dir().to_owned().expect("No local dir for repo set in db");
	let clone_url = repo.clone_ssh_url().to_string();
	return (clone_url, clone_dir);
}