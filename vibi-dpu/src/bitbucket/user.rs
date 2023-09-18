use chrono::{DateTime, Utc, FixedOffset};
use crate::db::auth::auth_info;
use crate::db::user::{add_bitbucket_user_to_workspace_user, get_workspace_user_from_db};
use crate::utils::auth::AuthInfo;
use crate::utils::lineitem::LineItem;
use crate::utils::user::BitbucketUser;
use super::config::{bitbucket_base_url, get_api_values, get_api};

pub async fn get_and_save_workspace_users(workspace_id: &str, access_token: &str) {
    let base_url = bitbucket_base_url();
    let members_url = format!("{}/workspaces/{}/members", &base_url, workspace_id);
    let response_json = get_api_values(&members_url, access_token, None).await;
    for user_json in response_json {
        let user_val = user_json.get("user").expect("Empty reviewers_opt");
        let user: BitbucketUser = serde_json::from_value(user_val.to_owned())
            .expect("Unable to deserialize user");
        add_bitbucket_user_to_workspace_user(user.clone());
    }
}

pub async fn get_commit_bb(commit: &str, repo_name: &str, repo_owner: &str) -> Option<LineItem>{
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
    let author_display_name = response_json["author"]["user"]["display_name"].to_string().replace('"', "");
    let user_res = response_json["author"].get("user");
    if user_res.is_none() {
        eprintln!("no user in response_json: {:?}", &response_json);
        return None;
    }
    let bitbucket_user_json = user_res.expect("empty user_res").to_owned();
    let bitbucket_user = serde_json::from_value::<BitbucketUser>(bitbucket_user_json)
        .expect("error in deserializing BitbucketUser from bitbucket_user_json");
    let mut user_key = author_display_name.clone();
    let user_opt = get_workspace_user_from_db(&user_key);
    if user_opt.is_none() {
        eprintln!("No user found in db for key: {}", &user_key);
        return None;
    }
    let user = user_opt.expect("empty user_opt");
    user_key = user.display_name().to_owned();
    return Some(LineItem::new(user_key, unix_timestamp_str));
}