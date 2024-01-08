use sled::IVec;

use crate::db::config::get_db;
use crate::utils::review::Review;
pub fn save_review_to_db(review: &Review) {
    let db = get_db();
    let review_key = review.db_key().to_string();  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(review).expect("Failed to serialize review");
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(review_key.as_bytes()), json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        log::error!("[save_review_to_db] Failed to upsert review into sled DB: {e}");
        return;
    }
    log::debug!("[save_review_to_db] Review succesfully upserted: {:?}", review);
}