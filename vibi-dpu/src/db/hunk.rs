use sled::IVec;

use crate::db::config::get_db;
use crate::utils::hunk::HunkMap;
use crate::utils::review::Review;
pub fn get_hunk_from_db(review: &Review) -> Option<HunkMap> {
	let db = get_db();
	let key = format!("{}/{}/{}", review.db_key(), 
		review.base_head_commit(), review.pr_head_commit());
	let hunkmap_val = db.get(&key);
	match hunkmap_val {
		Ok(hunkmap_val) => {
			match hunkmap_val {
				Some(hunkmap_json) => {
					match serde_json::from_slice(&hunkmap_json) {
						Ok(hunkmap) => {
							return Some(hunkmap);},
						Err(e) => {eprintln!("Error deserializing hunkmap: {}", e);},
					};
				}, None => {eprintln!("No hunkmap stored in db for key: {}", &key)}
			};
		}, Err(e) => {eprintln!("Error getting hunkmap from db, key: {}, err: {e}", &key);}
	};
	return None;
}

pub fn store_hunkmap_to_db(hunkmap: &HunkMap, review: &Review) {
    let db = get_db();
	let key = format!("{}/{}/{}", review.db_key(), review.base_head_commit(), review.pr_head_commit());
	println!("key = {}", key);
	let json = serde_json::to_vec(hunkmap).expect("Failed to serialize repo");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(key.as_bytes()), json).expect("Failed to upsert repo into sled DB");
}
