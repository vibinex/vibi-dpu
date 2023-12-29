use std::env;
mod pubsub;
mod db;
mod core;
mod bitbucket;
mod github;
mod utils;
mod logger;

#[tokio::main]
async fn main() {
    // Get topic subscription and Listen to messages 
    let gcp_credentials = //"/home/tapishr/dev-profiler/pubsub-sa.json".to_owned();
    env::var("GCP_CREDENTIALS").expect("GCP_CREDENTIALS must be set");
    let topic_name = //"rtapish-fromserver".to_owned();
    env::var("INSTALL_ID").expect("INSTALL_ID must be set");
    log::error!("[main] env vars = {}, {}", &gcp_credentials, &topic_name);

    let logs_init_status = logger::init::init_logger();
    if !logs_init_status {
        log::error!("[main] Unable to file logger");
    }

    pubsub::listener::listen_messages(
        &gcp_credentials, 
        &topic_name,
    ).await;
}