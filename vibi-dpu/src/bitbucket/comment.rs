use serde::Serialize;

use crate::utils::review::Review;
use crate::utils::reqwest_client::get_client;
use super::config::{bitbucket_base_url, prepare_headers};

#[derive(Serialize)]
struct Comment {
    content: Content,
}

#[derive(Serialize)]
struct Content {
    raw: String,
}
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
        eprintln!("Error in post request for adding comment - {:?}", e);
        return;
    }
    let response = response_res.expect("Error in getting response");
    println!("response from comment post request = {:?}", &response);
}

fn prepare_add_comment_url(review: &Review) -> String {
    let url = format!(
        "{}/repositories/{}/{}/pullrequests/{}/comments",
        bitbucket_base_url(),
        review.repo_owner(),
        review.repo_name(),
        review.id()
    );
    println!("comment url = {}", &url);
    return url;
}
fn prepare_body(comment_text: &str) -> Comment {
    let comment_payload = Comment {
        content: Content {
            raw: comment_text.to_string(),
        },
    };
    return comment_payload;
}