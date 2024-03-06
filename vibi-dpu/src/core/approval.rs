use serde_json::Value;

use crate::core::utils::get_access_token;
use crate::github;
use crate::{db::review::get_review_from_db, utils::user::ProviderEnum};


pub async fn process_approval(deserialised_msg_data: &Value,
        repo_owner: &str, repo_name: &str, pr_number: &str, repo_provider: &str) {
    log::debug!("[process_approval] processing approval msg - {:?}", deserialised_msg_data);
    let pr_head_commit = deserialised_msg_data["review"]["commit_id"]
        .to_string().trim_matches('"').to_string();
    let review_opt = get_review_from_db(&repo_name,
        &repo_owner, &repo_provider, &pr_number);
    if review_opt.is_none() {
        log::error!("[process_approval] Unable to get review from db");
        return;
    }
    let review = review_opt.expect("Empty review_opt");

    let access_token= get_access_token(&review).await;
	if access_token.is_none(){
		log::error!("[process_approval] no final access token opt");
		return;
	}
	let final_access_token = access_token.expect("Empty final access token opt");

    // get reviewer login array by getting pr all reviewer info from gh/bb
    let mut reviewer_handles = Vec::<String>::new();
    if repo_provider == ProviderEnum::Github.to_string().to_lowercase() {
        let reviewer_handles_opt = github::prs::pr_reviewer_handles(
            &repo_owner, &repo_name, &pr_number, &pr_head_commit, &final_access_token).await;
        if reviewer_handles_opt.is_none(){
            log::error!("[process_approval] no reviewers handles opt");
            return;
        }
        reviewer_handles = reviewer_handles_opt.expect("Empty reviewer_handles_opt");

    } //TODO: create a similar function for bitbucket too
    // get coverage map aliases and their corresponding logins from db/server
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
