use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use chrono::DateTime;
use serde::Deserialize;
use serde::Serialize;

use crate::utils::reqwest_client::get_client;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct HealthStatus {
    status: String,
    timestamp: String,
    topic: String,
}

pub async fn send_status_start() {
    send_status("START").await;
}

pub async fn send_status_failed() {
    send_status("FAILED").await;
}

pub async fn send_status_success() {
    send_status("SUCCESS").await;
}

async fn send_status(status: &str) {
    let topic_id = env::var("INSTALL_ID")
		.expect("INSTALL_ID must be set");
    let base_url = env::var("SERVER_URL")
		.expect("SERVER_URL must be set");
    let now = SystemTime::now();
    let now_ts = now.duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_secs();
    let datetime_opt = DateTime::from_timestamp(now_ts as i64, 0);
    if datetime_opt.is_none() {
        return;
    }
    let datetime = datetime_opt.expect("Empty datetime");
    let formatted_timestamp = datetime.to_rfc3339();
    log::debug!("[send_status] ==========<><><><><>===== now_ts = {:?}", &formatted_timestamp);
    let client = get_client();
	let status_url = format!("{base_url}/api/dpu/health");
    let body = HealthStatus {
        status: status.to_string(),
        timestamp: formatted_timestamp,
        topic: topic_id
    };
    let post_res = client
	  .post(&status_url)
	  .json(&body)
	  .send()
	  .await;
    log::debug!("[send_status] post_res = {:?}", &post_res);
	if post_res.is_err() {
		let e = post_res.expect_err(
            "No error in post_res in send_status");
		log::error!(
            "[send_status] error in send_status post_res: {:?}, url: {:?}",
            e, &status_url);
		return;
	}
}