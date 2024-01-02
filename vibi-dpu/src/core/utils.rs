use std::env;
use std::str;
use serde::{Deserialize, Serialize};

use crate::utils::reqwest_client::get_client;
use crate::utils::setup_info::SetupInfo;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PublishRequest {
    installationId: String,
    info: Vec<SetupInfo>,
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
