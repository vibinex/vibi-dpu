use sled::IVec;

use crate::db::config::get_db;
use crate::utils::user::User;

pub fn save_user_to_db(user: &User) {
    let db = get_db();
    let provider_obj = user.provider();
    let user_key = format!("{}/{}/{}", 
        provider_obj.provider_type().to_string(), user.workspace(), provider_obj.id());
    println!("user_key = {}", &user_key);
  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(user).expect("Failed to serialize user");
  
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(user_key.as_bytes()), json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        eprintln!("Failed to upsert user into sled DB: {:?}", e);
        return;
    }
    println!("User succesfully upserted: {:?}", user);
}
pub fn user_from_db(repo_provider: &str, workspace: &str, user_id: &str, ) -> Option<User> {
	let db = get_db();
	let user_key = format!("{}/{}/{}", 
        repo_provider, workspace, user_id);
    let get_res = db.get(IVec::from(user_key.as_bytes()));
    if get_res.is_err() {
        let e = get_res.expect("No error in get_res user_from_db");
        eprintln!("Unable to get user_from_db: {:?}", e);
        return None;
    }
	let user_opt = get_res.expect("Uncaught error in get_res user_from_db");
    if user_opt.is_none() {
        return None;
    }
	let user_ivec = user_opt.expect("Empty value");
	let user: User = serde_json::from_slice::<User>(&user_ivec).unwrap();
	println!("user from db = {:?}", &user);
	return Some(user);
}