use std::env;
mod pubsub;
mod db;
mod core;
mod bitbucket;
mod github;
mod utils;
mod logger;
use tokio::task;

#[tokio::main]
async fn main() {
    // Get topic subscription and Listen to messages 
    let gcp_credentials = //"/home/tapishr/dev-profiler/pubsub-sa.json".to_owned();
    env::var("GCP_CREDENTIALS").expect("GCP_CREDENTIALS must be set");
    let topic_name = //"rtapish-fromserver".to_owned();
    env::var("INSTALL_ID").expect("INSTALL_ID must be set");
    
    let logs_init_status = logger::init::init_logger();
    if !logs_init_status {
        log::error!("[main] Unable to file logger");
    }

    let pat_env_var = "GITHUB_PAT";
    let provider_env_var = "PROVIDER";
    if let (Ok(pat), Ok(provider)) = (env::var(pat_env_var), env::var(provider_env_var)) {
        log::info!("[main] Personal Access Token: [REDACTED]");
        log::info!("[main] Provider: {}", provider);
        if provider == "GITHUB" {
            task::spawn(async move {
                core::github::setup::setup_self_host_user_repos_github(&pat).await;
                log::info!("[main] Github repos self host setup processed");
            });    
        }
    }
    log::info!("[main] env vars = {}, {}", &gcp_credentials, &topic_name);
    pubsub::listener::listen_messages(
        &gcp_credentials, 
        &topic_name,
    ).await;
}