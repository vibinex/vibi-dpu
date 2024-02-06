use std::collections::HashSet;
use sled::IVec;

use crate::db::config::get_db;
use crate::utils::user::{BitbucketUser, WorkspaceUser};


pub fn set_workspace_user_in_db(user: &WorkspaceUser) {
    let db = get_db();
    let user_key = user.display_name().to_owned();
    let value_res = serde_json::to_vec(&user);
    if value_res.is_err() {
        let e = value_res.expect_err("No error in value_res");
        log::error!("[set_workspace_user_in_db] Error in deserializing workspace user: {:?}, key: {}", e, &user_key);
        return;
    }
    let value = value_res.expect("Uncaught error in value_res");
    let insert_res = db.insert(user_key.clone(), value);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        log::error!("[set_workspace_user_in_db] Failed to upsert user into sled DB: {:?}, key: {}", e, &user_key);
        return;
    }
    log::debug!("[set_workspace_user_in_db] Workspace User succesfully upserted: {:?} at key: {}", user, &user_key);
}

pub fn get_workspace_user_from_db(user_key: &str) -> Option<WorkspaceUser> {
    let db = get_db();
    let get_res = db.get(IVec::from(user_key.to_string().as_bytes()));
    if get_res.is_err() {
        let e = get_res.expect("No error in get_res get_workspace_user_from_db");
        log::error!("[get_workspace_user_from_db] Unable to get workspace_user_from_db: {:?}", e);
        return None;
    }
	let user_opt = get_res.expect("Uncaught error in get_res workspace_user_from_db");
    if user_opt.is_none() {
        return None;
    }
	let user_ivec = user_opt.expect("Empty value");
    let user_res = serde_json::from_slice(&user_ivec);
    if user_res.is_err() {
        let e = user_res.expect_err("No error in user_res");
        log::error!("[get_workspace_user_from_db] Unable to deserialize workspace_user_from_db: {:?}", e);
        return None;
    }
    let user: WorkspaceUser = user_res.expect("Uncaught error in user_res");
    log::debug!("[get_workspace_user_from_db] workspace user from db = {:?}", &user);
    return Some(user);
}

pub fn add_bitbucket_user_to_workspace_user(bitbucket_user: BitbucketUser) -> Option<WorkspaceUser> {
    let user_key = bitbucket_user.display_name().to_owned();
    let user_res = get_workspace_user_from_db(&user_key);
    let mut user = WorkspaceUser::new(
        bitbucket_user.display_name().to_owned(),
        HashSet::new(),
    );
    if user_res.is_none() {
        eprintln!("Couldn't get workspace user from db, now inserting, key: {}", &user_key);
        user.users_mut().insert(bitbucket_user);
        set_workspace_user_in_db(&user);
        return Some(user);
    }
    user = user_res.expect("empty user");
    println!("Got user from db, user: {:?}", &user);
    user.users_mut().insert(bitbucket_user);
    set_workspace_user_in_db(&user);
    return Some(user);
}