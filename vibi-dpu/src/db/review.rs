use sled::IVec;

use crate::db::config::get_db;
use crate::utils::review::Review;
pub fn save_review_to_db(review: &Review) {
    let db = get_db();
    let review_key = format!("review/{}", review.db_key());
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

pub fn get_review_from_db(repo_name: &str, repo_owner: &str,
        repo_provider: &str, review_id: &str) -> Option<Review> {
    let db = get_db();
    let review_key = format!("review/{}/{}/{}/{}",
        repo_provider, repo_owner, repo_name, review_id);
    let review_res = db.get(IVec::from(review_key.as_bytes()));
    if let Err(e) = review_res {
        log::error!("[get_review_from_db] Review key not found in db - {}, error: {:?}",
            &review_key, e);
        return None;
    }
    let ivec_opt = review_res.expect("Uncaught error in review_res");
    log::debug!("[get_review_from_db] ivec_opt: {:?}", ivec_opt);
    if ivec_opt.is_none() {
        log::error!("[get_review_from_db] No review found for {}/{}", repo_name, review_id);
        return None;
    }
    let ivec = ivec_opt.expect("Empty ivec_opt");
    let review_res = serde_json::from_slice(&ivec);
    if let Err(e) = review_res {
        log::error!(
            "[get_review_from_db] Failed to deserialize review from json: {:?}",
            e
        );
        return None;
    }
    let review: Review = review_res.expect("Uncaught error in review_res");
    return Some(review);
}