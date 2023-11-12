use chrono::DateTime;
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use std::env;
use std::str;
use chrono::{Utc, Duration};
use std::fs;
use crate::db::github::auth::github_auth_info;
use crate::{utils::reqwest_client::get_client, utils::github_auth_info::GithubAuthInfo, db::github::auth::save_github_auth_info_to_db};
use crate::utils::review::Review;
use crate::utils::gitops::set_git_remote_url;

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
    let pem_file_path = "/app/repo-profiler.pem";
    let pem_data_res = fs::read(pem_file_path);
    
    if pem_data_res.is_err() {
        let pem_data_err = pem_data_res.expect_err("No error in reading pem file");
        println!("Error reading pem file: {:?}", pem_data_err);
        return None;
    }
    let pem_data = pem_data_res.expect("Error reading pem file");

    let my_claims = Claims {
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + Duration::minutes(5)).timestamp(),
        iss: github_app_id.to_string(),
    };

    let encoding_key = EncodingKey::from_rsa_pem(&pem_data);
    if encoding_key.is_err() {
        println!("Error creating encoding key");
        return None;
    }

    let token_res = encode(&Header::new(Algorithm::RS256), &my_claims, &encoding_key.unwrap());
    if token_res.is_err() {
        let token_err = token_res.expect_err("No error in fetching token");
        println!("Error encoding JWT: {:?}", token_err);
        return None;
    };
    let token = token_res.expect("Error encoding JWT");
    Some(token)
}

pub async fn fetch_access_token(installation_id: &str) -> Option<GithubAuthInfo> {
    let github_app_id = env::var("GITHUB_APP_ID");
    let github_app_id_str = github_app_id.expect("GITHUB_APP_ID must be set");
    let jwt_token = generate_jwt(&github_app_id_str).expect("Error generating JWT");

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
        let response_access_token = response.expect("Uncaught error in reponse");
        if !response_access_token.status().is_success() {
            println!(
                "Failed to exchange code for access token. Status code: {}, Response content: {:?}",
                response_access_token.status(),
                response_access_token.text().await
            );
            return None;
        }
        println!("[fetch_access_token] response access token = {:?}", &response_access_token);
        let parse_res = response_access_token.json::<GithubAuthInfo>().await ;
        if parse_res.is_err() {
            let e = parse_res.expect_err("No error in parse_res for AuthInfo");
            eprintln!("error deserializing GithubAuthInfo: {:?}", e);
            return None;
        }
        let mut response_json = parse_res.expect("Uncaught error in parse_res for AuthInfo");
        save_github_auth_info_to_db(&mut response_json);
        return Some(response_json);
}

pub async fn update_access_token(auth_info: &GithubAuthInfo, clone_url: &str, directory: &str) -> Option<GithubAuthInfo> {
    let repo_provider = "github".to_string();
	let app_installation_id = auth_info.installation_id(); 
    let now_ts = Utc::now().timestamp();
    let expires_at = auth_info.expires_at();
    let expires_at_dt_res = DateTime::parse_from_rfc3339(expires_at);
    if expires_at_dt_res.is_err() {
        let e = expires_at_dt_res.expect_err("No error in expires_at_dt_res");
        eprintln!("[update_access_token] Unable to parse expires_at to datetime: {:?}", e);
        return None;
    }
    let expires_at_dt = expires_at_dt_res.expect("Uncaught error in expires_at_dt_res");
    let expires_at_ts = expires_at_dt.timestamp();
    if expires_at_ts > now_ts {  
        eprintln!("Not yet expired, expires_at = {}, now_secs = {}", expires_at, now_ts);
        return Some(auth_info.to_owned());
    }
    // auth info has expired
    println!("github auth info expired, expires_at = {}, now_secs = {}", expires_at, now_ts);
    let new_auth_info_opt = fetch_access_token(app_installation_id.as_str()).await;
    let mut new_auth_info = new_auth_info_opt.clone()
        .expect("empty auhtinfo_opt from update_access_token");
    println!("New github auth info  = {:?}", &new_auth_info);
    let access_token = new_auth_info.token().to_string();
    set_git_remote_url(clone_url, directory, &access_token, &repo_provider);
    save_github_auth_info_to_db(&mut new_auth_info);
    return new_auth_info_opt;

}

pub async fn refresh_git_auth(clone_url: &str, directory: &str) -> Option<String>{
	let authinfo_opt =  github_auth_info();
    if authinfo_opt.is_none() {
        return None;
    }
    let authinfo = authinfo_opt.expect("empty authinfo_opt in refresh_git_auth");
    let authinfo_opt = update_access_token(&authinfo, clone_url, directory).await;
    if authinfo_opt.is_none() {
        eprintln!("Empty authinfo_opt from update_access_token for github auth info");
        return None;
    }
    let latest_authinfo = authinfo_opt.expect("Empty authinfo_opt");
    let access_token = latest_authinfo.token().to_string();
    return Some(access_token);
}

pub async fn get_access_token_review(review: &Review) -> Option<String> {
	let clone_url = review.clone_url();
	let directory = review.clone_dir();
	
	let access_token_opt = refresh_git_auth(clone_url, directory).await;
	if access_token_opt.is_none(){
		return None;
	}

	let access_token = access_token_opt.expect("empty access token option");
	return Some(access_token);
}