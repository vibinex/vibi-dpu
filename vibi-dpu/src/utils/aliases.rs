use crate::{core::utils::get_handles_from_server, db::aliases::get_handles_from_db};

use super::review::Review;


pub async fn get_login_handles(git_alias: &str, review: &Review) -> Option<Vec<String>> {
    // Get aliases from db
    let handles_db_opt = get_handles_from_db(git_alias, &review.provider());
    if handles_db_opt.is_none() {
        let server_aliases_opt = get_handles_from_server(review).await;
        log::debug!("[get_login_handles] server_aliases_opt = {:?}", &server_aliases_opt);
        if server_aliases_opt.is_none() {
            log::error!("[get_login_strings] No login strings found for git alias {} from server or db", git_alias);
            return None;
        }
        let server_alias_map = server_aliases_opt.expect("Empty server_aliases_opt");
        let login_handles_opt = server_alias_map.get(git_alias);
        if login_handles_opt.is_none() {
            log::error!("[get_login_handles] Unable to get login handles from server_alias_map - {:?}",
                &server_alias_map);
            return None;
        }
        let login_handles = login_handles_opt.expect("Empty login_handles_opt");
        return Some(login_handles.to_owned());
    }
    log::debug!("[get_login_handles] handles_db_opt = {:?}", &handles_db_opt);
    let handles = handles_db_opt.expect("Empty handles_db_opt");
    return Some(handles);
}