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
    // TODO - check comment setting
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
    let mut coverage_map_obj = CoverageMap::new(repo_provider);
    coverage_map_obj.calculate_coverage_map(relevance_vec, reviewer_handles);
    // add up contribution of aliases
    // add comment
    let comment_text = approval_comment_text(&coverage_map_obj);
    // get access token and call add_comment in gh/bb
}

fn approval_comment_text(coverage_map: &CoverageMap) -> String {
    let mut comment = "Relevant users for this PR:\n\n".to_string();  // Added two newlines
    let coverage_text = coverage_map.generate_coverage_table();
    comment += &coverage_text;
    comment += "\n\n";
    comment += "If you are a relevant reviewer, you can use the [Vibinex browser extension](https://chromewebstore.google.com/detail/vibinex-code-review/jafgelpkkkopeaefadkdjcmnicgpcncc) to see parts of the PR relevant to you\n";  // Added a newline at the end
    comment += "Relevance of the reviewer is calculated based on the git blame information of the PR. To know more, hit us up at contact@vibinex.com.\n\n";  // Added two newlines
    comment += "To change comment and auto-assign settings, go to [your Vibinex repository settings page.](https://vibinex.com/u)\n";  // Added a newline at the end
    return comment;
}