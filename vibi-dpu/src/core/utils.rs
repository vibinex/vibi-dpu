use std::collections::HashMap;
use std::env;
use std::str;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bitbucket;
use crate::db::aliases::update_handles_in_db;
use crate::github;
use crate::health::status::send_status_failed;
use crate::health::status::send_status_success;
use crate::utils::repo::Repository;
use crate::utils::reqwest_client::get_client;
use crate::utils::review::Review;
use crate::utils::setup_info::SetupInfo;
use crate::utils::user::ProviderEnum;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PublishRequest {
	installationId: String,
	info: Vec<SetupInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AliasRequest {
	repo_name: String,
	repo_owner: String,
	repo_provider: String,
	aliases: Vec<String>
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct AliasResponse {
	aliases: Vec<AliasResponseHandles>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct AliasResponseHandles {
	git_alias: String,
	github: Option<Vec<String>>,
	bitbucket: Option<Vec<String>>
}

#[derive(Serialize, Deserialize, Debug)]
struct BitbucketDBRepo {
	repo_name: String,
	repo_owner: String,
	repo_provider: String,
	#[serde(default)]
	clone_ssh_url: Option<String>,
	#[serde(default)]
	is_private: Option<bool>,
	#[serde(default)]
	uuid: Option<String>,
	#[serde(default)]
	workspace: Option<String>,
	project_name: Option<String>,
	project_type: Option<String>,
}

pub async fn send_setup_info(setup_info: &Vec<SetupInfo>) {
	log::info!("Sending repo names and owners to Vibinex...");
	let installation_id = env::var("INSTALL_ID")
		.expect("INSTALL_ID must be set");
	log::debug!("[send_setup_info] install_id = {:?}", &installation_id);
	let base_url = env::var("SERVER_URL")
		.expect("SERVER_URL must be set");
	let body = PublishRequest {
		installationId: installation_id,
		info: setup_info.to_vec(),
	};
	log::debug!("[send_setup_info] body = {:?}", &body);
	let client = get_client();
	let setup_url = format!("{base_url}/api/dpu/setup");
	let post_res = client
	  .post(&setup_url)
	  .json(&body)
	  .send()
	  .await;
	if post_res.is_err() {
		let e = post_res.expect_err("No error in post_res in send_setup_info");
		log::error!("[send_setup_info] error in send_setup_info post_res: {:?}, url: {:?}", e, &setup_url);
		send_status_failed().await;
		return;
	}
	let resp = post_res.expect("Uncaught error in post_res");
	if !resp.status().is_success() {
		log::error!("[send_setup_info] Unable to send setup info to server, status = {:?}", resp.status());
		send_status_failed().await;
		return;
	}
	log::debug!("[send_setup_info] Response: {:?}", resp.text().await);
	send_status_success().await;
}

pub async fn send_aliases(repo: &Repository, aliases: &Vec<String>) {
	log::info!("Sending alias list to Vibinex...");
	let base_url = env::var("SERVER_URL")
		.expect("SERVER_URL must be set");
	let body = AliasRequest {
		repo_name: repo.name().to_owned(),
		repo_owner: repo.owner().to_owned(),
		repo_provider: repo.provider().to_owned(),
		aliases: aliases.to_owned()
	};
	log::debug!("[send_aliases] body = {:?}", &body);
	let client = get_client();
	let alias_url = format!("{base_url}/api/dpu/aliases");
	let post_res = client
	  .post(&alias_url)
	  .json(&body)
	  .send()
	  .await;
	if post_res.is_err() {
		let e = post_res.expect_err("No error in post_res in send_aliases");
		log::error!("[send_aliases] error in send_aliases post_res: {:?}, url: {:?}", e, &alias_url);
		return;
	}
	let resp = post_res.expect("Uncaught error in post_res");
	log::debug!("[send_aliases] Response: {:?}", resp.text().await);
}

pub async fn get_handles_from_server(review: &Review) -> Option<HashMap<String, Vec<String>>>{
	let base_url = env::var("SERVER_URL")
		.expect("SERVER_URL must be set");
	let client = get_client();
	let alias_url = format!("{base_url}/api/dpu/aliases?repo_name={}&repo_owner={}&repo_provider={}",
							review.repo_name(),
							review.repo_owner(),
							review.provider());
	let get_res = client
		.get(&alias_url)
		.send()
		.await;

	if let Err(e) = get_res {
		log::error!("[get_handles_from_server] error in get_res: {:?}, url: {:?}", e, &alias_url);
		return None;
	}

	let resp = get_res.expect("Uncaught error in get_res");
	let body_text = resp.text().await.expect("Unable to read response body");
	log::debug!("[get_handles_from_server] body text = {:?}", &body_text);
	let alias_response: AliasResponse = serde_json::from_str(&body_text)
		.expect("Failed to deserialize JSON response");
	let alias_handles = alias_response.aliases.to_owned();
	let mut aliases_map = HashMap::<String, Vec<String>>::new();
	for alias_handle in alias_handles {
		if review.provider().to_owned() == ProviderEnum::Github.to_string()
			&& alias_handle.github.is_some() {
				let gh_handles = alias_handle.github.expect("Empty github handles");
				update_handles_in_db(&alias_handle.git_alias, &review.provider(), gh_handles.clone());
				aliases_map.insert(alias_handle.git_alias, gh_handles);
				continue;
		}
		if review.provider().to_owned() == ProviderEnum::Bitbucket.to_string()
			&& alias_handle.bitbucket.is_some() {
				let bb_handles = alias_handle.bitbucket.expect("Empty github handles");
				update_handles_in_db(&alias_handle.git_alias, &review.provider(), bb_handles.clone());
				aliases_map.insert(alias_handle.git_alias, bb_handles);
		}
	}
	if aliases_map.is_empty() {
		log::error!(
			"[get_handles_from_server] No aliases found for review - {:?}",
			&review);
		return None;
	}
	Some(aliases_map)
}

pub async fn get_access_token (review: &Option<Review>, provider: &str) -> Option<String> {
	let access_token: String;
	if provider == ProviderEnum::Bitbucket.to_string().to_lowercase() {
		let access_token_opt = bitbucket::auth::refresh_git_auth(review).await;
		if access_token_opt.is_none() {
			log::error!("[get_access_token] Unable to get access token, review: {:?}",
				&review);
			return None;
		}
		access_token = access_token_opt.expect("Empty access_token_opt");
	} 
	else if provider == ProviderEnum::Github.to_string().to_lowercase(){
		let access_token_opt = github::auth::gh_access_token(review).await;
		if access_token_opt.is_none() {
			log::error!("[get_access_token] Unable to get access token, review: {:?}",
				&review);
			return None;
		}
		access_token = access_token_opt.expect("Empty access_token");
	} else {
		log::error!("[git pull] | repo provider is not github or bitbucket");
		return None;
	}
	return Some(access_token);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSelectedRepo {
	pub repo_name: String,
	pub repo_owner: String,
	pub repo_provider: String,
}

pub async fn user_selected_repos(provider: &str) -> Option<Vec<Repository>> {
	let client = get_client();
	let topic_name = env::var("INSTALL_ID").expect("INSTALL_ID must be set");
	let server_prefix_url = env::var("SERVER_URL").expect("SERVER_URL must be set");
	let url = format!("{}/api/dpu/repos?topicId={}&provider={}",
					  &server_prefix_url, &topic_name, &provider);
	let repos_res = client.get(url).send().await;
	if let Err(e) = repos_res {
		log::error!("[user_selected_repos] Unable to get repos from server, {:?}", e);
		return None;
	}
	let repos_response = repos_res.expect("Uncaught error in repos_res");
	return parse_repos_response(repos_response).await;
}

async fn parse_repos_response(response: Response) -> Option<Vec<Repository>> {
	let repos_value_res = response.json::<Value>().await;
	log::debug!("[parse_repos_response] ========================================<><><><><> repos_values_res = {:?}", &repos_value_res);
	if let Err(e) = &repos_value_res {
		log::error!("[parse_repos_response] Unable to parse response {:?}", e);
		return None;
	}
	let repos_value = repos_value_res.expect("Uncaught error in repos_value_res");
	let repolist_json_opt = repos_value.get("repoList");
	if repolist_json_opt.is_none() {
		log::error!("[parse_repos_response] No repoList key in response json: {:?}", &repos_value);
		return None;
	}
	let repolist_json = repolist_json_opt.expect("Uncaught error in repolist_json_opt");
	let repolist_vec_res: Result<Vec<BitbucketDBRepo>, _> = serde_json::from_value(repolist_json.clone());
	if let Err(e) = &repolist_vec_res {
		log::error!("[parse_repos_response] Unable to parse vec of BitbucketDBRepo from response: {:?}", e);
		return None;
	}
	let repolist_vec: Vec<BitbucketDBRepo> = repolist_vec_res.expect("Uncaught error in repolist_vec_res");

	// Transform BitbucketDBRepo to Repository
	let repositories: Vec<Repository> = repolist_vec.into_iter().map(|repo| {
		Repository::new(
			repo.repo_name,
			repo.uuid.unwrap_or_else(String::new),                // Handle Option<String>
			repo.repo_owner,
			repo.is_private.unwrap_or(false),                     // Handle Option<bool>
			repo.clone_ssh_url.unwrap_or_else(String::new),       // Handle Option<String>
			repo.project_name,
			repo.project_type,
			repo.workspace.unwrap_or_else(String::new),           // Handle Option<String>
			None,
			repo.repo_provider)
	}).collect();

	return Some(repositories);
}