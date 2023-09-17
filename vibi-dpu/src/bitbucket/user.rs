use chrono::{DateTime, Utc, FixedOffset};
use crate::db::auth::auth_info;
use crate::db::user::{save_user_to_db, user_from_db};
use crate::utils::auth::AuthInfo;
use crate::utils::lineitem::LineItem;
use crate::utils::user::{User, Provider, ProviderEnum};
use super::config::{bitbucket_base_url, get_api_values, get_api};

pub async fn get_and_save_workspace_users(workspace_id: &str, access_token: &str) {
    let base_url = bitbucket_base_url();
    let members_url = format!("{}/workspaces/{}/members", &base_url, workspace_id);
    let response_json = get_api_values(&members_url, access_token, None).await;
    for user_json in response_json {
        let provider_id = user_json["user"]["uuid"].to_string().replace('"', "");
        let user = User::new(
            Provider::new(
                provider_id,
                ProviderEnum::Bitbucket),
        user_json["user"]["display_name"].to_string().replace('"', ""),
        user_json["workspace"]["slug"].to_string().replace('"', ""),
        None);
        save_user_to_db(&user);
    }
}

pub async fn get_commit_bb(commit: &str, repo_name: &str, repo_owner: &str) -> Option<LineItem>{
    let base_url = bitbucket_base_url();
    let commits_url = format!("{}/repositories/{}/{}/commit/{}", &base_url, repo_owner, repo_name, commit);
    println!("commits url = {}", &commits_url);
    let authinfo: AuthInfo =  auth_info();
    let access_token = authinfo.access_token();
    let response_opt = get_api(&commits_url, access_token, &None).await;
    if response_opt.is_none() {
        return None;
    }
    let response = response_opt.expect("Empty response_opt from commit url");
    let parse_res = response.json::<serde_json::Value>().await;//.expect("Error in deserializing json");
    if parse_res.is_err() {
        let e = parse_res.expect_err("No error in parse_res");
        eprintln!("Error in deserializing json: {:?}", e);
        return None;
    }
    let response_json = parse_res.expect("Uncaught error in parse_res");
    let timestamp_str = &response_json["date"].to_string().replace('"', "");
    println!("timestamp_str = {}", timestamp_str);
    // Explicitly specify the format
    let datetime_res = DateTime::parse_from_rfc3339(&timestamp_str);
    if datetime_res.is_err() {
        let e = datetime_res.expect_err("No error in dateime_res");
        eprintln!("Failed to parse timestamp: {:?}", e);
        return None;
    }
    let datetime: DateTime<FixedOffset> = datetime_res.expect("Uncaught error in datetime_res");
    // Convert to Utc
    let datetime_utc = datetime.with_timezone(&Utc);

    let unix_timestamp = datetime_utc.timestamp();
    let unix_timestamp_str = unix_timestamp.to_string();
    let author_id = response_json["author"]["user"]["uuid"].to_string().replace('"', "");
    let author_name = response_json["author"]["user"]["display_name"].to_string().replace('"', "");
    let user_opt = user_from_db(
        &ProviderEnum::Bitbucket.to_string(), 
        repo_owner, &author_id);
    if user_opt.is_none() {
        let user = User::new(
            Provider::new(author_id.clone(), 
            ProviderEnum::Bitbucket), 
            author_name, repo_owner.to_string(), None);
        save_user_to_db(&user);
    }
    return Some(LineItem::new(author_id, unix_timestamp_str));
}