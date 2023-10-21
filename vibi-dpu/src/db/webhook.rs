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
    let parse_res = serde_json::to_vec(webhook);
    if parse_res.is_err() {
        let e = parse_res.expect_err("No error in parse_res in save_webhook_to_db");
        eprintln!("Failed to serialize webhook: {:?}", e);
        return;
    }
    let webhook_json = parse_res.expect("Uncaught error in parse_res webhook");
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(id), webhook_json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        eprintln!("Failed to upsert webhook into sled DB: {e}");
        return;
    }
    println!("Webhook succesfully upserted: {:?}", webhook);
}