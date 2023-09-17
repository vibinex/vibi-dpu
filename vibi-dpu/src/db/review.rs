use sled::IVec;

use crate::db::config::get_db;
use crate::utils::review::Review;
pub fn save_review_to_db(review: &Review) {
    let db = get_db();
    let review_key = review.db_key().to_string();  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(review).expect("Failed to serialize repo");
    // Insert JSON into sled DB
    db.insert(IVec::from(review_key.as_bytes()), json).expect("Failed to upsert repo into sled DB");
}