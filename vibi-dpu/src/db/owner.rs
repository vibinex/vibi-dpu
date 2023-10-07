use sled::IVec;

use crate::db::config::get_db;
use crate::utils::owner::Workspace;

pub fn save_workspace_to_db(workspace: &Workspace) {
    let uuid = workspace.uuid().clone();
    let db = get_db();
    let json = serde_json::to_string(&workspace).expect("Failed to serialize workspace");
    // Convert JSON string to bytes
    let bytes = json.as_bytes(); 
    // Create IVec from bytes
    let ivec = IVec::from(bytes);
    db.insert(format!("owners:{}", uuid), ivec).expect("Unable to save workspace in db");  
}