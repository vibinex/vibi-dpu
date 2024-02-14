use std::collections::HashMap;
use std::env;
use std::str;
use serde::{Deserialize, Serialize};

use crate::db::aliases::update_handles_in_db;
use crate::utils::repo::Repository;
use crate::utils::reqwest_client::get_client;
use crate::utils::review::Review;
use crate::utils::setup_info::SetupInfo;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PublishRequest {
    installationId: String,
    info: Vec<SetupInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AliasRequest {
    repo_name: String,
    repo_owner: String,
    repo_provider: String,
    aliases: Vec<String>
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct AliasResponse {
    aliases: HashMap<String, Vec<String>>,
}

pub async fn send_setup_info(setup_info: &Vec<SetupInfo>) {
    let installation_id = env::var("INSTALL_ID")
        .expect("INSTALL_ID must be set");
    log::debug!("[send_setup_info] install_id = {:?}", &installation_id);
    let base_url = env::var("SERVER_URL")
        .expect("SERVER_URL must be set");
    let body = PublishRequest {
        installationId: installation_id,
        info: setup_info.to_vec(),
    };
    log::debug!("[send_setup_info] body = {:?}", &body);
    let client = get_client();
    let setup_url = format!("{base_url}/api/dpu/setup");
    let post_res = client
      .post(&setup_url)
      .json(&body)
      .send()
      .await;
    if post_res.is_err() {
        let e = post_res.expect_err("No error in post_res in send_setup_info");
        log::error!("[send_setup_info] error in send_setup_info post_res: {:?}, url: {:?}", e, &setup_url);
        return;
    }
    let resp = post_res.expect("Uncaught error in post_res");
    log::debug!("[send_setup_info] Response: {:?}", resp.text().await);
}

pub async fn send_aliases(repo: &Repository, aliases: &Vec<String>) {
    let base_url = env::var("SERVER_URL")
        .expect("SERVER_URL must be set");
    let body = AliasRequest {
        repo_name: repo.name().to_owned(),
        repo_owner: repo.owner().to_owned(),
        repo_provider: repo.provider().to_owned(),
        aliases: aliases.to_owned()
    };
    log::debug!("[send_aliases] body = {:?}", &body);
    let client = get_client();
    let alias_url = format!("{base_url}/api/dpu/aliases");
    let post_res = client
      .post(&alias_url)
      .json(&body)
      .send()
      .await;
    if post_res.is_err() {
        let e = post_res.expect_err("No error in post_res in send_aliases");
        log::error!("[send_aliases] error in send_aliases post_res: {:?}, url: {:?}", e, &alias_url);
        return;
    }
    let resp = post_res.expect("Uncaught error in post_res");
    log::debug!("[send_aliases] Response: {:?}", resp.text().await);
}

pub async fn get_aliases(review: &Review) -> Option<HashMap<String, Vec<String>>>{
    let base_url = env::var("SERVER_URL")
        .expect("SERVER_URL must be set");
    let client = get_client();
    let alias_url = format!("{base_url}/api/dpu/aliases?repo_name={}&repo_owner={}&repo_provider={}",
                            review.repo_name(),
                            review.repo_owner(),
                            review.provider());
    let get_res = client
        .get(&alias_url)
        .send()
        .await;

    if let Err(e) = get_res {
        log::error!("[get_aliases] error in get_res: {:?}, url: {:?}", e, &alias_url);
        return None;
    }

    let resp = get_res.expect("Uncaught error in get_res");
    let body_text = resp.text().await.expect("Unable to read response body");
    let alias_response: AliasResponse = serde_json::from_str(&body_text)
        .expect("Failed to deserialize JSON response");
    let alias_map = alias_response.aliases.to_owned();
    for (alias, handles) in alias_map {
        update_handles_in_db(&alias, &review.provider(), handles.to_owned());
    }
    Some(alias_response.aliases)
}