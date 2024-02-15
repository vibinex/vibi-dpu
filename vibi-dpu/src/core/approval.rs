use serde_json::Value;

pub async fn process_approval(deserialised_msg_data: &Value) {
    log::debug!("[process_approval] processing approval msg - {:?}", deserialised_msg_data);
    // parse approval message
    // get reviewer login array by getting pr all reviewer info from gh/bb
    // get coverage map aliases and their corresponding logins from db/server
    // add up contribution of aliases
    // add comment
}