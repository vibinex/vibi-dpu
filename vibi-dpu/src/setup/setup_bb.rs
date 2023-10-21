use std::env;
use std::str;
use std::io::ErrorKind;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::{task, fs};

use crate::bitbucket::auth::get_access_token_from_bitbucket;
use crate::client::config::get_client;
use crate::bitbucket::repo::get_workspace_repos;
use crate::bitbucket::workspace::get_bitbucket_workspaces;
use crate::bitbucket::webhook::{get_webhooks_in_repo, add_webhook};
use crate::bitbucket::user::get_and_save_workspace_users;
use crate::bitbucket::prs::{list_prs_bitbucket, get_and_store_pr_info};
use crate::db::repo::save_repo_to_db;
use crate::db::webhook::save_webhook_to_db;
use crate::utils::repo::Repository;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SetupInfo {
    provider: String,
    owner: String,
    repos: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PublishRequest {
    installationId: String,
    info: Vec<SetupInfo>,
}

pub async fn handle_install_bitbucket(installation_code: &str) {
    // get access token from installation code by calling relevant repo provider's api
    // out of github, bitbucket, gitlab

    let authinfo_opt = get_access_token_from_bitbucket(installation_code).await;
    if authinfo_opt.is_none() {
        eprintln!("Unable to get authinfo in get_access_token_from_bitbucket");
        return;
    }
    let authinfo = authinfo_opt.expect("Empty authinfo_opt");
    println!("AuthInfo: {:?}", authinfo);
    // let auth_info = { "access_token": access_token, "expires_in": expires_in_formatted, "refresh_token": auth_info["refresh_token"] }; db.insert("auth_info", serde_json::to_string(&auth_info).unwrap());
    let access_token = authinfo.access_token().clone();
    let user_workspaces = get_bitbucket_workspaces(&access_token).await;
    let mut pubreqs: Vec<SetupInfo> = Vec::new();
    for workspace in user_workspaces {
        let workspace_slug = workspace.slug();
        println!("=========<{:?}>=======", workspace_slug);
    
        let repos = get_workspace_repos(workspace.uuid(), 
            &access_token).await;
        get_and_save_workspace_users(workspace.uuid(), &access_token).await;
        let mut reponames: Vec<String> = Vec::new();
        for repo in repos.expect("repos is None") {
            let token_copy = access_token.clone();
            let mut repo_copy = repo.clone();
            clone_git_repo(&mut repo_copy, &token_copy).await;
            let repo_name = repo.name();
            reponames.push(repo_name.clone());
            println!("Repo url git = {:?}", &repo.clone_ssh_url());
            println!("Repo name = {:?}", repo_name);
            process_webhooks(workspace_slug.to_string(),
            repo_name.to_string(),
            access_token.to_string()).await;
            let repo_name_async = repo_name.clone();
            let workspace_slug_async = workspace_slug.clone();
            let access_token_async = access_token.clone();
            task::spawn(async move {
                let prs = list_prs_bitbucket(&workspace_slug_async, &repo_name_async, &access_token_async, "OPEN").await;
                if prs.is_empty() {
                    println!("No open pull requests found for processing.");
                    return;
                }
                // We can concurrently process each PR with tokio::spawn.
                let handles: Vec<_> = prs.into_iter().map(|pr| {
                    let workspace_slug_async = workspace_slug_async.clone(); //Instead of cloning each time, I could have used ARC but not sure what is the best way.
                    let repo_name_async = repo_name_async.clone();
                    let access_token_async = access_token_async.clone();
                    tokio::spawn(async move {
                        get_and_store_pr_info(&workspace_slug_async, &repo_name_async, &access_token_async, &pr.to_string()).await;
                    })
                }).collect();
        
                // Wait for all async tasks to complete.
                for handle in handles {
                    if let Err(e) = handle.await {
                        eprintln!("Error in processing PR: {:?}", e);
                    }
                }                
            });
        }
        pubreqs.push(SetupInfo {
            provider: "bitbucket".to_owned(),
            owner: workspace_slug.clone(),
            repos: reponames });
    } 
    send_setup_info(&pubreqs).await;
}


async fn send_setup_info(setup_info: &Vec<SetupInfo>) {
    let installation_id = env::var("INSTALL_ID")
        .expect("INSTALL_ID must be set");
    println!("install_id = {:?}", &installation_id);
    let base_url = env::var("SERVER_URL")
        .expect("SERVER_URL must be set");
    let body = PublishRequest {
        installationId: installation_id,
        info: setup_info.to_vec(),
    };
    println!("body = {:?}", &body);
    let client = get_client();
    let setup_url = format!("{base_url}/api/dpu/setup");
    let post_res = client
      .post(&setup_url)
      .json(&body)
      .send()
      .await;
    if post_res.is_err() {
        let e = post_res.expect_err("No error in post_res in send_setup_info");
        eprintln!("error in send_setup_info post_res: {:?}, url: {:?}", e, &setup_url);
        return;
    }
    let resp = post_res.expect("Uncaught error in post_res");
    println!("Response: {:?}", resp.text().await);
}

async fn clone_git_repo(repo: &mut Repository, access_token: &str) {
    let git_url = repo.clone_ssh_url();
    let clone_url = git_url.to_string()
        .replace("git@", format!("https://x-token-auth:{{{access_token}}}@").as_str())
        .replace("bitbucket.org:", "bitbucket.org/");
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let mut directory = format!("/tmp/{}/{}/{}", repo.provider(), 
        repo.workspace(), random_string);
    // Check if directory exists
    let exists_res = fs::metadata(&directory).await;
    if exists_res.is_err() {
        let e = exists_res.expect_err("No error in exists_res");
        println!("executing metadata in {:?}, output: {:?}",
                &directory, e);
        if e.kind() != ErrorKind::NotFound {
            return;
        }
    }
    let remove_dir_opt = fs::remove_dir_all(&directory).await;
    if remove_dir_opt.is_err() {
        let e = remove_dir_opt.expect_err("No error in remove_dir_opt");
        println!("Execute in directory: {:?}, remove_dir_all: {:?}",
            &directory, e);
        if e.kind() != ErrorKind::NotFound {
            return;
        }
    }
    let create_dir_opt = fs::create_dir_all(&directory).await;
    if create_dir_opt.is_err() {
        let e = create_dir_opt.expect_err("No error in create_dir_opt");
        println!("Executing in directory: {:?}, create_dir_all: {:?}",
            &directory, e);
        if e.kind() != ErrorKind::NotFound {
            return;
        }
    }
    println!("directory exists? {}", fs::metadata(&directory).await.is_ok());
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg(clone_url).current_dir(&directory);
    let output_res = cmd.output();
    if output_res.is_err() {
        let e = output_res.expect_err("No error in output_res in git clone");
        eprintln!("Executing in directory: {:?}, git clone: {:?}",
            &directory, e);
        return;
    }
    let output = output_res.expect("Uncaught error in output_res");
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("git pull stderr = {:?}", v),
		Err(e) => {/* error handling */ println!("git clone stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("git pull stdout = {:?}", v),
		Err(e) => {/* error handling */ println!("git clone stdout error {}", e)}, 
	};
    directory = format!("{}/{}", &directory, repo.name());
    repo.set_local_dir(&directory);
    save_repo_to_db(repo);
}

async fn process_webhooks(workspace_slug: String, repo_name: String, access_token: String) {
    let webhooks_data = get_webhooks_in_repo(
        &workspace_slug, &repo_name, &access_token).await;
    let webhook_callback_url = format!("{}/api/bitbucket/callbacks/webhook", 
        env::var("SERVER_URL").expect("SERVER_URL must be set"));
    if webhooks_data.is_empty() {
        println!("Adding new webhook...");
        let repo_name_async = repo_name.clone();
        let workspace_slug_async = workspace_slug.clone();
        let access_token_async = access_token.clone();
        task::spawn(async move {
            add_webhook(
                &workspace_slug_async, 
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
        let workspace_slug_async = workspace_slug.clone();
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