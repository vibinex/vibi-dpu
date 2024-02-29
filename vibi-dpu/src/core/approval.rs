use serde_json::Value;

use crate::{db::review::get_review_from_db, utils::{coverage::CoverageMap, user::ProviderEnum}};

pub async fn process_approval(deserialised_msg_data: &Value) {
    log::debug!("[process_approval] processing approval msg - {:?}", deserialised_msg_data);
    let repo_owner = deserialised_msg_data["eventPayload"]["repository"]["owner"]["login"].to_string().trim_matches('"').to_string();
    let repo_name = deserialised_msg_data["eventPayload"]["repository"]["name"].to_string().trim_matches('"').to_string();
    let pr_number = deserialised_msg_data["eventPayload"]["pull_request"]["number"].to_string().trim_matches('"').to_string();
    let repo_provider = deserialised_msg_data["repositoryProvider"].to_string().trim_matches('"').to_string();
    if repo_provider == ProviderEnum::Github.to_string().to_lowercase() {}
    // get reviewer login array by getting pr all reviewer info from gh/bb
    let reviewer_handles = Vec::<String>::new();
    // get relevance map aliases and their corresponding logins from db/server
    let review_opt = get_review_from_db(&repo_name, &repo_owner, &repo_provider, &pr_number);
    if review_opt.is_none() {
        log::error!("[process_approval] Unable to get review from db");
        return;
    }
    let review = review_opt.expect("Empty review_opt");
    let relevance_vec_opt = review.relevance();
    if relevance_vec_opt.is_none() {
        log::error!("[process_approval] Unable to get coverage from db");
        return;
    }
    let relevance_vec = relevance_vec_opt.to_owned().expect("Empty coverage_opt");
    let coverage_map_obj = CoverageMap::calculate_coverage_map(repo_provider, reviewer_handles, relevance_vec);
    // add up contribution of aliases
    // add comment
}