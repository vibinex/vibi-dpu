use std::env;
mod pubsub;
mod db;
mod core;
mod bitbucket;
mod github;
mod utils;
mod logger;
use github::auth::gh_access_token;
use tokio::task;
use crate::{core::github::setup::process_repos, utils::user::ProviderEnum};

#[tokio::main]
async fn main() {
	// Get topic subscription and Listen to messages 
	let gcp_credentials = //"/home/tapishr/dev-profiler/pubsub-sa.json".to_owned();
	env::var("GCP_CREDENTIALS").expect("GCP_CREDENTIALS must be set");
	let topic_name = //"rtapish-fromserver".to_owned();
	env::var("INSTALL_ID").expect("INSTALL_ID must be set");

	let logs_init_status = logger::init::init_logger();
	if !logs_init_status {
		log::warn!("[main] Unable to create file logger");
	}
	log::info!("Setting up your Vibinex Data Processing Unit, sit back and relax...");
	let github_pat_res = env::var("GITHUB_PAT");
	let provider_res = env::var("PROVIDER");
	let mut is_pat = false;
	if github_pat_res.is_err() {
		log::debug!("[main] GITHUB PAT env var must be set");
	} else {
		let github_pat = github_pat_res.expect("Empty GITHUB_PAT env var");
		log::debug!("[main] GITHUB PAT: [{}]", &github_pat);

		if provider_res.is_err() {
			log::debug!("[main] PROVIDER env var must be set");
		} else {
			let provider = provider_res.expect("Empty PROVIDER env var");
			log::debug!("[main] PROVIDER: {}", provider);

			if provider.eq_ignore_ascii_case("GITHUB") {
				is_pat = true;
					core::github::setup::setup_self_host_user_repos_github(&github_pat).await;
			}
		}
	}
	if !is_pat {
		load_auth_from_previous_installation().await;
	}
	log::debug!("[main] env vars = {}, {}", &gcp_credentials, &topic_name);
	pubsub::listener::listen_messages(
		&gcp_credentials, 
		&topic_name,
	).await;
}

async fn load_auth_from_previous_installation() {
	if let Some(access_token) = gh_access_token(&None).await {
		log::info!("[load_auth_from_previous_installation] Loaded auth from file, processing repos..");
		process_repos(&access_token, &ProviderEnum::Github.to_string()).await;
	}
}