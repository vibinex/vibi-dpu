use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use chrono::{Utc, Duration};
use std::fs;
use crate::client::config::get_client;

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

fn generate_jwt(github_app_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pem_file_path = "/tmp/repoprofiler_private.pem";
    let pem_data = fs::read(pem_file_path)?;

    let my_claims = Claims {
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + Duration::minutes(5)).timestamp(),
        iss: github_app_id.to_string(),
    };

    let encoding_key = EncodingKey::from_rsa_pem(&pem_data)?;
    let token = encode(&Header::new(Algorithm::RS256), &my_claims, &encoding_key)?;

    Ok(token)
}

pub async fn fetch_access_token(installation_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let github_app_id = env::var("GITHUB_APP_ID")?;
    let jwt_token = generate_jwt(&github_app_id)?;

    let client = get_client();
    let response: Value = client.post(&format!("https://api.github.com/app/installations/{}/access_tokens", installation_id))
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", jwt_token))
        .send()
        .await?
        .json()
        .await?;
    println!("[fetch_access_token] response = {:?}", response);
    Ok(response)
}
