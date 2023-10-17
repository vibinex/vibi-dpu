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
use crate::utils::prInfo::PrInfo;

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

pub async fn get_pr_info(workspace_slug: &str, repo_slug: &str, access_token: &str, pr_number: &str) -> Result<PrInfo, Box<dyn std::error::Error>> {
    let url = format!("{}/repositories/{}/{}/pullrequests/{}", env::var("SERVER_URL").expect("SERVER_URL must be set"), workspace_slug, repo_slug, pr_number);

    let client = get_client();
    let response = client.get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        let pr_data: serde_json::Value = response.json().await?;

        // create a PrInfo object from it
        let pr_info = PrInfo {
            base_head_commit: pr_data["destination"]["commit"]["hash"].as_str().unwrap().to_string(),
            pr_head_commit: pr_data["source"]["commit"]["hash"].as_str().unwrap().to_string(),
            state: pr_data["state"].as_str().unwrap().to_string(),
            pr_branch: pr_data["source"]["branch"]["name"].as_str().unwrap().to_string(),
        };

        Ok(pr_info)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other, 
            format!("Failed to get PR info, status: {}", response.status()),
        )))
    }
}

