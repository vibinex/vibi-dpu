// setup_gh.rs
use std::env;
use std::str;
use tokio::task;

use crate::github::auth::fetch_access_token; // Import shared utilities
use crate::utils::setup_info::SetupInfo;
use crate::github::repos::get_github_app_installed_repos;
use crate::utils::gitops::clone_git_repo;
use crate::github::webhook::{get_webhooks_in_repo, add_webhook};
use crate::db::webhook::save_webhook_to_db;
use crate::core::utils::send_setup_info;

pub async fn handle_install_github(installation_code: &str) {
    let repo_provider = "github";
    let auth_info_opt = fetch_access_token(installation_code).await;
    println!("[handle_install_github] auth_info = {:?}", &auth_info_opt);
    
    if auth_info_opt.is_none() {
        eprintln!("Unable to get authinfo from fetch_access_token in Github setup");
        return;
    }
    let auth_info = auth_info_opt.expect("Empty authinfo_opt");
    let access_token = auth_info.access_token().clone();
    
    let mut pubreqs: Vec<SetupInfo> = Vec::new();
    let repos_opt = get_github_app_installed_repos(&access_token).await;
    if repos_opt.is_none(){
        eprintln!("No repositories found for GitHub app");
        return;
    }
    let repos = repos_opt.expect("Empty repos option");
    println!("Got repos: {:?}", repos);
    let repo_owner = repos[0].owner().clone();
    let mut repo_names: Vec<String> = Vec::new();

    for repo in repos{
        let token_copy = access_token.clone();
        let mut repo_copy = repo.clone();
        clone_git_repo(&mut repo_copy, &token_copy, &repo_provider).await;
        let repo_name = repo.name();
        repo_names.push(repo_name.clone());
        println!("Repo url git = {:?}", &repo.clone_ssh_url());
        println!("Repo name = {:?}", repo_name);
        let repo_owner = repo.owner();
        process_webhooks(repo_owner.to_string(), repo_name.to_string(), access_token.to_string()).await;
        let repo_name_async = repo_name.clone();
        let repo_owner_async = repo_owner.clone();
        let access_token_async = access_token.clone();
        // task::spawn(async move {
        //     process_prs(&repo_owner_async, &repo_name_async, &access_token_async).await
        // })
    }
    pubreqs.push(SetupInfo {
        provider: "github".to_owned(),
        owner: repo_owner.clone(),
        repos: repo_names
    });
    send_setup_info(&pubreqs).await;

}


async fn process_webhooks(repo_owner: String, repo_name: String, access_token: String) {
    let webhooks_data = get_webhooks_in_repo(&repo_owner, &repo_name, &access_token).await;
    let webhook_callback_url = format!("{}/api/github/callbacks/webhook", 
        env::var("SERVER_URL").expect("SERVER_URL must be set"));
    if webhooks_data.is_empty() {
        println!("Adding new webhook...");
        let repo_name_async = repo_name.clone();
        let repo_owner_async = repo_owner.clone();
        let access_token_async = access_token.clone();
        task::spawn(async move {
            add_webhook(
                &repo_owner_async, 
                &repo_name_async, 
                &access_token_async).await;
        });
        return;
    }
    let matching_webhook = webhooks_data.into_iter()
        .find(|w| w.url().to_string() == webhook_callback_url);
    if matching_webhook.is_none() {
        println!("Adding new webhook...");
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
    println!("Webhook already exists: {:?}", &webhook);
    save_webhook_to_db(&webhook);
}

async fn process_prs(repo_owner: &String, repo_name: &String, access_token: &String) {

}
