use serde_json::Value;

use crate::db::review::get_review_from_db;

pub async fn process_approval(deserialised_msg_data: &Value) {
    log::debug!("[process_approval] processing approval msg - {:?}", deserialised_msg_data);
    // parse approval message
    // get reviewer login array by getting pr all reviewer info from gh/bb
    // get coverage map aliases and their corresponding logins from db/server
    // let review_opt = get_review_from_db();
    // add up contribution of aliases
    // add comment
}