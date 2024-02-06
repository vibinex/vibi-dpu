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

    let github_pat_res = env::var("GITHUB_PAT");
    let provider_res = env::var("PROVIDER");
    if github_pat_res.is_err() {
        log::info!("[main] GITHUB PAT env var must be set");
    } else {
        let github_pat = github_pat_res.expect("Empty GITHUB_PAT env var");
        log::info!("[main] GITHUB PAT: [REDACTED]");

        if provider_res.is_err() {
            log::info!("[main] PROVIDER env var must be set");
        } else {
            let provider = provider_res.expect("Empty PROVIDER env var");
            log::info!("[main] PROVIDER: {}", provider);

            if provider.eq_ignore_ascii_case("GITHUB") {
                task::spawn(async move {
                    core::github::setup::setup_self_host_user_repos_github(&github_pat).await;
                    log::info!("[main] Github repos self host setup processed");
                });
            }
        }
    }
 
    log::info!("[main] env vars = {}, {}", &gcp_credentials, &topic_name);
    pubsub::listener::listen_messages(
        &gcp_credentials, 
        &topic_name,
    ).await;
}