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

pub async fn author_uuid_from_commit(commit: &str, repo_name: &str, repo_owner: &str) -> Option<String>{
    let base_url = bitbucket_base_url();
    let commits_url = format!("{}/repositories/{}/{}/commit/{}", &base_url, repo_owner, repo_name, commit);
    println!("commits url = {}", &commits_url);
    let authinfo_opt =  auth_info();
    if authinfo_opt.is_none() {
        return None;
    }
    let authinfo = authinfo_opt.expect("empty authinfo_opt in get_commit_bb");
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

    let author_id = response_json["author"]["user"]["uuid"].to_string().replace('"', "");
    return Some(author_id);
}