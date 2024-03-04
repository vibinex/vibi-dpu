use sled::IVec;

use crate::db::config::get_db;
use crate::db::hunk;
use crate::utils::hunk::HunkMap;
use crate::utils::review::Review;
pub fn get_hunk_from_db(review: &Review) -> Option<HunkMap> {
	let db = get_db();
	let key = format!("hunk/{}/{}/{}", review.db_key(), 
		review.base_head_commit(), review.pr_head_commit());
	let hunkmap_res = db.get(&key);
	if hunkmap_res.is_err() {
		let e = hunkmap_res.expect_err("No error in hunkmap_res");
		log::error!("[get_hunk_from_db] Error getting hunkmap from db, key: {:?}, err: {:?}", &key, e);
		return None;
	}
	let hunkmap_opt = hunkmap_res.expect("Uncaught error in hunkmap_res");
	if hunkmap_opt.is_none() {
		log::error!("[get_hunk_from_db] No hunkmap stored in db for key: {}", &key);
		return None;
	}
	let hunkmap_ivec = hunkmap_opt.expect("Empty hunkmap_opt");
	let hunkmap_res = serde_json::from_slice(&hunkmap_ivec);
	if hunkmap_res.is_err() {
		let e = hunkmap_res.expect_err("No error in hunkmap_res");
		log::error!("[get_hunk_from_db] Error deserializing hunkmap: {:?}", e);
		return None;
	}
	let hunkmap = hunkmap_res.expect("Uncaught error in hunkmap_res");
	return Some(hunkmap);
}

pub fn store_hunkmap_to_db(hunkmap: &HunkMap, review: &Review) {
	log::debug!("[store_hunkmap_to_db]: hunkmap: {:?}, review: {:?}", &hunkmap, &review);
    let db = get_db();
	let hunk_key = format!("hunk/{}/{}/{}", review.db_key(), review.base_head_commit(), review.pr_head_commit());
	log::debug!("[store_hunkmap_to_db] hunk_key = {}", hunk_key);
	let json = serde_json::to_vec(hunkmap).expect("Failed to serialize hunkmap");
  
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(hunk_key.as_bytes()), json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        log::error!("[store_hunkmap_to_db] Failed to upsert hunkmap into sled DB: {e}");
        return;
    }
    log::debug!("[store_hunkmap_to_db] Hunkmap succesfully upserted: {:?}", hunkmap);
}
