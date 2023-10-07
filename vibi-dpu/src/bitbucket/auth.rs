use std::env;
use std::str;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;
use crate::db::auth::{save_auth_info_to_db, auth_info};
use crate::utils::auth::AuthInfo;

pub async fn get_access_token_from_bitbucket(code: &str) -> Option<AuthInfo> {
    let client = Client::new();
    let bitbucket_client_id = env::var("BITBUCKET_CLIENT_ID").unwrap();
    let bitbucket_client_secret = env::var("BITBUCKET_CLIENT_SECRET").unwrap();
    let mut params = std::collections::HashMap::new();
    let redirect_uri = format!("{}/api/bitbucket/callbacks/install",
        env::var("SERVER_URL").expect("SERVER_URL must be set"));
    params.insert("client_id", bitbucket_client_id);
    params.insert("client_secret", bitbucket_client_secret);
    params.insert("code", code.to_owned());
    params.insert("grant_type", "authorization_code".to_owned());
    params.insert("redirect_uri", redirect_uri);
    println!("params = {:?}", &params);
    let response = client
        .post("https://bitbucket.org/site/oauth2/access_token")
        .form(&params)
        .send()
        .await;
    if response.is_err() {
        let response_err = response.expect_err("No error in access token response");
        eprintln!("error in calling api : {:?}", &response_err);
        return None;
    }
    let res = response.expect("Uncaught error in response");
    if !res.status().is_success() {
        eprintln!(
            "Failed to exchange code for access token. Status code: {}, Response content: {}",
            res.status(),
            res.text().await.expect("No text in response")
        );
        return None;
    }
    let json_res = res.json::<AuthInfo>().await;
    if json_res.is_err() {
        let json_error = json_res.expect_err("Error not found in json");
        eprintln!("error deserializing : {:?}", json_error);
        return None;
    }
    let mut response_json = json_res.expect("Uncaught error in deserializing response json");
    save_auth_info_to_db(&mut response_json);
    return Some(response_json);
}