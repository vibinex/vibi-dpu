// setup_gh.rs
use std::env;
use std::str;
use serde_json::Value;
use tokio::task;

use crate::core::utils::send_aliases;
use crate::db::repo::get_repo_from_db;
use crate::github::auth::fetch_access_token; use crate::github::prs::{list_prs_github, get_and_store_pr_info};
use crate::github::repos::get_user_github_repos_using_graphql_api;
use crate::utils::gitops::get_git_aliases;
use crate::utils::parsing::parse_string_field_pubsub;
use crate::utils::repo::Repository;
// Import shared utilities
use crate::utils::setup_info::SetupInfo;
use crate::github::repos::get_github_app_installed_repos;
use crate::utils::gitops::clone_git_repo;
use crate::github::webhook::{get_webhooks_in_repo, add_webhook};
use crate::db::webhook::save_webhook_to_db;
use crate::core::utils::send_setup_info;

pub async fn handle_install_github(installation_code: &str) {
	let repo_provider = "github";
	let auth_info_opt = fetch_access_token(installation_code).await;
	log::debug!("[handle_install_github] auth_info = {:?}", &auth_info_opt);
	
	if auth_info_opt.is_none() {
		log::error!("[handle_install_github] Unable to get authinfo from fetch_access_token in Github setup");
		return;
	}
	let auth_info = auth_info_opt.expect("Empty authinfo_opt");
	let access_token = auth_info.token().clone();
	process_repos(&access_token, repo_provider).await;
}

pub async fn process_repos(access_token: &str, repo_provider: &str) {
	log::info!("Processing repos...");
	let mut pubreqs: Vec<SetupInfo> = Vec::new();
	let repos_opt = get_github_app_installed_repos(&access_token).await;
	if repos_opt.is_none(){
		log::error!("[handle_install_github] No repositories found for GitHub app");
		return;
	}
	let repos = repos_opt.expect("Empty repos option");
	log::debug!("[handle_install_github] Got repos: {:?}", repos);
	let repo_owner = repos[0].owner().clone();
	let mut repo_names: Vec<String> = Vec::new();

	for repo in repos{
		let token_copy = access_token.to_owned().clone();
		let mut repo_copy = repo.clone();
		clone_git_repo(&mut repo_copy, &token_copy, repo_provider).await;
		let aliases_opt = get_git_aliases(&repo_copy);
		if aliases_opt.is_none() {
			log::error!("[handle_install_github] Unable to get aliases for repo: {}", repo.name());
			continue;
		}
		let aliases = aliases_opt.expect("Empty aliases_opt");
		send_aliases(&repo, &aliases).await;
		let repo_name = repo.name();
		repo_names.push(repo_name.clone());
		log::debug!("[handle_install_github] Repo url git = {:?}", &repo.clone_ssh_url());
		log::debug!("[handle_install_github] Repo name = {:?}", repo_name);
		let repo_owner = repo.owner();
		process_webhooks(repo_owner.to_string(), repo_name.to_string(), access_token.to_string()).await;
		let repo_name_async = repo_name.clone();
		let repo_owner_async = repo_owner.clone();
		let access_token_async = access_token.to_owned().clone();
		task::spawn(async move {
			process_prs(&repo_owner_async, &repo_name_async, &access_token_async).await;
		});
	}
	pubreqs.push(SetupInfo {
		provider: "github".to_owned(),
		owner: repo_owner.clone(),
		repos: repo_names
	});
	send_setup_info(&pubreqs).await;
}


async fn process_webhooks(repo_owner: String, repo_name: String, access_token: String) {
	log::info!("Processing webhooks for : {}...", &repo_name);
	let webhooks_data_opt = get_webhooks_in_repo(&repo_owner, &repo_name, &access_token).await;
	if webhooks_data_opt.is_none() {
		log::error!("[process_webhooks] Unable to get webhooks for repo: {:?}, other params: {:?}, {:?}",
			repo_name, repo_owner, access_token);
		return;
	}
	let webhooks_data = webhooks_data_opt.expect("Empty webhooks_data_opt");
	let webhook_callback_url = format!("{}/api/github/callbacks/webhook", 
		env::var("SERVER_URL").expect("SERVER_URL must be set"));
	log::debug!("[process_webhooks] webhooks_data = {:?}", &webhooks_data);
	let matching_webhook = webhooks_data.into_iter()
		.find(|w| w.url().to_string() == webhook_callback_url);
	log::debug!("[process_webhooks] matching_webhook = {:?}", &matching_webhook);
	if matching_webhook.is_none() {
		let repo_name_async = repo_name.clone();
		let workspace_slug_async = repo_owner.clone();
		let access_token_async = access_token.clone();
		task::spawn(async move {
			add_webhook(
				&workspace_slug_async, 
				&repo_name_async, 
				&access_token_async).await;
		});
		return;
	}
	let webhook = matching_webhook.expect("no matching webhook");
	log::debug!("[process_webhooks] Webhook already exists: {:?}", &webhook);
	save_webhook_to_db(&webhook);
}

async fn process_prs(repo_owner_async: &String, repo_name_async: &String, access_token_async: &String) {
	log::info!("Processing all open pull requests...");
	let pr_list_opt = list_prs_github(&repo_owner_async, &repo_name_async, &access_token_async, "OPEN").await;
	if pr_list_opt.is_none() {
		log::warn!("Unable to get any open pull requests for processing.");
		return;
	}
	let pr_list = pr_list_opt.expect("Empty pr_list_opt");

	for pr_id in &pr_list {
		let repo_owner = repo_owner_async.clone(); //Instead of cloning each time, I could have used ARC but not sure what is the best way.
		let repo_name = repo_name_async.clone();
		let access_token = access_token_async.clone();
		let pr_id_async = pr_id.clone();
		task::spawn(async move {
			get_and_store_pr_info(&repo_owner, &repo_name, &access_token, &pr_id_async).await;
		});
	}
	
}

pub async fn process_pat_repos(message_data: &[u8]) {
	let repos_opt = parse_pat_repos(message_data);
	let repo_provider = env::var("PROVIDER").expect("provider must be set").to_lowercase();
	let access_token = env::var("GITHUB_PAT").expect("GITHUB_PAT must be set");
	if repos_opt.is_none() {
		log::error!("[process_PAT_repos] Failed to parse PAT repos data");
		return;
	}
	let repos = repos_opt.expect("Empty repos option");
	for repo in repos {
		let mut repo_copy = repo.clone();
		clone_git_repo(&mut repo_copy, &access_token, &repo_provider).await;
		let repo_name = repo.name();
		let repo_owner = repo.owner();
		process_webhooks(repo_owner.to_string(), 
			repo_name.to_string(), access_token.to_string()).await;
		let repo_name_async = repo_name.clone();
		let repo_owner_async = repo_owner.clone();
		let access_token_async = access_token.to_string().clone();
		task::spawn(async move {
			process_prs(&repo_owner_async, &repo_name_async, &access_token_async).await;
		});
	}
}

pub async fn setup_self_host_user_repos_github(access_token: &str) {
	log::info!("Getting all user's repositories...");
	let repos_opt = get_user_github_repos_using_graphql_api(&access_token).await;
	if repos_opt.is_none() {
		log::error!("[setup_self_host_user_repos_github] No repositories found for the user");
		return;
	}
	let repos = repos_opt.expect("Empty repos option");
	
	log::debug!("[setup_self_host_user_repos_github] Got repos: {:?}", repos);

	// Create a mapping between repo_owner and associated repo_names
	let mut repo_owner_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
	for repo in repos {
		let repo_name = repo.name();
		let repo_owner = repo.owner();
		repo_owner_map
		.entry(repo_owner.to_string())
		.or_insert_with(Vec::new)
		.push(repo_name.to_string());
		
		log::debug!(
			"[setup_self_host_user_repos_github] Repo url git = {:?}",
			&repo.clone_ssh_url()
		);
	}
	let mut pubreq_vec =  Vec::<SetupInfo>::new();
	// Send a separate pubsub publish request for each unique repo_owner
	for (repo_owner, repo_names) in repo_owner_map {
		let pubreq = SetupInfo {
			provider: "github".to_owned(),
			owner: repo_owner,
			repos: repo_names,
		};
		pubreq_vec.push(pubreq);
	}
	send_setup_info(&pubreq_vec).await;
}

fn parse_pat_repos(message_data: &[u8]) -> Option<Vec<Repository>> {
	let data_res = serde_json::from_slice::<Vec<Value>>(&message_data);
	if data_res.is_err() {
		let e = data_res.expect_err("No error in data_res");
		log::error!("[parse_pat_repos] Error parsing incoming messages: {:?}", e);
		return None;
	}
	let deserialized_data = data_res.expect("Uncaught error in deserializing message_data");
	let user_repos_opt = parse_user_repos(&deserialized_data);
	if user_repos_opt.is_none() {
		log::error!("[parse_pat_repos] Unable to parse message and get user repos");
		return None;
	}
	let user_repos = user_repos_opt.expect("Empty user_repos_opt");
	let mut all_repos = Vec::<Repository>::new();
	for user_setup_info in user_repos {
		let provider = &user_setup_info.provider;
		let owner = &user_setup_info.owner;
		for repo_name in user_setup_info.repos.iter() { 
			let repo_db_opt = get_repo_from_db(provider, owner, repo_name);
			if repo_db_opt.is_none() { continue; }
			let repo_db = repo_db_opt.expect("Empty repo_db_opt");
			all_repos.push(repo_db);
		}
	}
	log::debug!("[parse_pat_repos] Successfully parsed repos: {:?}", &all_repos);
	return Some(all_repos);
}

fn parse_user_repos(msg: &Vec<Value>) -> Option<Vec<SetupInfo>> {
	let mut setup_info_vec = Vec::<SetupInfo>::new();
	for owner_item in msg {
		let repo_provider_opt = parse_string_field_pubsub("provider", owner_item);
		let repo_owner_opt = parse_string_field_pubsub("owner", owner_item);
		let repo_names_res = serde_json::from_value::<Vec<String>>(owner_item["repos"].clone());
		if repo_provider_opt.is_none() || repo_names_res.is_err() || repo_owner_opt.is_none() {
			log::error!("[parse_user_repos] Unable to parse item fields --> {:?}, {:?}, {:?}", &repo_provider_opt, repo_owner_opt, repo_names_res);
			continue;
		}
		let setup_info_item = SetupInfo {
			provider: repo_provider_opt.expect("Empty repo_provider_opt"),
			owner: repo_owner_opt.expect("Empty repo_owner_opt"),
			repos: repo_names_res.expect("Empty repo_names_opt")
		};
		setup_info_vec.push(setup_info_item);
	}
	
	return Some(setup_info_vec);
}
