use serde_json::{json, Value};

use crate::{github::config::{github_base_url, prepare_headers}, utils::{review::Review, reqwest_client::get_client}};

pub async fn add_comment(comment_text: &str, review: &Review, access_token: &str) {
    let url = prepare_add_comment_url(review);
    let comment_payload = prepare_body(comment_text);
    let client = get_client();
    let headers_opt = prepare_headers(&access_token);
    if headers_opt.is_none() {
        eprintln!("Unable to prepare_headers_comment, empty headers_opt");
        return;
    }
    let headers = headers_opt.expect("Empty headers_opt");
    let response_res = client.post(&url).
        headers(headers).json(&comment_payload).send().await;
    if response_res.is_err() {
        let e = response_res.expect_err("No error in response_res");
        eprintln!("[github/add_comment] Error in post request for adding comment - {:?}", e);
        return;
    }
    let response = response_res.expect("Error in getting response");
    println!("[github/add_comment] response from comment post request = {:?}", &response);
}

fn prepare_add_comment_url(review: &Review) -> String {
    let url = format!(
        "{}/repos/{}/{}/issues/{}/comments",
        github_base_url(),
        review.repo_owner(),
        review.repo_name(),
        review.id()
    );
    println!("comment url = {}", &url);
    return url;
}

fn prepare_body(comment_text: &str) -> Value {
    return json!({
        "body": comment_text
    });
}

// def add_pr_comment(repo_name, repo_owner, pr_number, access_token, body):
// 	url = f"{github_api}/repos/{repo_owner}/{repo_name}/issues/{pr_number}/comments"
// 	headers = {
// 		"Authorization": f"Bearer {access_token}",
// 		"Accept": "application/vnd.github+json"
// 	}
// 	response = requests.post(url, headers=headers, json={"body": body})
// 	if response.status_code != 201:
// 		raise Exception(f"Error adding comment on pr {pr_number}: {response.status_code}, {response.content.decode()}")
// 	return response