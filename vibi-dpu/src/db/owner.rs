use sled::IVec;

use crate::db::config::get_db;
use crate::utils::owner::Workspace;

pub fn save_workspace_to_db(workspace: &Workspace) {
    let uuid = workspace.uuid().clone();
    let db = get_db();
    let parse_json = serde_json::to_string(&workspace);
    if parse_json.is_err() {
        let e = parse_json
            .expect_err("No error in parse_json for workspace");
        eprintln!("Failed to serialize workspace: {:?}", e);
        return;
    }
    let json = parse_json.expect("Uncaught error in parse_json workspace");
    // Convert JSON string to bytes
    let bytes = json.as_bytes(); 
    // Create IVec from bytes
    let ivec = IVec::from(bytes);
    let insert_res = db.insert(format!("owners:{}", uuid), ivec);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        eprintln!("Failed to upsert workspace into sled DB: {e}");
        return;
    }
    let insert_output = insert_res.expect("Uncaught error in insert_res");
    println!("Workspace succesfully upserted: {:?}", &insert_output);  
}