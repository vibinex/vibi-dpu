use serde_json::Value;

use crate::{db::review::get_review_from_db, utils::user::ProviderEnum};

pub async fn process_approval(deserialised_msg_data: &Value) {
    log::debug!("[process_approval] processing approval msg - {:?}", deserialised_msg_data);
    let repo_owner = deserialised_msg_data["eventPayload"]["repository"]["owner"]["login"].to_string().trim_matches('"').to_string();
    let repo_name = deserialised_msg_data["eventPayload"]["repository"]["name"].to_string().trim_matches('"').to_string();
    let pr_number = deserialised_msg_data["eventPayload"]["pull_request"]["number"].to_string().trim_matches('"').to_string();
    let repo_provider = deserialised_msg_data["repositoryProvider"].to_string().trim_matches('"').to_string();
    if repo_provider == ProviderEnum::Github.to_string().to_lowercase() {}
    // get reviewer login array by getting pr all reviewer info from gh/bb
    let reviewer_handles = Vec::<String>::new();
    // get coverage map aliases and their corresponding logins from db/server
    let review_opt = get_review_from_db(&repo_name, &repo_owner, &repo_provider, &pr_number);
    if review_opt.is_none() {
        log::error!("[process_approval] Unable to get review from db");
        return;
    }
    let review = review_opt.expect("Empty review_opt");
    let coverage_opt = review.coverage();
    if coverage_opt.is_none() {
        log::error!("[process_approval] Unable to get coverage from db");
        return;
    }
    let coverage = coverage_opt.to_owned().expect("Empty coverage_opt");
    let mut curr_coverage = 0;
    for (git_alias, (coverage_val, handles_opt)) in coverage {
        if handles_opt.is_none() {
            log::debug!("[process_approval] handles not in db for {}", &git_alias);
            continue;
        }
        let handles = handles_opt.expect("Empty handles_opt");
        for handle in handles {
            if reviewer_handles.contains(&handle) {
                // curr_coverage += coverage_val
            }
        }
    }
    // add up contribution of aliases
    // add comment
}