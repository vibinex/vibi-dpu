use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use std::str;
use std::env;
use std::process::Command;
use std::time::SystemTime;
use crate::client::config::get_client;
use crate::utils::auth::AuthInfo;
use crate::utils::review::Review;

pub async fn list_prs_bitbucket(repo_owner: &str, repo_name: &str, access_token: &str, state: &str) -> Vec<u32> {
    let mut pr_list = Vec::new();
    let client = get_client();

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", access_token));
    
    let mut params = HashMap::new();
    params.insert("state".to_string(), state.to_string());

    let response = client
        .get(&format!("{}/repositories/{}/{}/pullrequests", env::var("SERVER_URL").expect("SERVER_URL must be set"), repo_owner, repo_name))
        .headers(headers)
        .query(&params)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() != StatusCode::OK {
                eprintln!("Request failed with status: {}", resp.status());
                return pr_list;
            }
            let data: Value = match resp.json().await {
                Ok(json) => json,
                Err(err) => {
                    eprintln!("Failed to parse JSON: {:?}", err);
                    return pr_list;
                },
            };

            if let Value::Array(pull_requests) = data["values"].clone() {
                for pr in pull_requests.iter() {
                    if let Some(id) = pr["id"].as_u64() {
                        pr_list.push(id as u32);
                    }
                }
            }
        },
        Err(error) => {
            eprintln!("Failed to make a request: {:?}", error);
        }
    }
    pr_list
}
