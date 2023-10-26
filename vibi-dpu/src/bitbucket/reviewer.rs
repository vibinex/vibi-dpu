use reqwest::Response;
use serde_json::Value;

use crate::utils::review::Review;
use crate::utils::user::BitbucketUser;

use super::config::{get_client, prepare_headers};

pub async fn add_reviewers(user: &BitbucketUser, review: &Review, access_token: &str) {
    let url = prepare_get_prinfo_url(review.repo_owner(), review.repo_name(), review.id());
    let pr_info = get_pr_info(&url, access_token).await;
    let reviewers_opt = get_updated_reviewers_vec(pr_info, user).await;
    if reviewers_opt.is_none() {
        eprintln!("[add_reviewers] Unable to add reviewers for review: {}", review.id());
        return;
    }
    let (reviewers, pr_info_json) = reviewers_opt.expect("Empty reviewers_opt");
    let put_payload = prepare_put_body(&reviewers, &pr_info_json);
    put_reviewers(&url, access_token, &put_payload).await;
}

async fn get_updated_reviewers_vec(pr_opt: Option<Response>, user_from_db: &BitbucketUser) -> Option<(Vec<BitbucketUser>, Value)> {
    let reviewers_opt = parse_reviewers_from_prinfo(pr_opt).await;
    if reviewers_opt.is_none() {
        eprintln!("Unable to parse and add reviewers");
        return None;
    }
    let (mut reviewers, get_response_json) = reviewers_opt.expect("Empty reviewers_opt");
    println!("reviewers = {:?}", reviewers);
    println!("user_from_db = {:?}", &user_from_db);
    if !reviewers.contains(user_from_db) {
        // Add the user to reviewers
        reviewers.push(user_from_db.clone());
    }
    println!("Updated reviewers = {:?}", reviewers);
    return Some((reviewers, get_response_json));
}

fn prepare_put_body(updated_reviewers: &Vec<BitbucketUser>, pr_info_json: &Value) -> Option<Value> {
    // Serialize and add updated reviewers to response json
    let reviewers_obj_res = serde_json::to_value(updated_reviewers);
    if reviewers_obj_res.is_err() {
        let e = reviewers_obj_res.expect_err("No error in reviewers_obj_res");
        eprintln!("Unable to serialize users: {:?}", e);
        return None;
    }
    let reviewers_obj = reviewers_obj_res.expect("Uncaught error in reviewers_obj_res");
    let mut response_json = pr_info_json.to_owned();
    let obj_opt = response_json.as_object_mut();
    if obj_opt.is_none() {
        eprintln!("Unable to get mutable reviewer response obj");
        return None;
    }
    // Update obj
    let obj = obj_opt.expect("empty obj_opt");
    obj.insert("reviewers".to_string(), reviewers_obj);
    obj.remove("summary"); // API gives error if not removed
    return Some(response_json);
}

async fn parse_reviewers_from_prinfo(pr_opt: Option<Response>) -> Option<(Vec<BitbucketUser>, Value)> {
    if pr_opt.is_none() {
        eprintln!("Empty get response for pr_info");
        return None;
    }
    let pr_info_response = pr_opt.expect("Error in getting response");
    println!("get API status: {}", pr_info_response.status());
    let pr_info_json = pr_info_response.json::<Value>().await;
    if pr_info_json.is_err() {
        let e = pr_info_json.expect_err("No error in pr_info_json");
        eprintln!("Unable to deserialize response_json: {:?}", e);
        return None;
    }
    let pr_info = pr_info_json.expect("Uncaught error in pr_info_json");
    let reviewers_opt = pr_info.get("reviewers");
    if reviewers_opt.is_none() {
        eprintln!("No reviewers found in response: {:?}", &pr_info);
        return None;
    }
    let reviewers_value = reviewers_opt.expect("Empty reviewers_opt").to_owned();
    let reviewers_res = serde_json::from_value(reviewers_value);
    if reviewers_res.is_err() {
        let e = reviewers_res.expect_err("No error in reviewers_res");
        eprintln!("Failed to serialize reviewers: {:?}", e);
        return None;
    }
    let reviewers: Vec<BitbucketUser> = reviewers_res.expect("Uncaught error in response_res");
    return Some((reviewers, pr_info));
}

async fn put_reviewers(url: &str, access_token: &str, put_body_opt: &Option<Value>) {
    if put_body_opt.is_none() {
        eprintln!("Empty put request body, not adding reviewers");
        return;
    }
    let put_body = put_body_opt.to_owned().expect("Empty put_body_opt");
    // Make the PUT API call
    let client = get_client();
    let response_res = client
        .put(url)
        .bearer_auth(&access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&put_body)
        .send().await;

    // Handle the response_res as necessary
    println!("response_res = {:?}", &response_res);
    // for debugging
    match response_res {
        Ok(v) => println!("response v = {:?}", v.text().await),
        Err(e) => println!("response err = {:?}", e),
    };
}

async fn get_pr_info(url: &str, access_token: &str) -> Option<Response> {
    let client = get_client();
    let headers_opt = prepare_headers(&access_token);
    if headers_opt.is_none() {
        eprintln!("Unable to prepare_headers, empty headers_opt");
        return None;
    }
    let headers = headers_opt.expect("Empty headers_opt");
    let get_res = client.get(url).headers(headers).send().await;
    if get_res.is_err() {
        let e = get_res.expect_err("No error in response_res");
        eprintln!("Error in get request for adding reviewer - {:?}", e);
        return None;
    }
    let get_response = get_res.expect("Uncaught error in get_res");
    return Some(get_response);
}

fn prepare_get_prinfo_url(repo_owner: &str, repo_name: &str, review_id: &str) -> String {
    let url = format!(
        "{}/repositories/{}/{}/pullrequests/{}",
        "https://api.bitbucket.org/2.0".to_string(),
        repo_owner,
        repo_name,
        review_id
    );
    println!("add reviews url = {}", &url);
    return url;
}
