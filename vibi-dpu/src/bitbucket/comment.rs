use std::{env, collections::HashMap};

use reqwest::{Response, header::{HeaderMap, HeaderValue}};
use serde::Serialize;
use serde_json::Value;

use crate::db::auth::auth_info;
use crate::db::user::get_workspace_user_from_db;
use crate::utils::review::Review;
use crate::utils::user::BitbucketUser;

use super::{config::{bitbucket_base_url, get_client}};

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
    deserialize_response(&url, response).await;
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
fn prepare_headers(access_token: &str) -> Option<HeaderMap> {
    let mut headers = reqwest::header::HeaderMap::new(); 
    let auth_header_res = format!("Bearer {}", access_token).parse();
    if auth_header_res.is_err() {
        let e = auth_header_res
            .expect_err("Empty error in auth_header_res");
        eprintln!("Invalid auth header: {:?}", e);
        return None;
    }
    let header_authval = auth_header_res
        .expect("Uncaught error in auth_header_res");
    headers.insert( reqwest::header::AUTHORIZATION, header_authval);
    let header_accept_res =  "application/json".parse();
    if header_accept_res.is_err() {
        let e = header_accept_res
            .expect_err("No error in header_accept_res");
        eprintln!("Invalide accept header val, error: {:?}", e);
        return None;
    }
    let header_acceptval = header_accept_res
        .expect("Uncaught error in header_accept_res");
    headers.insert("Accept", header_acceptval);
    return Some(headers);
}

async fn deserialize_response(url: &str, response: Response) {
    println!("API {}, status: {}", url, response.status());
    let response_json = response.json::<Value>().await;
    println!("url = {}, response from api = {:?}", url, &response_json);
}

async fn deserialize_response_reviewer(url: &str, response: Response, user_key: &str, access_token: &str, review_id: &str) {
    println!("API {}, status: {}", url, response.status());
    let response_json = response.json::<Value>().await;
    println!("user_key = {:?}", user_key);
    let response = response_json.expect("Unable to deserialize response_json");
    let user_from_db_opt = get_workspace_user_from_db(&user_key);
    if user_from_db_opt.is_none() {
        eprintln!("empty user_from_db_opt");
        return;
    }
    let user_from_db = user_from_db_opt.expect("empty user_from_db_opt");
    println!("user_from_db = {:?}", &user_from_db);
    let reviewers_opt = response.get("reviewers");
    if reviewers_opt.is_none() {
        eprintln!("No reviewers found in response: {:?}", response);
        return;
    }
    let reviewers_value = reviewers_opt.expect("Empty reviewers_opt").to_owned();
    let mut reviewers: Vec<BitbucketUser> = serde_json::from_value(reviewers_value).expect("Failed to serialize reviewers");
    println!("reviewers = {:?}", reviewers);
    // For each user in user_from_db.users()...
    for user in user_from_db.users().iter() {
        // If the reviewers vector doesn't contain the user...
        if !reviewers.contains(user) {
            // Add the user to reviewers
            reviewers.push(user.clone());
        }
    }
    put_reviewers(response, reviewers, access_token, url, review_id).await;
}

async fn put_reviewers(mut response_data: Value, reviewers: Vec<BitbucketUser>, access_token: &str, url: &str, review_id: &str) {
    println!("reviewers = {:?}", &reviewers);
    let reviewers_obj: Value = serde_json::to_value(reviewers).expect("Unable to serialize users");
    if let Some(obj) = response_data.as_object_mut() {
        obj.insert("reviewers".to_string(), reviewers_obj);
        obj.remove("summary");
        // Note: You don't need to insert "type" and "id" as they should already be part of response_data.
    }
    // // Navigate to the summary and remove the "html" key.
    // if let Some(response_mut) = response_data.as_mut().and_then(Value::as_object_mut) {
    //     response_mut.remove("summary");
    // }
    // Serialize the updated response to JSON
    let payload = serde_json::to_string(&response_data).expect("Failed to serialize to string");
    println!("put reviewers payload = {:?}", &payload);
    // Make the PUT API call
    let client = get_client();
    let response_res = client
        .put(url)
        .bearer_auth(&access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(payload)
        .send().await;

    // Handle the response_res as necessary
    println!("response_res = {:?}", &response_res);
    // for debugging
    match response_res {
        Ok(v) => println!("response v = {:?}", v.text().await),
        Err(e) => println!("response err = {:?}", e)
    };
}

pub async fn add_reviewers(user_key: &str, review: &Review, access_token: &str) {
    let url = prepare_add_reviewers_url(review.repo_owner(), review.repo_name(), review.id());
    let client = get_client();
    let headers_opt = prepare_headers(&access_token);
    if headers_opt.is_none() {
        eprintln!("Unable to prepare_headers, empty headers_opt");
        return;
    }
    let headers = headers_opt.expect("Empty headers_opt");
    let get_res = client.get(&url).headers(headers).send().await;
    if get_res.is_err() {
        let e = get_res.expect_err("No error in response_res");
        eprintln!("Error in get request for adding reviewer - {:?}", e);
        return;
    }
    let get_response = get_res.expect("Error in getting response");
    println!("get API {}, status: {}", url, get_response.status());
    deserialize_response_reviewer(&url, get_response, user_key, &access_token, review.id()).await;
    
    // let response_res = client
    //     .put(&url)
    //     .bearer_auth(&access_token) // Use Bearer authentication with your personal access token
    //     .header("Accept", "application/json")
    //     .send().await;
    // if response_res.is_err() {
    //     let e = response_res.expect_err("No error in response_res");
    //     eprintln!("Error in put request for auto assign - {:?}", e);
    //     return;
    // }
    // let response = response_res.expect("Error in getting response");
    // deserialize_response(&url, response).await;
}

fn prepare_add_reviewers_url(repo_owner: &str, 
    repo_name: &str, review_id: &str) -> String {
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