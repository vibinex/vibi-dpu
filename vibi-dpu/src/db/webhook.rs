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
    db.insert(IVec::from(id), json).expect("Failed to insert webhook into sled DB");
}