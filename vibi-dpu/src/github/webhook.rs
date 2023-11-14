use std::env;
use std::collections::HashMap;

use reqwest::{Response, Error};
use serde_json::{json, Value};

use crate::{db::webhook::save_webhook_to_db, utils::github_webhook::Webhook, github::config::{github_base_url, get_webhook_api_values}};
use crate::utils::reqwest_client::get_client;
use super::config::prepare_headers;


pub async fn get_webhooks_in_repo(repo_owner: &str, repo_name: &str, access_token: &str) -> Vec<Webhook> {
    let url = format!("{}/repos/{}/{}/hooks", github_base_url(), repo_owner, repo_name);
    println!("Getting webhooks from {}", url);
    let response_json = get_webhook_api_values(&url, access_token, None).await;
    let mut webhooks = Vec::new();
    for webhook_json in response_json {
        let active = matches!(webhook_json["active"].to_string().trim_matches('"'), "true" | "false");
        let webhook = Webhook::new(
            webhook_json["id"].to_string(),
            active,
            webhook_json["created_at"].to_string().replace('"', ""),
            webhook_json["events"].as_array().expect("Unable to deserialize events").into_iter()
                .map(|events| events.as_str().expect("Unable to convert event").to_string()).collect(),
            webhook_json["ping_url"].to_string().replace('"', ""),
            webhook_json["config"]["url"].to_string().replace('"', ""),
            webhook_json.get("config")
                .and_then(Value::as_object)
                .map(|config_obj| {
                    config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<String, Value>>()
                }).expect("Config should be a JSON object")
        );
        webhooks.push(webhook);
    }
    return webhooks;
}

pub async fn add_webhook(repo_owner: &str, repo_name: &str, access_token: &str) {
    let url = format!("{}/repos/{}/{}/hooks", github_base_url(), repo_owner, repo_name);

    let headers_map_opt = prepare_headers(&access_token);
    if headers_map_opt.is_none() {
        return;
    }
    let headers_map = headers_map_opt.expect("Empty headers_map_opt");
    let callback_url = format!("{}/api/github/callbacks/webhook", 
        env::var("SERVER_URL").expect("SERVER_URL must be set"));
    let payload = json!({
        "name": "web", 
        "events": ["pull_request", "pull_request_review"],
        "config": { "url": callback_url, "content_type":"json", "insecure_ssl":"0"},
        "active": true,
    });
    let response = get_client()
        .post(&url)
        .headers(headers_map)
        .json(&payload)
        .send()
        .await;
    process_add_webhook_response(response).await;
}

async fn process_add_webhook_response(response: Result<Response, Error>){
    if response.is_err() {
        let err = response.expect_err("No error in response");
        eprintln!("Error in api call: {:?}", err);
        return;
    }
    let res = response.expect("Uncaught error in response");
    if !res.status().is_success() {
        eprintln!("Failed to add webhook. Status code: {}, Text: {:?}",
            res.status(), res.text().await);
        return;
    }
    let webhook_json = res.json::<Value>().await.expect("[process_add_webhook_response] Unable to deserialize res to Value");
    let webhook = Webhook::new(
        webhook_json["id"].to_string(),
        webhook_json["active"].as_bool().expect("Unable to deserialize active"),
        webhook_json["created_at"].to_string().replace('"', ""),
        webhook_json["events"].as_array().expect("Unable to deserialize events").into_iter()
            .map(|events| events.as_str().expect("Unable to convert event").to_string()).collect(),
        webhook_json["ping_url"].to_string().replace('"', ""),
        webhook_json["config"]["url"].to_string().replace('"', ""),
        webhook_json.get("config")
            .and_then(Value::as_object)
            .map(|config_obj| {
                config_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<String, Value>>()
            }).expect("Config should be a JSON object")
    );
    save_webhook_to_db(&webhook); 
}