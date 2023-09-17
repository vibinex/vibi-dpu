use std::env;

use reqwest::{header::HeaderValue, Response, Error};
use serde_json::json;

use crate::{db::webhook::save_webhook_to_db, utils::webhook::{Webhook, WebhookResponse}, bitbucket::config::{bitbucket_base_url, get_api}};

use super::config::{get_client, prepare_auth_headers};


pub async fn get_webhooks_in_repo(workspace_slug: &str, repo_slug: &str, access_token: &str) -> Vec<Webhook> {
    let url = format!("{}/repositories/{}/{}/hooks", bitbucket_base_url(), workspace_slug, repo_slug);
    println!("Getting webhooks from {}", url);
    let response_json = get_api(&url, access_token, None).await;
    let mut webhooks = Vec::new();
    for webhook_json in response_json {
        let active = matches!(webhook_json["active"].to_string().trim_matches('"'), "true" | "false");
        let webhook = Webhook::new(
            webhook_json["uuid"].to_string(),
            active,
            webhook_json["created_at"].to_string().replace('"', ""),
            webhook_json["events"].as_array().expect("Unable to deserialize events").into_iter()
                .map(|events| events.as_str().expect("Unable to convert event").to_string()).collect(),
            webhook_json["links"]["self"]["href"].to_string().replace('"', ""),
            webhook_json["url"].to_string().replace('"', ""),
        );
        webhooks.push(webhook);
    }
    return webhooks;
}

pub async fn add_webhook(workspace_slug: &str, repo_slug: &str, access_token: &str) {
    let url = format!(
        "{}/repositories/{}/{}/hooks", 
        bitbucket_base_url(), workspace_slug, repo_slug
    );

    let mut headers_map = prepare_auth_headers(&access_token);
    headers_map.insert("Accept", HeaderValue::from_static("application/vnd.github+json"));
    let callback_url = format!("{}/api/bitbucket/callbacks/webhook", 
        env::var("SERVER_URL").expect("SERVER_URL must be set"));
    let payload = json!({
        "description": "Webhook for PRs when raised and when something is pushed to the open PRs",
        "url": callback_url,
        "active": true,
        "events": ["pullrequest:created", "pullrequest:updated"] 
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
        println!("Failed to add webhook. Status code: {}, Text: {:?}",
            res.status(), res.text().await);
        return;
    }
    let webhook_res = res.json::<WebhookResponse>().await;
    if webhook_res.is_err() {
        let err = webhook_res.expect_err("No error in webhook response");
        return;
    }
    
    let webhook = webhook_res.expect("Uncaught error in webhook response");
    let webhook_data = Webhook::new(
        webhook.uuid().to_string(),
        webhook.active(),
        webhook.created_at().to_string(),
        webhook.events().to_owned(),
        webhook.links()["self"]["href"].clone(),
        webhook.url().to_string(),
    );
    save_webhook_to_db(&webhook_data); 
}