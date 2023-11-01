use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use std::env;
use std::str;
use std::process::Command;
use chrono::{Utc, Duration};
use std::fs;
use crate::db::github::auth::github_auth_info;
use crate::{utils::reqwest_client::get_client, utils::github_auth_info::GithubAuthInfo, db::github::auth::save_github_auth_info_to_db};
use crate::utils::review::Review;

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
        let parse_res = response_access_token.json::<GithubAuthInfo>().await ;
        if parse_res.is_err() {
            let e = parse_res.expect_err("No error in parse_res for AuthInfo");
            eprintln!("error deserializing GithubAuthInfo: {:?}", e);
            return None;
        }
        let response_json = parse_res.expect("Uncaught error in parse_res for AuthInfo");
        return Some(response_json);
}

pub async fn update_access_token(auth_info: &GithubAuthInfo, clone_url: &str, directory: &str) -> Option<GithubAuthInfo> {
	let app_installation_id = auth_info.installation_id(); 
	let now = SystemTime::now();
    let now_secs = now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
    let timestamp_opt = auth_info.timestamp();
    if timestamp_opt.is_none() {
        eprintln!("No timestamp in GithubAuthInfo");
        return None;
    }
    let timestamp = timestamp_opt.expect("Empty timestamp");
    let expires_at = auth_info.expires_at();
    if expires_at > now_secs {  
        eprintln!("Not yet expired, expires_at = {}, now_secs = {}", expires_at, now_secs);
        return Some(auth_info.to_owned());
    }
    // auth info has expired
    println!("github auth info expired, expires_at = {}, now_secs = {}", expires_at, now_secs);
    let new_auth_info_opt = fetch_access_token(app_installation_id.as_str()).await;
    let mut new_auth_info = new_auth_info_opt.clone()
        .expect("empty auhtinfo_opt from update_access_token");
    println!("New github auth info  = {:?}", &new_auth_info);
    let access_token = new_auth_info.access_token().to_string();
    set_git_remote_url(clone_url, directory, &access_token);
    save_github_auth_info_to_db(&mut new_auth_info);
    return new_auth_info_opt;

}

fn set_git_remote_url(git_url: &str, directory: &str, access_token: &str) {
    let clone_url = git_url.to_string()
        .replace("git@", format!("https://x-access-token:{access_token}@").as_str())
        .replace("github.com:", "github.com/");
    let output = Command::new("git")
		.arg("remote").arg("set-url").arg("origin")
		.arg(clone_url)
		.current_dir(directory)
		.output()
		.expect("failed to execute git pull");
    // Only for debug purposes
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("set_git_url stderr = {:?}", v),
		Err(e) => eprintln!("set_git_url stderr error: {}", e), 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("set_git_urll stdout = {:?}", v),
		Err(e) => eprintln!("set_git_url stdout error: {}", e), 
	};
	println!("git pull output = {:?}, {:?}", &output.stdout, &output.stderr);
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
    let access_token = latest_authinfo.access_token().to_string();
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