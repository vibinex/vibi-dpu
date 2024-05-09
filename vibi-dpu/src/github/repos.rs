use std::borrow::Borrow;

use serde_json::Value;
use reqwest::Client;
use serde_json::json;

use super::config::{get_api_paginated, github_base_url};
use crate::bitbucket::repo;
use crate::{core::utils::user_selected_repos, utils::reqwest_client::get_client};
use crate::utils::repo::Repository;
use crate::db::repo::save_repo_to_db;
use crate::utils::user::ProviderEnum;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphQLResponse {
	data: GraphQLData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphQLData {
	viewer: GraphQLViewer,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphQLViewer {
	repositories: GraphQLRepositories,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphQLRepositories {
	nodes: Vec<Repository>,
	page_info: PageInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Owner {
	login: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PageInfo {
	has_next_page: bool,
	end_cursor: Option<String>,
}


pub async fn get_github_app_installed_repos(access_token: &str) -> Option<Vec<Repository>> {
	let repos_url = format!("{}/installation/repositories", github_base_url());
	let repos_opt = get_api_paginated(&repos_url, access_token, None).await;
	if repos_opt.is_none() {
		log::error!("[get_github_app_installed_repos] Unable to call get api and get all repos");
		return None;
	}
	let repos_val = repos_opt.expect("Empty repos_opt");
	let repositories = deserialize_repos(repos_val);
	log::debug!("[get_github_app_installed_repos] Fetched {:?} repositories from GitHub", &repositories);
	return Some(repositories)
}

pub async fn get_user_accessed_github_repos(access_token: &str) -> Option<Vec<Repository>> {
	let repos_url = format!("{}/user/repos", github_base_url());
	let repos_opt = get_api_paginated(&repos_url, access_token, None).await;
	if repos_opt.is_none() {
		log::error!("[get_user_accessed_github_repos] Unable to call get api and get all repos");
		return None;
	}
	let repos_val = repos_opt.expect("Empty repos_opt");
	let all_repos = deserialise_github_pat_repos(repos_val);
	// filter repositories vec after calling vibinex-server api
	// call vibinex-server api and get selected repo list
	let selected_repositories_opt = user_selected_repos(&ProviderEnum::Github.to_string()).await;
	if selected_repositories_opt.is_none() {
		log::error!("[get_user_accessed_github_repos] Unable to get repos from server");
		return None;
	}
	let selected_repositories = selected_repositories_opt.expect("EMpty selected_repositories_opt");
	// Set union of all and user selected repos
	let pat_repos: Vec<Repository> = all_repos.into_iter().filter(|repo| {
		selected_repositories.iter().any(|selected_repo| {
			repo.name() == &selected_repo.repo_name &&
			repo.provider() == &selected_repo.repo_provider &&
			repo.owner() == &selected_repo.repo_owner
		})
	})
	.collect();
	log::debug!("[get_user_accessed_github_repos] Fetched {:?} repositories from GitHub", &pat_repos);
	return Some(pat_repos)
}

fn conditional_end_cursor (end_cursor: &str) -> String {
    if end_cursor == "null" {
       return "null".to_string();
    } else {
        return format!("\"{}\"", end_cursor);
    }
}

pub async fn get_user_github_repos_using_graphql_api(
	access_token: &str,
) -> Option<Vec<Repository>> {
	let client = get_client();
	let pageCount = "100";
    let mut has_next_page = true;
    let mut end_cursor = "null".to_string();
    
    let mut all_repositories = Vec::<Repository>::new();
    
    while has_next_page {
        log::debug!("[get_user_github_repos_using_graphql_api] has_next_page while starting: {}", has_next_page);
        log::debug!("[get_user_github_repos_using_graphql_api] end_cursor while starting: {}", end_cursor);

        let query = format!("query {{ viewer {{ repositories(first: {pageCount}, after: {:?}, affiliations: [OWNER, ORGANIZATION_MEMBER, COLLABORATOR], ownerAffiliations: [OWNER, ORGANIZATION_MEMBER, COLLABORATOR]) {{ totalCount nodes {{ name id isPrivate sshUrl owner {{ login }} }} pageInfo {{ hasNextPage endCursor }} }} }} }}",
        conditional_end_cursor(&end_cursor));
        let body = json!({
            "query": query
        });
    
        log::debug!("[get_user_github_repos_using_graphql_api] Executing GraphQL query: {:?}", body);
        let graphql_request = client
            .post("https://api.github.com/graphql")
            .header("Authorization", format!("Bearer {access_token}")) 
            .header("Content-Type", "application/json")
            .header("User-Agent", "vibi-dpu")
            .json(&body).build().unwrap();
        log::debug!("[get_user_github_repos_using_graphql_api] Request Headers: {:?}", graphql_request.headers());
        log::debug!("[get_user_github_repos_using_graphql_api] Request URL: {:?}", graphql_request.url());
        let response = client.execute(graphql_request)
            .await
            .expect("Failed to execute request");
    
        let status = response.status();
        log::debug!("[get_user_github_repos_using_graphql_api] status = {status}");
        let text_body = response.text().await.expect("Failed to get response body");
        log::debug!("[get_user_github_repos_using_graphql_api] Response Body: {:?}", text_body);
        let resp_val_res = serde_json::from_str(&text_body);
        if resp_val_res.is_err() {
        let err = resp_val_res.expect_err("No error in resp_val_res"); 
        log::error!("[get_user_github_repos_using_graphql_api] error in parsing json: {:?}", &err);
        return None;
        }
        let resp_val: Value = resp_val_res.expect("Uncaught error in resp_val_res");
        let nodes_vec_res = serde_json::from_value(
            resp_val["data"]["viewer"]["repositories"]["nodes"].clone());
            if nodes_vec_res.is_err(){
                let err = nodes_vec_res.expect_err("Empty error in nodes_vec_res");
                log::error!("[get_user_github_repos_using_graphql_api] unable to parse nodes from json: {:?}", &err);
                return None;
            }
            let nodes_vec: Vec<Value> = nodes_vec_res.expect("Uncaught error in nodes_)vec_res"); 
            let repos = deserialize_repos_object_graphql(&nodes_vec);
            all_repositories.extend(repos);
            log::debug!("[get_user_github_repos_using_graphql_api] Fetched {:?} repositories from GitHub", all_repositories);

            end_cursor = resp_val["data"]["viewer"]["repositories"]["pageInfo"]["endCursor"].to_string().trim_matches('"').to_string();
            log::debug!("[get_user_github_repos_using_graphql_api] end_cursor = {}", end_cursor);
            let has_next_page_opt = resp_val["data"]["viewer"]["repositories"]["pageInfo"]["hasNextPage"].as_bool().clone();
            log::debug!("[get_user_github_repos_using_graphql_api] has_next_page_opt = {:?}", has_next_page_opt);
            if has_next_page_opt.is_none(){
                log::error!("[get_user_github_repos_using_graphql_api] unable to parse hasNextPage from json");
                break;
            }
            has_next_page = has_next_page_opt.expect("Unable to parse hasNextPage from JSON");
            log::debug!("[get_user_github_repos_using_graphql_api] has_next_page = {}", has_next_page);
    }
        log::debug!("[get_user_github_repos_using_graphql_api] repos = {:?}", &all_repositories);
        return Some(all_repositories);
}

fn deserialize_repos_object_graphql(repos_json: &Vec<Value>) -> Vec<Repository> {
	let mut repos = Vec::<Repository>::new();
	for repo_json in repos_json {
		let is_private_res = repo_json["isPrivate"].as_bool();
		let mut is_private = true;
		if is_private_res.is_some() {
			is_private = is_private_res.expect("Uncaught error in is_private_res");
		}
		let repo = Repository::new(
			repo_json["name"].to_string().trim_matches('"').to_string(),
			repo_json["id"].to_string().trim_matches('"').to_string(),
			repo_json["owner"]["login"].to_string().trim_matches('"').to_string(),
			is_private,
			repo_json["sshUrl"].to_string().trim_matches('"').to_string(),
			None,    
			repo_json["owner"]["login"].to_string().trim_matches('"').to_string(),
			None,
			"github".to_string(),
		);
		repos.push(repo);
	}
	return repos;
}



fn deserialize_repos(repos_val: Vec<Value>) -> Vec<Repository> {
	let mut all_repos = Vec::new();
	for response_json in repos_val {
		let repo_json_opt = response_json["repositories"].as_array();
		if repo_json_opt.is_none() {
			log::error!("[deserialize_repos] Unable to deserialize repo value: {:?}", &response_json);
			continue;
		}
		let repos_page_json = repo_json_opt.expect("Empty repo_json_opt").to_owned();
		for repo_json in repos_page_json {
			let repo = deserialize_repo_object(&repo_json);
			save_repo_to_db(&repo);
			all_repos.push(repo);
		}
	}
	return all_repos;
}

fn deserialise_github_pat_repos(repos_val: Vec<Value>) -> Vec<Repository> {
	let mut all_repos = Vec::new();
	for response_json in repos_val {
		let repo_json_opt = response_json.as_array();
		if repo_json_opt.is_none() {
			log::error!("[deserialize_repos] Unable to deserialize repo value: {:?}", &response_json);
			continue;
		}
		let repos_page_json = repo_json_opt.expect("Empty repo_json_opt").to_owned();
		for repo_json in repos_page_json {
			let repo = deserialize_repo_object(&repo_json);
			save_repo_to_db(&repo);
			all_repos.push(repo);
		}
	}
	return all_repos;
}

fn deserialize_repo_object(repo_json: &Value) -> Repository {
	let is_private_res = repo_json["private"].as_bool();
	let mut is_private = true;
	if is_private_res.is_some() {
		is_private = is_private_res.expect("Uncaught error in is_private_res");
	}
	let repo = Repository::new(
		repo_json["name"].to_string().trim_matches('"').to_string(),
		repo_json["id"].to_string().trim_matches('"').to_string(),
		repo_json["owner"]["login"].to_string().trim_matches('"').to_string(),
		is_private,
		repo_json["ssh_url"].to_string().trim_matches('"').to_string(),
		None,    
		repo_json["owner"]["login"].to_string().trim_matches('"').to_string(),
		None,
		"github".to_string(),
	);
	return repo;
}