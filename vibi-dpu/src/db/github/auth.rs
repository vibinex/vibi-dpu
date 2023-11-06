use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use sled::IVec;

use crate::db::config::get_db;
use crate::utils::github_auth_info::GithubAuthInfo;

pub fn save_github_auth_info_to_db(auth_info: &mut GithubAuthInfo) {
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
    let insert_res = db.insert("github_auth_info", ivec);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        eprintln!("Failed to upsert github auth info into sled DB: {e}");
        return;
    }
    println!("GithubAuthInfo succesfully upserted: {:?}", auth_info);
}

pub fn github_auth_info() -> Option<GithubAuthInfo> {
    let db = get_db();
	let authinfo_key = "github_auth_info";
	let authinfo_res = db.get(IVec::from(authinfo_key.as_bytes()));
    if authinfo_res.is_err() {
        let e = authinfo_res.expect_err("No error in authinfo_res");
        eprintln!("Unable to get github authinfo from db: {:?}", e);
        return None;
    }
    let authinfo_opt = authinfo_res.expect("Uncaught error in authinfo_res");
    if authinfo_opt.is_none() {
        eprintln!("No github authinfo in db");
        return None;
    }
    let authinfo_ivec = authinfo_opt.expect("Empty authinfo_opt");
    let authinfo_parse = serde_json::from_slice(&authinfo_ivec);
    if authinfo_parse.is_err() {
        let e = authinfo_parse.expect_err("No error in authinfo_parse");
        eprintln!("Unable to deserialize github authinfo_parse: {:?}", e);
        return None;
    }
	let github_auth_info: GithubAuthInfo =  authinfo_parse.expect("Uncaught error in authinfo_parse");
    return Some(github_auth_info);
}