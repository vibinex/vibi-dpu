use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use sled::IVec;

use crate::db::config::get_db;
use crate::utils::auth::AuthInfo;

pub fn save_auth_info_to_db(auth_info: &mut AuthInfo) {
    let db = get_db();
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");  
    auth_info.set_timestamp(since_epoch.as_secs());
    println!("auth info = {:?}", &auth_info);
    let json = serde_json::to_string(&auth_info).expect("Failed to serialize auth info");
    // Convert JSON string to bytes
    let bytes = json.as_bytes(); 

    // Create IVec from bytes
    let ivec = IVec::from(bytes);

    // Insert into sled DB
    db.insert("bitbucket_auth_info", ivec).expect("Failed to insert auth info in sled database");
}

pub fn auth_info() -> AuthInfo {
    let db = get_db();
	let authinfo_key = "bitbucket_auth_info";
	let authinfo_ivec = db.get(IVec::from(authinfo_key.as_bytes()))
		.expect("Unable to get bb authinfo from db")
		.expect("Empty bitbucket authinfo in db");
	let authinfo: AuthInfo =  
		serde_json::from_slice(&authinfo_ivec)
        .expect("Unable to deserialize authinfo");
    return authinfo;
}