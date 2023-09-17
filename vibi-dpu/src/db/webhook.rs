use sled::IVec;
use uuid::Uuid;

use crate::db::config::get_db;
use crate::utils::webhook::Webhook;
pub fn save_webhook_to_db(webhook: &Webhook) {
    let db = get_db();
    // Generate unique ID
    let uuid = Uuid::new_v4();
    let id = uuid.as_bytes();
    // Serialize webhook struct to JSON
    let json = serde_json::to_vec(webhook).expect("Failed to serialize webhook");
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(id), json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        eprintln!("Failed to upsert webhook into sled DB: {e}");
        return;
    }
    let insert_output = insert_res.expect("Uncaught error in insert_res");
    println!("Webhook succesfully upserted: {:?}", &insert_output);
}