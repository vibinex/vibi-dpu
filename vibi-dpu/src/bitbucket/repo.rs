use serde_json::Value;

use crate::db::repo::save_repo_to_db;
use crate::utils::repo::Repository;
use super::config::{bitbucket_base_url, get_api_values};

pub async fn get_workspace_repos(workspace: &str, access_token: &str) -> Option<Vec<Repository>> {
    let repos_url = format!("{}/repositories/{}", bitbucket_base_url(), workspace);
    let response_json = get_api_values(&repos_url, access_token).await;
    let mut repos_data = Vec::new();
    for repo_json in response_json {
        let is_private_res = repo_json["is_private"].as_bool();
        let mut is_private = true;
        if is_private_res.is_none() {
            eprintln!("Error in deserializing is_private_res: {:?}", &repo_json);
        }
        is_private = is_private_res.expect("Uncaught error in is_private_res");
        let val = Repository::new(
            repo_json["name"].to_string().trim_matches('"').to_string(),
            repo_json["uuid"].to_string().trim_matches('"').to_string(),
            repo_json["owner"]["username"].to_string().trim_matches('"').to_string(),
            is_private,
            repo_json["links"]["clone"].as_array()
                .expect("Unable to convert clone to array").iter().filter(|clone_val| {
                clone_val["name".to_string()].as_str() == Some("ssh")
            }).collect::<Vec<&Value>>()[0]["href"].to_string().replace('\"',""),
            Some(repo_json["project"]["name"].to_string().trim_matches('"').to_string()),
            repo_json["workspace"]["slug"].to_string().trim_matches('"').to_string(),
            None,
            "bitbucket".to_string(),
        );
        save_repo_to_db(&val);
        repos_data.push(val);
    }
    Some(repos_data)
}