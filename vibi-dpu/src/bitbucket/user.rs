use crate::db::bitbucket::auth::bitbucket_auth_info;
use crate::db::user::{add_bitbucket_user_to_workspace_user, get_workspace_user_from_db};
use crate::utils::user::BitbucketUser;
use super::config::{bitbucket_base_url, get_api_values, get_api_response};

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

pub async fn author_from_commit(commit: &str, repo_name: &str, repo_owner: &str) -> Option<BitbucketUser>{
    let base_url = bitbucket_base_url();
    let commits_url = format!("{}/repositories/{}/{}/commit/{}", &base_url, repo_owner, repo_name, commit);
    println!("commits url = {}", &commits_url);
    let authinfo_opt =  bitbucket_auth_info();
    if authinfo_opt.is_none() {
        return None;
    }
    let authinfo = authinfo_opt.expect("empty authinfo_opt in get_commit_bb");
    let access_token = authinfo.access_token();
    let response_opt = get_api_response(&commits_url, None, access_token, &None).await;
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
    let author_val = response_json["author"]["user"].to_owned();
    let author_res = serde_json::from_value(author_val);
    if author_res.is_err() {
        let err = author_res.expect_err("Empty error in author_res");
        eprintln!("[author_from_commit] Unable to deserialize author: {:?}", err);
        return None;
    }
    let author: BitbucketUser = author_res.expect("Uncaught error in author_res");
    return Some(author);
}