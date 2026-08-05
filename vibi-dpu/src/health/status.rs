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
    let dpu_auth_token = match env::var("DPU_AUTH_TOKEN") {
        Ok(token) if !token.is_empty() => token,
        _ => {
            log::error!("[send_status] DPU_AUTH_TOKEN must be set to send health status");
            return;
        }
    };
    let topic_id = env::var("INSTALL_ID").expect("INSTALL_ID must be set");
    let base_url = env::var("SERVER_URL").expect("SERVER_URL must be set");
    let now = SystemTime::now();
    let now_ts = now
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let datetime_opt = DateTime::from_timestamp(now_ts as i64, 0);
    if datetime_opt.is_none() {
        return;
    }
    let datetime = datetime_opt.expect("Empty datetime");
    let formatted_timestamp = datetime.to_rfc3339();
    log::debug!(
        "[send_status] ==========<><><><><>===== now_ts = {:?}",
        &formatted_timestamp
    );
    let client = get_client();
    let status_url = format!("{base_url}/api/dpu/health");
    let body = HealthStatus {
        status: status.to_string(),
        timestamp: formatted_timestamp,
        topic: topic_id,
    };
    let post_res = post_status(&client, &status_url, &dpu_auth_token, &body).await;
    log::debug!("[send_status] post_res = {:?}", &post_res);
    if post_res.is_err() {
        let e = post_res.expect_err("No error in post_res in send_status");
        log::error!(
            "[send_status] error in send_status post_res: {:?}, url: {:?}",
            e,
            &status_url
        );
        return;
    }
}

async fn post_status(
    client: &reqwest::Client,
    status_url: &str,
    dpu_auth_token: &str,
    body: &HealthStatus,
) -> reqwest::Result<reqwest::Response> {
    client
        .post(status_url)
        .bearer_auth(dpu_auth_token)
        .json(body)
        .send()
        .await?
        .error_for_status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvRestore(Vec<(&'static str, Option<String>)>);

    impl EnvRestore {
        fn remove(keys: &[&'static str]) -> Self {
            let saved = keys
                .iter()
                .map(|key| {
                    let value = env::var(key).ok();
                    env::remove_var(key);
                    (*key, value)
                })
                .collect();
            Self(saved)
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in &self.0 {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }
    }

    #[tokio::test]
    async fn missing_auth_token_returns_before_other_required_variables_are_read() {
        let _lock = ENV_LOCK.lock().expect("environment lock poisoned");
        let _restore = EnvRestore::remove(&["DPU_AUTH_TOKEN", "INSTALL_ID", "SERVER_URL"]);

        send_status("START").await;
    }

    #[tokio::test]
    async fn rejected_health_response_is_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind test server");
        let address = listener.local_addr().expect("test server has no address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("failed to accept request");
            let mut request = [0; 4096];
            stream.read(&mut request).expect("failed to read request");
            stream
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .expect("failed to write response");
        });
        let body = HealthStatus {
            status: "START".to_string(),
            timestamp: "2026-08-05T00:00:00+00:00".to_string(),
            topic: "test-install".to_string(),
        };

        let result = post_status(
            &get_client(),
            &format!("http://{address}/api/dpu/health"),
            "test-token",
            &body,
        )
        .await;

        server.join().expect("test server panicked");
        let error = result.expect_err("401 response should be an error");
        assert_eq!(error.status(), Some(reqwest::StatusCode::UNAUTHORIZED));
    }
}
