use serde_json::json;

use crate::{utils::{review::Review, reqwest_client::get_client}, github::config::{github_base_url, prepare_headers}};

pub async fn add_reviewers(reviewers: &Vec<String>, review: &Review, access_token: &str) {
    let url = format!("{}/repos/{}/{}/pulls/{}/requested_reviewers",
        github_base_url(),
        review.repo_owner(),
        review.repo_name(),
        review.id());
    let headers_opt = prepare_headers(access_token);
    if headers_opt.is_none() {
        return;
    }
    let headers = headers_opt.expect("Empty headers_opt");
    let body_json = json!({
        "reviewers": reviewers
    });
    let body = body_json.to_string();
    println!("[add_reviewers] url = {:?}, body = {:?}", &url, &body);
    let client = get_client();
    let response_res = client.post(url).headers(headers).body(body).send().await;
    if response_res.is_err() {
        let e = response_res.expect_err("No error in response_res");
        eprintln!("[add_reviewers] Unable to add reviewers: {:?}, {:?}", e, &reviewers);
        return;
    }
    let response = response_res.expect("Uncaught error in response_res");
    println!("Added reviewers, response: {:?}", response.text().await);
}