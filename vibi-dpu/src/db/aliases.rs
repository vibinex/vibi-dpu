use sled::IVec;
use std::collections::HashSet;
use crate::db::config::get_db;

fn save_handles_to_db(alias_key: &str, provider: &str, provider_login_ids: Vec<String>) {
    let db = get_db();
    let full_key = format!("{}/aliases/{}", provider, alias_key);
    log::debug!("[save_handles_to_db] full_key = {}", &full_key);
    let val_res = serde_json::to_vec(&provider_login_ids);
    if let Err(e) = val_res {
        log::error!(
            "[save_handles_to_db] Failed to serialize provider login ids to json: {:?}",
            e
        );
        return;
    }
    let val = val_res.expect("Uncaught error in val_res");
    let insert_res = db.insert(
        IVec::from(full_key.as_bytes()),
        val,
    );
    if let Err(e) = insert_res {
        log::error!(
            "[save_handles_to_db] Failed to upsert aliases into sled DB: {:?}",
            e
        );
        return;
    } 
    log::debug!(
        "[save_handles_to_db] Aliases successfully upserted for provider: {}",
        provider
    );
}

pub fn get_handles_from_db(alias_key: &str, provider: &str) -> Option<Vec<String>> {
    let db = get_db();
    let full_key = format!("{}/aliases/{}", provider, alias_key);
    log::debug!("[get_handles_from_db] full_key = {}", &full_key);
    
    let result = db.get(IVec::from(full_key.as_bytes()));
    if let Err(e) = result {
        log::error!(
            "[get_handles_from_db] Failed to retrieve aliases from sled DB: {:?}",
            e
        );
        return None;
    }
    let ivec_opt = result.expect("Uncaught error in result");
    if ivec_opt.is_none() {
        log::debug!("[get_handles_from_db] No aliases found for provider: {}", provider);
        return None;
    }
    let ivec = ivec_opt.expect("Empty ivec_opt");
    let alias_res = serde_json::from_slice(&ivec);
    if let Err(e) = alias_res {
        log::error!(
            "[get_handles_from_db] Failed to deserialize aliases from json: {:?}",
            e
        );
        return None;
    }
    let alias: Vec<String> = alias_res.expect("Uncaught error in alias_res");
    return Some(alias);
}

pub fn update_handles_in_db(alias_key: &str, provider: &str, new_logins: Vec<String>) -> Vec<String>{
    // Retrieve existing aliases from the database
    let existing_aliases = match get_handles_from_db(alias_key, provider) {
        Some(aliases) => aliases,
        None => Vec::new(), // If no aliases found, initialize as empty vector
    };

    // Convert both existing and new logins into sets for easy set operations
    let mut aliases_set: HashSet<String> = existing_aliases.into_iter().collect();
    let new_logins_set: HashSet<String> = new_logins.into_iter().collect();

    // Perform a set union to combine existing aliases and new logins
    aliases_set.extend(new_logins_set);

    // Convert the set back into a vector
    let updated_aliases: Vec<String> = aliases_set.into_iter().collect();

    // Save the updated aliases back to the database
    save_handles_to_db(alias_key, provider, updated_aliases.to_owned());
    return updated_aliases;
}

