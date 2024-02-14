use crate::{core::utils::get_aliases, db::aliases::get_handles_from_db};

use super::review::Review;


pub async fn get_login_handles(git_alias: &str, review: &Review) -> Option<Vec<String>> {
    // Get aliases from db
    let handles_opt = get_handles_from_db(git_alias, &review.provider());
    if handles_opt.is_none() {
        let server_aliases_opt = get_aliases(review).await;
        if server_aliases_opt.is_none() {
            log::debug!("[get_login_strings] No login strings found for git alias {} from server or db", git_alias);
            return None;
        }
        let server_alias_map = server_aliases_opt.expect("Empty server_aliases_opt");
        let login_handles = &server_alias_map[git_alias];
        return Some(login_handles.to_owned());
    }
    let handles = handles_opt.expect("Empty handles_opt");
    return Some(handles);
}