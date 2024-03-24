use serde_json::Value;

use crate::{core::{review::{commit_check, process_review_changes, send_hunkmap}, utils::get_access_token}, db::{repo::get_clone_url_clone_dir, repo_config::save_repo_config_to_db, review::get_review_from_db}, github::prs::get_and_store_pr_info, utils::{repo_config::RepoConfig, review::Review, user::ProviderEnum}};

#[derive(Debug)]
struct TriggerRepo {
    repo_name: String,
    repo_owner: String,
    repo_provider: String,
    pr_number: String,
}
pub async fn process_trigger(message_data: &Vec<u8>) {
    // parse message
    let parse_res = parse_trigger_msg(message_data);
    // create review object from db
    if parse_res.is_none() {
        log::error!("[process_trigger] Unable to parse message: {:?}", &message_data);
        return;
    }
    let (trigger_repo, repo_config) = parse_res.expect("Empty parse_res");
    log::info!("[process_trigger] Processing PR: {} in repo: {}", &trigger_repo.pr_number, &trigger_repo.repo_name);
    // get access token
    if trigger_repo.repo_provider != ProviderEnum::Github.to_string().to_lowercase() {
        log::error!("[process_trigger] Not implemented for non github providers");
        return;
    }
    let access_token_opt = get_access_token(&None, &trigger_repo.repo_provider).await;
	if access_token_opt.is_none() {
		log::error!("[process_trigger] Unable to retrieve access token, failing, message: {:?}",
			&trigger_repo);
		return;
	}
    let access_token = access_token_opt.expect("Empty access_token_opt");
    let review_opt = get_review_obj(&trigger_repo, &access_token).await;
    if review_opt.is_none() {
        log::error!("[process_trigger] Unable to get review details: {:?}", &trigger_repo);
        return;
    }
    let review = review_opt.expect("Empty review_opt");
    // commit_check
    commit_check(&review, &access_token).await;
    // process_review_changes
	let hunkmap_opt = process_review_changes(&review).await;
    // send_hunkmap
	send_hunkmap(&hunkmap_opt, &review, &repo_config, &access_token).await;
}


fn parse_trigger_msg(message_data: &Vec<u8>) -> Option<(TriggerRepo, RepoConfig)> {
	let data_res = serde_json::from_slice::<Value>(&message_data);
	if data_res.is_err() {
		let e = data_res.expect_err("No error in data_res");
		log::error!("[parse_trigger_msg] Incoming message does not contain valid triggers: {:?}", e);
		return None;
	}
	let deserialized_data = data_res.expect("Uncaught error in deserializing message_data");
	log::debug!("[parse_trigger_msg] deserialized_data == {:?}", &deserialized_data);
	let repo_provider = deserialized_data["repo_provider"].to_string().trim_matches('"').to_string();
    let repo_owner = deserialized_data["repo_owner"].to_string().trim_matches('"').to_string();
    let repo_name = deserialized_data["repo_name"].to_string().trim_matches('"').to_string();
    let pr_number = deserialized_data["pr_number"].to_string().trim_matches('"').to_string();
    let repo_config_res = serde_json::from_value(deserialized_data["repo_config"].clone());
    if let Err(e) = &repo_config_res {
        log::error!("[parse_trigger_msg] Error in parsing repo config: {:?}", e);
        return None;
    }
    let repo_config: RepoConfig = repo_config_res.expect("Uncaught error in repo_config_res");
    let trigger_repo = TriggerRepo { repo_name, repo_owner, repo_provider, pr_number };
    return Some((trigger_repo, repo_config));
}

async fn get_review_obj(trigger_repo: &TriggerRepo, access_token: &str) -> Option<Review> {
    let pr_info_opt = get_and_store_pr_info(&trigger_repo.repo_owner, &trigger_repo.repo_name, access_token, &trigger_repo.pr_number).await;
    if pr_info_opt.is_none() {
        log::error!("[get_review_obj] Unable to get and store pr info: {:?}", &trigger_repo);
        return None;
    }
    let pr_info = pr_info_opt.expect("Empty pr_info_opt");
    let clone_opt = get_clone_url_clone_dir(&trigger_repo.repo_provider, &trigger_repo.repo_owner, &trigger_repo.repo_name);
	if clone_opt.is_none() {
		log::error!("[get_review_obj] Unable to get clone url and directory for bitbucket review");
		return None;
	}
	let (clone_url, clone_dir) = clone_opt.expect("Empty clone_opt");
    let author_opt = pr_info.author;
    if author_opt.is_none() {
        log::error!("[get_review_obj] Unable to get pr author");
        return None;
    }
    let author = author_opt.expect("Empty author_opt");
    let review = Review::new(
		pr_info.base_head_commit,
		pr_info.pr_head_commit,
		trigger_repo.pr_number.clone(),
		trigger_repo.repo_name.clone(),
		trigger_repo.repo_owner.clone(),
		trigger_repo.repo_provider.clone(),
		format!("github/{}/{}/{}", 
            &trigger_repo.repo_owner, &trigger_repo.repo_name, &trigger_repo.pr_number),
		clone_dir,
		clone_url,
		author,
		None,
	);
    return Some(review);
}