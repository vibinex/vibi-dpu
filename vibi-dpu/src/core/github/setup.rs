// setup_gh.rs
use std::env;
use std::str;
use tokio::task;

use crate::core::utils::send_aliases;
use crate::github::auth::fetch_access_token; use crate::github::prs::{list_prs_github, get_and_store_pr_info};
use crate::github::repos::get_user_accessed_github_repos;
use crate::utils::gitops::get_git_aliases;
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
        let token_copy = access_token.clone();
        let mut repo_copy = repo.clone();
        clone_git_repo(&mut repo_copy, &token_copy, &repo_provider).await;
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
        let access_token_async = access_token.clone();
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
        log::info!("[process_webhooks] Adding new webhook...");
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
    log::info!("[process_webhooks] Webhook already exists: {:?}", &webhook);
    save_webhook_to_db(&webhook);
}

async fn process_prs(repo_owner_async: &String, repo_name_async: &String, access_token_async: &String) {
    let pr_list_opt = list_prs_github(&repo_owner_async, &repo_name_async, &access_token_async, "OPEN").await;
    if pr_list_opt.is_none() {
        log::info!("[process_prs] No open pull requests found for processing.");
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

pub async fn setup_self_host_user_repos_github(access_token: &str) {
    let repo_provider = env::var("PROVIDER").expect("provider must be set").to_lowercase();

    let repos_opt = get_user_accessed_github_repos(&access_token).await;
    if repos_opt.is_none() {
        log::error!("[setup_self_host_user_repos_github] No repositories found for the user");
        return;
    }
    let repos = repos_opt.expect("Empty repos option");
    log::debug!("[setup_self_host_user_repos_github] Got repos: {:?}", repos);
    let list = vec!["dev-profiler-27", "dev-profiler-28"];
    // Create a mapping between repo_owner and associated repo_names
    let mut repo_owner_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for repo in repos {
        let repo_name = repo.name();
        log::debug!("[setup_self_host_user_repos_github]/repo_name: {:?}", &repo_name.to_string());
        if list.contains(&repo_name.as_str()){
            log::debug!("[setup_self_host_user_repos_github]/repo_name inside for loop: {:?}", &repo_name.to_string());
            let mut repo_copy = repo.clone();
            clone_git_repo(&mut repo_copy, access_token, &repo_provider).await;
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
            log::debug!("[setup_self_host_user_repos_github] Repo name = {:?}", repo_name);
            process_webhooks(repo_owner.to_string(), repo_name.to_string(), access_token.to_string())
                .await;
    
            let repo_name_async = repo_name.clone();
            let repo_owner_async = repo_owner.clone();
            let access_token_async = access_token.to_string().clone();
            task::spawn(async move {
                process_prs(&repo_owner_async, &repo_name_async, &access_token_async).await;
            });
        }
    }

    // Send a separate pubsub publish request for each unique repo_owner
    for (repo_owner, repo_names) in repo_owner_map {
        let pubreq = SetupInfo {
            provider: "github".to_owned(),
            owner: repo_owner,
            repos: repo_names,
        };
        send_setup_info(&vec![pubreq]).await;
    }
}
