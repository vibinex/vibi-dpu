use crate::pubsub::listener::process_message;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, time::Duration};

const DEFAULT_POLL_INTERVAL_MS: u64 = 5_000;
const DEFAULT_LEASE_SECONDS: u64 = 300;

#[derive(Debug, Deserialize)]
struct ClaimResponse {
	jobs: Vec<DpuJob>,
}

#[derive(Debug, Deserialize)]
struct DpuJob {
	id: String,
	#[serde(rename = "msgType")]
	msg_type: String,
	payload: Value,
}

#[derive(Debug, Serialize)]
struct ClaimRequest<'a> {
	#[serde(rename = "installationId")]
	installation_id: &'a str,
	#[serde(rename = "maxJobs")]
	max_jobs: u8,
	#[serde(rename = "leaseSeconds")]
	lease_seconds: u64,
}

#[derive(Debug, Serialize)]
struct AckRequest<'a> {
	#[serde(rename = "installationId")]
	installation_id: &'a str,
	#[serde(rename = "jobId")]
	job_id: &'a str,
}

#[derive(Debug, Serialize)]
struct FailRequest<'a> {
	#[serde(rename = "installationId")]
	installation_id: &'a str,
	#[serde(rename = "jobId")]
	job_id: &'a str,
	error: &'a str,
	retry: bool,
}

pub async fn poll_messages(server_url: &str, installation_id: &str, auth_token: &str) {
	let client = Client::builder()
		.connect_timeout(Duration::from_secs(10))
		.timeout(Duration::from_secs(30))
		.build()
		.expect("Failed to build HTTP client");
	let poll_interval = std::env::var("DPU_POLL_INTERVAL_MS")
		.ok()
		.and_then(|value| value.parse::<u64>().ok())
		.unwrap_or(DEFAULT_POLL_INTERVAL_MS);
	let lease_seconds = std::env::var("DPU_JOB_LEASE_SECONDS")
		.ok()
		.and_then(|value| value.parse::<u64>().ok())
		.unwrap_or(DEFAULT_LEASE_SECONDS);
	let normalized_server_url = server_url.trim_end_matches('/');
	log::info!("[http_queue] Polling DPU jobs from {}", normalized_server_url);

	loop {
		match claim_job(
			&client,
			normalized_server_url,
			installation_id,
			auth_token,
			lease_seconds,
		)
		.await
		{
			Ok(Some(job)) => {
				process_job(&client, normalized_server_url, installation_id, auth_token, job).await;
			}
			Ok(None) => {
				tokio::time::sleep(Duration::from_millis(poll_interval)).await;
			}
			Err(err) => {
				log::error!("[http_queue] Failed to claim job: {:?}", err);
				tokio::time::sleep(Duration::from_millis(poll_interval)).await;
			}
		}
	}
}

async fn claim_job(
	client: &Client,
	server_url: &str,
	installation_id: &str,
	auth_token: &str,
	lease_seconds: u64,
) -> Result<Option<DpuJob>, reqwest::Error> {
	let response = client
		.post(format!("{}/api/dpu/jobs/claim", server_url))
		.bearer_auth(auth_token)
		.json(&ClaimRequest {
			installation_id,
			max_jobs: 1,
			lease_seconds,
		})
		.send()
		.await?
		.error_for_status()?
		.json::<ClaimResponse>()
		.await?;
	Ok(response.jobs.into_iter().next())
}

async fn process_job(
	client: &Client,
	server_url: &str,
	installation_id: &str,
	auth_token: &str,
	job: DpuJob,
) {
	log::info!("[http_queue] Received job {} ({})", job.id, job.msg_type);
	let msg_bytes = match serde_json::to_vec(&job.payload) {
		Ok(bytes) => bytes,
		Err(err) => {
			log::error!("[http_queue] Could not serialize job payload for {}: {:?}", job.id, err);
			let _ = fail_job(
				client,
				server_url,
				installation_id,
				auth_token,
				&job.id,
				"invalid payload",
				false,
			)
			.await;
			return;
		}
	};
	let mut attributes = HashMap::new();
	attributes.insert("msgtype".to_string(), job.msg_type.clone());
	// NOTE: process_message internally spawns background tasks for some message types
	// (e.g. webhook_callback → process_review, install_callback → handle_install_*).
	// The ACK below happens after process_message returns, but those spawned tasks may
	// still be running. If the DPU is killed between ACK and task completion, work is
	// silently lost. Tracked as a known limitation; a proper fix requires process_message
	// to expose JoinHandles so they can be awaited before ACK.
	process_message(&attributes, &msg_bytes).await;
	if let Err(err) = ack_job(client, server_url, installation_id, auth_token, &job.id).await {
		log::error!("[http_queue] Failed to ack job {}: {:?}", job.id, err);
	}
}

async fn ack_job(
	client: &Client,
	server_url: &str,
	installation_id: &str,
	auth_token: &str,
	job_id: &str,
) -> Result<(), reqwest::Error> {
	client
		.post(format!("{}/api/dpu/jobs/ack", server_url))
		.bearer_auth(auth_token)
		.json(&AckRequest {
			installation_id,
			job_id,
		})
		.send()
		.await?
		.error_for_status()?;
	Ok(())
}

async fn fail_job(
	client: &Client,
	server_url: &str,
	installation_id: &str,
	auth_token: &str,
	job_id: &str,
	error: &str,
	retry: bool,
) -> Result<(), reqwest::Error> {
	client
		.post(format!("{}/api/dpu/jobs/fail", server_url))
		.bearer_auth(auth_token)
		.json(&FailRequest {
			installation_id,
			job_id,
			error,
			retry,
		})
		.send()
		.await?
		.error_for_status()?;
	Ok(())
}
