use std::collections::HashMap;

use futures_util::StreamExt;
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_default::WithAuthExt;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    subscription::{SubscriptionConfig, Subscription},
};
use serde::Deserialize;
use tokio::task;
use std::collections::VecDeque;
use sha256::digest;
use tonic::Code;
use crate::core::setup::handle_install_bitbucket;
// use crate::core::{setup::handle_install_bitbucket, review::process_review}; // To be added in future PR

#[derive(Debug, Deserialize)]
struct InstallCallback {
    repository_provider: String,
    installation_code: String,
}


async fn process_message(attributes: &HashMap<String, String>, data_bytes: &Vec<u8>) {
    let msgtype_opt = attributes.get("msgtype");
    if msgtype_opt.is_none() {
        eprintln!("msgtype attribute not found in message : {:?}", attributes);
        return;
    }
    let msgtype = msgtype_opt.expect("Empty msgtype");
    match msgtype.as_str() {
        "install_callback" => {
            println!("Processing install callback message");
            let msg_data_res =  serde_json::from_slice::<InstallCallback>(data_bytes);
            if msg_data_res.is_err() {
                eprintln!("Error deserializing install callback: {:?}", msg_data_res);
                return;
            }
            let data = msg_data_res.expect("msg_data not found");
            let code_async = data.installation_code.clone();
            task::spawn(async move {
                handle_install_bitbucket(&code_async).await;
                println!("Processed install callback message");
            });
        },
        "webhook_callback" => {
            task::spawn(async move {
                // process_review(&data_bytes).await;
                println!("Processed webhook callback message");
            });
        }
        _ => {
            eprintln!("Message type not found for message : {:?}", attributes);
        }
    };
}

pub async fn get_pubsub_client_config(keypath: &str) -> ClientConfig {
    let credfile = CredentialsFile::new_from_file(keypath.to_string()).await
        .expect("Failed to locate credentials file");
    return ClientConfig::default()
        .with_credentials(credfile)
        .await
        .expect("Unable to get PubSub Client config");
}

async fn setup_subscription(keypath: &str, topicname: &str) -> Subscription{
    let config = get_pubsub_client_config(keypath).await;
    let client = Client::new(config).await
        .expect("Unable to create pubsub client to listen to messages");
    let topic = client.topic(topicname);
    let topic_res = topic.exists(None).await;
    if topic_res.is_err() {
        let e = topic_res.expect_err("No error found in topic_res");
        if e.code() == Code::NotFound {
            client.create_topic(topicname, None, None).await
                .expect("Unable to create topic");
        }
        else {
            eprintln!("Error getting topic: {:?}", e);
        }
    }
    let sub_config = SubscriptionConfig {
        enable_message_ordering: true,
        ..Default::default()
    };
    let subscription_name = format!("{topicname}-sub");
    let subscription = client.subscription(&subscription_name);
    let subconfig = SubscriptionConfig {
        enable_message_ordering: true,
        ..Default::default()
    };
    if !subscription.exists(None).await.expect("Unable to get subscription information") {
        subscription.create(
            topic.fully_qualified_name(), subconfig, None)
            .await.expect("Unable to create subscription for listening to messages");
    }
    println!("sub = {:?}", &subscription);
    subscription
}

pub async fn listen_messages(keypath: &str, topicname: &str) {
    let queue_cap = 100;
    let mut message_hashes = VecDeque::with_capacity(queue_cap);
    let subscription = setup_subscription(keypath, topicname).await;
    let mut stream = subscription.subscribe(None).await
        .expect("Unable to subscribe to messages");
    while let Some(message) = stream.next().await {
        println!("Listening for messages...");
        let attrmap: HashMap<String, String> = message.message.attributes.clone().into_iter().collect();
        let message_hash = digest(&*message.message.data);
        if !message_hashes.contains(&message_hash) {
            message_hashes.push_back(message_hash);
            if message_hashes.len() > queue_cap {
                while message_hashes.len() > queue_cap {
                    message_hashes.pop_front();
                }
            }
            let msg_bytes = message.message.data.clone();
            process_message(&attrmap, &msg_bytes).await;
        }
        // Ack or Nack message.
        let _ = message.ack().await;
    }
}