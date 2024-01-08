use std::env;
use std::collections::HashMap;

use reqwest::{Response, Error};
use serde_json::{json, Value};

use crate::{db::webhook::save_webhook_to_db, utils::github_webhook::Webhook, github::config::{github_base_url, get_api_paginated}};
use crate::utils::reqwest_client::get_client;
use super::config::prepare_headers;


pub async fn get_webhooks_in_repo(repo_owner: &str, repo_name: &str, access_token: &str) -> Option<Vec<Webhook>> {
    let url = format!("{}/repos/{}/{}/hooks", github_base_url(), repo_owner, repo_name);
    log::info!("[get_webhooks_in_repo] Getting webhooks from {}", url);
    let response_opt = get_api_paginated(&url, access_token, None).await;
    if response_opt.is_none() {
        log::error!("[get_webhooks_in_repo] Unable to call get api and get all webhooks");
        return None;
    }
    let webhook_val = response_opt.expect("Empty repos_opt");
    let webhooks = deserialize_webhooks(webhook_val);
    log::info!("[get_webhooks_in_repo] Fetched {:?} repositories from GitHub", &webhooks);
    return Some(webhooks);
}

fn deserialize_webhooks(webhook_val: Vec<Value>) -> Vec<Webhook> {
    let mut all_webhooks = Vec::new();
    for response_json in webhook_val {
        let webhook_json_opt = response_json.as_array();
        if webhook_json_opt.is_none() {
            log::error!("[deserialize_webhooks] Unable to deserialize webhook value: {:?}", &response_json);
            continue;
        }
        let webhook_page_json = webhook_json_opt.expect("Empty repo_json_opt").to_owned();
        for webhook_json in webhook_page_json {
            let webhook = deserialize_webhook_object(&webhook_json);
            save_webhook_to_db(&webhook);
            all_webhooks.push(webhook);
        }
    }
    return all_webhooks;
}

fn deserialize_webhook_object(webhook_json: &Value) -> Webhook {
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
    return webhook;
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
        log::error!("[process_add_webhook_response] Error in api call: {:?}", err);
        return;
    }
    let res = response.expect("Uncaught error in response");
    if !res.status().is_success() {
        log::error!("[process_add_webhook_response] Failed to add webhook. Status code: {}, Text: {:?}",
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