use serde_json::Value;

use crate::db::repo::save_repo_to_db;
use crate::utils::repo::Repository;
use super::config::{bitbucket_base_url, get_api};

pub async fn get_workspace_repos(workspace: &str, access_token: &str) -> Option<Vec<Repository>> {
    let repos_url = format!("{}/repositories/{}", bitbucket_base_url(), workspace);
    let response_json = get_api(&repos_url, access_token, None).await;
    let mut repos_data = Vec::new();
    for repo_json in response_json {
        let val = Repository::new(
            repo_json["name"].to_string().trim_matches('"').to_string(),
            repo_json["uuid"].to_string().trim_matches('"').to_string(),
            repo_json["owner"]["username"].to_string().trim_matches('"').to_string(),
            repo_json["is_private"].as_bool().unwrap_or(false),
            repo_json["links"]["clone"].as_array()
                .expect("Unable to convert clone to array").iter().filter(|clone_val| {
                clone_val["name".to_string()].as_str() == Some("ssh")
            }).collect::<Vec<&Value>>()[0]["href"].to_string().replace('\"',""),
            repo_json["project"]["name"].to_string().trim_matches('"').to_string(),
            repo_json["workspace"]["slug"].to_string().trim_matches('"').to_string(),
            None,
            "bitbucket".to_string(),
        );
        save_repo_to_db(&val);
        repos_data.push(val);
    }
    Some(repos_data)
}