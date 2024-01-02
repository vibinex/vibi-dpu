use sled::IVec;

use crate::db::config::get_db;
use crate::utils::repo::Repository;

pub fn save_repo_to_db(repo: &Repository) {
    let db = get_db();
    let repo_key = format!("{}/{}/{}", repo.provider(), repo.workspace(), repo.name());
    log::debug!("[save_repo_to_db] repo_key = {}",  &repo_key);
  
    // Serialize repo struct to JSON 
    let parse_res = serde_json::to_vec(repo);
	if parse_res.is_err() {
		let e = parse_res.expect_err("Empty error in parse_res");
		log::error!("[save_repo_to_db] Unable to serialize repo: {:?}, error: {:?}", repo, e);
		return;
	}
	let repo_json = parse_res.expect("Uncaught error in parse_res repo");
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(repo_key.as_bytes()), repo_json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        log::error!("[save_repo_to_db] Failed to upsert repo into sled DB: {:?}", e);
        return;
    }
    log::debug!("[save_repo_to_db] Repo succesfully upserted: {:?}", repo);
}

pub fn get_clone_url_clone_dir(repo_provider: &str, workspace_name: &str, repo_name: &str) -> Option<(String, String)> {
	let db = get_db();
	let key = format!("{}/{}/{}", repo_provider, workspace_name, repo_name);
	let repo_res = db.get(IVec::from(key.as_bytes()));
	if repo_res.is_err() {
		let e = repo_res.expect_err("No error in repo_res");
		log::error!("[get_clone_url_clone_dir] Unable to get repo from db: {:?}", e);
		return None;
	}
	let repo_opt = repo_res.expect("Uncaught error in repo_res");
	if repo_opt.is_none() {
		log::error!("[get_clone_url_clone_dir] Empty repo_opt from db");
		return None;
	}
	let repo_ivec = repo_opt.expect("Empty repo_opt");
	let parse_res = serde_json::from_slice::<Repository>(&repo_ivec);
	if parse_res.is_err() {
		let e = parse_res.expect_err("No error in parse_res repo");
		log::error!("[get_clone_url_clone_dir] error in deserializing repo from db: {:?}", e);
		return None;
	}
	let repo: Repository = parse_res.expect("Uncaught error in parse_res");
	log::debug!("[get_clone_url_clone_dir] repo = {:?}", &repo);
	let clone_dir_opt = repo.local_dir().to_owned();
	if clone_dir_opt.is_none() {
		log::error!("[get_clone_url_clone_dir] Empty clone_dir_opt in db, repo: {:?}", &repo);
		return None;
	}
	let clone_dir = clone_dir_opt.expect("Empty clone_dir_opt");
	let clone_url = repo.clone_ssh_url().to_string();
	return Some((clone_url, clone_dir));
}