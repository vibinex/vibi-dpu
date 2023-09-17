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
    match response {
        Ok(res) => {
            if !res.status().is_success() {
                println!(
                    "Failed to exchange code for access token. Status code: {}, Response content: {}",
                    res.status(),
                    res.text().await.expect("No text in response")
                );
                return None;
            }
            match res.json::<AuthInfo>().await { Ok(mut response_json) => {
                save_auth_info_to_db(&mut response_json);
                return Some(response_json);
            }, Err(e) => {
                println!("error deserializing : {:?}", e);
                return None;} };
        },
        Err(e) => {
            println!("error in calling api : {:?}", e);
            return None},
    }
}

pub async fn refresh_git_auth(clone_url: &str, directory: &str) -> String{
	let authinfo: AuthInfo =  auth_info();
    let mut access_token = authinfo.access_token().to_string();
    match update_access_token(&authinfo).await {
        Some(mut new_auth_info) => {
            println!("New auth info  = {:?}", &new_auth_info);
            access_token = new_auth_info.access_token().to_string();
            set_git_remote_url(clone_url, directory, &access_token);
            save_auth_info_to_db(&mut new_auth_info);
        },
        None => {println!(" No new auth info");},
    }
    return access_token;
}

pub async fn update_access_token(auth_info: &AuthInfo) -> Option<AuthInfo> {

    let now = SystemTime::now();
    let now_secs = now.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();

    if let Some(timestamp) = auth_info.timestamp() {
        let expires_at = timestamp + auth_info.expires_in();
        println!(" expires_at = {expires_at}, now_secs = {now_secs}");
        if expires_at <= now_secs {  
            // auth info has expired
            match bitbucket_refresh_token(auth_info.refresh_token()).await {
                Some(new_auth_info) => return Some(new_auth_info),
                None => {},
            };
        }
    }
    return None;
}

async fn bitbucket_refresh_token(
    refresh_token: &str, 
) -> Option<AuthInfo> {

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

    let client = Client::new();
    match client.post(token_url)
        .headers(headers)
        .basic_auth(client_id, Some(client_secret))
        .form(payload)
        .send()
        .await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json().await {
                        Ok(resbody) => return Some(resbody),
                        Err(e) => eprintln!("Unable to deserialize response: {}", e),
                    }
                }
                else {
                    eprintln!("Failed to get refresh token, status: {}", response.status());
                }
            },
            Err(e) => eprintln!("Unable to get refresh token: {}", e),
        };
        return None;
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
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("git pull stderr = {:?}", v),
		Err(e) => {/* error handling */ println!("{}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("git pull stdout = {:?}", v),
		Err(e) => {/* error handling */ println!("{}", e)}, 
	};
	println!("git pull output = {:?}, {:?}", &output.stdout, &output.stderr);
}