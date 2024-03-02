use serde_json::Value;

use crate::{db::review::get_review_from_db, utils::user::ProviderEnum};
use crate::github::config::{prepare_headers, get_access_token_based_on_env_values};
use crate::utils::reqwest_client::get_client;



pub async fn process_approval(deserialised_msg_data: &Value) {
    log::debug!("[process_approval] processing approval msg - {:?}", deserialised_msg_data);
    let repo_owner = deserialised_msg_data["eventPayload"]["repository"]["owner"]["login"].to_string().trim_matches('"').to_string();
    let repo_name = deserialised_msg_data["eventPayload"]["repository"]["name"].to_string().trim_matches('"').to_string();
    let pr_number = deserialised_msg_data["eventPayload"]["pull_request"]["number"].to_string().trim_matches('"').to_string();
    let repo_provider = deserialised_msg_data["repositoryProvider"].to_string().trim_matches('"').to_string();
    let pr_head_commit = deserialised_msg_data["eventPayload"]["commit_id"].to_string().trim_matches('"').to_string();

    let review_opt = get_review_from_db(&repo_name, &repo_owner, &repo_provider, &pr_number);
    if review_opt.is_none() {
        log::error!("[process_approval] Unable to get review from db");
        return;
    }
    let review = review_opt.expect("Empty review_opt");

    let access_token= get_access_token_based_on_env_values(&review).await;
	if access_token.is_none(){
		log::error!("[process_approval] no final access token opt");
		return;
	}
	let final_access_token = access_token.expect("Empty final access token opt");

    // get reviewer login array by getting pr all reviewer info from gh/bb
    let mut reviewer_handles = Vec::<String>::new();
    if repo_provider == ProviderEnum::Github.to_string().to_lowercase() {
        let reviewer_handles_opt = get_reviewers_login_handles_for_github_pr(&repo_owner, &repo_name, &pr_number, &pr_head_commit, &final_access_token).await;
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
    let mut curr_coverage = 0;
    for relevance_obj in relevance_vec {
        let handles_opt = relevance_obj.handles();
        if handles_opt.is_none() {
            log::debug!("[process_approval] handles not in db for {}", relevance_obj.git_alias());
            continue;
        }
        let handles = handles_opt.to_owned().expect("Empty handles_opt");
        for handle in handles {
            if reviewer_handles.contains(&handle) {
                // curr_coverage += coverage_val
            }
        }
    }
    // add up contribution of aliases
    // add comment
}

pub async fn get_reviewers_login_handles_for_github_pr(repo_owner: &str, repo_name: &str, pr_number: &str, pr_head_commit: &str, access_token: &str) -> Option<Vec<String>> {
    let headers_opt = prepare_headers(access_token);
    if headers_opt.is_none() {
        log::error!("[get_reviewers_login_handles_for_github_pr] Unable to prepare auth headers for repository: {}", repo_name);
        return None;
    }
    let headers = headers_opt.expect("Headers should be present");
    let client = get_client();
    let response_result = client
        .get(&format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
            repo_owner, repo_name, pr_number
        ))
        .headers(headers)
        .send()
        .await;

    if response_result.is_err() {
		let e = response_result.expect_err("No error in sending request");
		log::error!("[get_reviewers_login_handles_for_github_pr] Failed to send the request: {:?}", e);
		return None;
	}

	let response = response_result.expect("Uncaught error in parsing response");
    if !response.status().is_success() {
        log::error!(
            "[get_reviewers_login_handles_for_github_pr] Error in retrieving review list: {:?}",
            response.status()
        );
        return None;
    }

    let parse_result = response.json::<Vec<Value>>().await;
    if parse_result.is_err() {
		let e = parse_result.expect_err("No error in parsing");
		log::error!(
			"[get_reviewers_login_handles_for_github_pr] Failed to parse JSON: {:?}",
			e
		);
		return None;
	}
	let reviewr_list_result = parse_result.expect("Uncaught error in parsing reviewers list data");

    // Initialize a vector to store reviewer handles
    let mut reviewer_handles = Vec::new();

    // Process the review list
    for review in reviewr_list_result {
        let state = review["state"].as_str().unwrap_or_default();
        let commit_id = review["commit_id"].as_str().unwrap_or_default();
        if state == "APPROVED" && commit_id == pr_head_commit {
            // Extract reviewer login
            if let Some(login) = review["user"]["login"].as_str() {
                reviewer_handles.push(login.to_string());
            }
        }
    }

    Some(reviewer_handles)
}