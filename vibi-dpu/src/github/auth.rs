use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use serde::{Deserialize, Serialize};
use std::env;
use chrono::{Utc, Duration};
use std::fs;
use crate::{client::config::get_client, utils::auth::AuthInfo, };

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    token: String,
    // Add other fields if necessary
}

#[derive(Debug, Serialize)]
struct Claims {
    iat: i64,
    exp: i64,
    iss: String,
}

fn generate_jwt(github_app_id: &str) -> Option<String> {
    let pem_file_path = "/app/repoprofiler_private.pem";
    let pem_data = match fs::read(pem_file_path) {
        Ok(data) => data,
        Err(e) => {
            println!("Error reading pem file: {}", e);
            return None;
        },
    };
    let my_claims = Claims {
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + Duration::minutes(5)).timestamp(),
        iss: github_app_id.to_string(),
    };

    let encoding_key = match EncodingKey::from_rsa_pem(&pem_data) {
        Ok(key) => key,
        Err(e) => {
            println!("Error creating encoding key: {}", e);
            return None;
        },
    };
    match encode(&Header::new(Algorithm::RS256), &my_claims, &encoding_key) {
        Ok(token) => Some(token),
        Err(e) => {
            println!("Error encoding JWT: {}", e);
            None
        },
    }
}

pub async fn fetch_access_token(installation_id: &str) -> Option<String> {
    let github_app_id = env::var("GITHUB_APP_ID").unwrap();
    let jwt_token = generate_jwt(&github_app_id).unwrap();

    let client = get_client();
    let response = client.post(&format!("https://api.github.com/app/installations/{}/access_tokens", installation_id))
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("User-Agent", "Vibinex code review Test App")
        .send()
        .await;
        if response.is_err() {
            let e = response.expect_err("No error in response");
            eprintln!("error in calling github api : {:?}", e);
            return None;
        }
        let res = response.expect("Uncaught error in reponse");
        if !res.status().is_success() {
            println!(
                "Failed to exchange code for access token. Status code: {}, Response content: {:?}",
                res.status(),
                res.text().await
            );
            return None;
        }
        let parse_res = res.text().await ;
        if parse_res.is_err() {
            let e = parse_res.expect_err("No error in parse_res for AuthInfo");
            eprintln!("error deserializing AuthInfo: {:?}", e);
            return None;
        }
        let response_json = parse_res.expect("Uncaught error in parse_res for AuthInfo");
        return Some(response_json);
}