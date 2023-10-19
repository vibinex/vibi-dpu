use std::env;
use std::str;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;
use super::config::get_client;
use crate::db::auth::{save_auth_info_to_db, auth_info};
use crate::utils::auth::AuthInfo;
use crate::utils::review::Review;

pub async fn get_access_token_from_bitbucket(code: &str) -> Option<AuthInfo> {
    let client = get_client();
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
    let post_res = client
        .post("https://bitbucket.org/site/oauth2/access_token")
        .form(&params)
        .send()
        .await;
    if post_res.is_err() {
        let e = post_res.expect_err("No error in post_res");
        eprintln!("error in calling api : {:?}", e);
        return None;
    }
    let res = post_res.expect("Uncaught error in post_res");
    if !res.status().is_success() {
        println!(
            "Failed to exchange code for access token. Status code: {}, Response content: {:?}",
            res.status(),
            res.text().await
        );
        return None;
    }
    let parse_res = res.json::<AuthInfo>().await ;
    if parse_res.is_err() {
        let e = parse_res.expect_err("No error in parse_res for AuthInfo");
        eprintln!("error deserializing AuthInfo: {:?}", e);
        return None;
    }
    let mut response_json = parse_res.expect("Uncaught error in parse_res for AuthInfo");
    save_auth_info_to_db(&mut response_json);
    return Some(response_json);
}

pub async fn refresh_git_auth(clone_url: &str, directory: &str) -> Option<String>{
	let authinfo_opt =  auth_info();
    if authinfo_opt.is_none() {
        return None;
    }
    let authinfo = authinfo_opt.expect("empty authinfo_opt in refresh_git_auth");
    let authinfo_opt = update_access_token(&authinfo, clone_url, directory).await;
    if authinfo_opt.is_none() {
        eprintln!("Empty authinfo_opt from update_access_token");
        return None;
    }
    let latest_authinfo = authinfo_opt.expect("Empty authinfo_opt");
    let access_token = latest_authinfo.access_token().to_string();
    return Some(access_token);
}

pub async fn update_access_token(auth_info: &AuthInfo,
        clone_url: &str, directory: &str) -> Option<AuthInfo> {
    let now = SystemTime::now();
    let now_secs = now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
    let timestamp_opt = auth_info.timestamp();
    if timestamp_opt.is_none() {
        eprintln!("No timestamp in authinfo");
        return None;
    }
    let timestamp = timestamp_opt.expect("Empty timestamp");
    let expires_at = timestamp + auth_info.expires_in();
    if expires_at > now_secs {  
        eprintln!("Not yet expired, expires_at = {}, now_secs = {}", expires_at, now_secs);
        return Some(auth_info.to_owned());
    }
    // auth info has expired
    println!("auth info expired, expires_at = {}, now_secs = {}", expires_at, now_secs);
    let new_auth_info_opt = bitbucket_refresh_token(auth_info.refresh_token()).await;
    let mut new_auth_info = new_auth_info_opt.clone()
        .expect("empty auhtinfo_opt from update_access_token");
    println!("New auth info  = {:?}", &new_auth_info);
    let access_token = new_auth_info.access_token().to_string();
    set_git_remote_url(clone_url, directory, &access_token);
    save_auth_info_to_db(&mut new_auth_info);
    return new_auth_info_opt;
}

async fn bitbucket_refresh_token(refresh_token: &str) -> Option<AuthInfo> {
    let token_url = "https://bitbucket.org/site/oauth2/access_token";
    let client_id = std::env::var("BITBUCKET_CLIENT_ID")
        .expect("BITBUCKET_CLIENT_ID must be set");
    let client_secret = std::env::var("BITBUCKET_CLIENT_SECRET")
        .expect("BITBUCKET_CLIENT_SECRET must be set");
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::CONTENT_TYPE, 
        "application/x-www-form-urlencoded".parse().expect("Invalid content-type"));
    let payload = &[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token)
    ];
    let client = get_client();
    let post_res = client.post(token_url)
        .headers(headers)
        .basic_auth(client_id, Some(client_secret))
        .form(payload)
        .send()
        .await;
    if post_res.is_err() {
        let e = post_res.expect_err("No error in post_err for refres token");
        eprintln!("Unable to get refresh token: {}", e);
        return None;
    }
    let response = post_res.expect("Uncaught error in post_res");
    if !response.status().is_success() {
        eprintln!("Failed to get refresh token, status: {} body: {:?}", 
            response.status(), response.text().await);
        return None;
    }
    let parse_res =  response.json().await;
    if parse_res.is_err() {
        let e = parse_res.expect_err("No error in parse_res refresh_token");
        eprintln!("Unable to deserialize refresh token response: {}", e);
        return None;
    }
    let refresh_token_resbody = parse_res.expect("Uncaught error in parse_res");
    return Some(refresh_token_resbody);
}

fn set_git_remote_url(git_url: &str, directory: &str, access_token: &str) {
    let clone_url = git_url.to_string()
        .replace("git@", format!("https://x-token-auth:{{{access_token}}}@").as_str())
        .replace("bitbucket.org:", "bitbucket.org/");
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

pub async fn get_access_token_review(review: &Review) -> Option<String> {
    let authinfo_opt = auth_info();
    if authinfo_opt.is_none() {
        return None;
    }
    let auth_info = authinfo_opt.expect("Uncaught error in authinfo_opt");
    let mut access_token = auth_info.access_token().clone();
    let clone_url = review.clone_url();
    let directory = review.clone_dir();
    let new_auth_opt = update_access_token(&auth_info, &clone_url, &directory).await;
    if new_auth_opt.is_none() {
        return None;
    }
    let new_auth = new_auth_opt.expect("empty new_auth_opt");
    access_token = new_auth.access_token().to_string();
    return Some(access_token);
}