use serde_json::Value;

use crate::{core::{review::{commit_check, process_review_changes, send_hunkmap}, utils::get_access_token}, db::{repo::get_clone_url_clone_dir, repo_config::save_repo_config_to_db, review::get_review_from_db}, github::prs::get_and_store_pr_info, utils::{repo_config::RepoConfig, review::Review, user::ProviderEnum}};

#[derive(Debug)]
struct TriggerReview {
	repo_provider: String,
	repo_owner: String,
	repo_name: String,
	pr_number: String
}

pub async fn process_trigger(message_data: &Vec<u8>) {
	// parse message
	let parse_res = parse_trigger_msg(message_data);
	// create review object from db
	if parse_res.is_none() {
		log::error!("[process_trigger] Unable to parse message: {:?}", &message_data);
		return;
	}
	let (trigger_review, repo_config) = parse_res.expect("Empty parse_res");
	log::info!("[process_trigger] Processing PR: {} in repo: {}", &trigger_review.pr_number, &trigger_review.repo_name);
	// get access token
	if trigger_review.repo_provider != ProviderEnum::Github.to_string().to_lowercase() {
		log::error!("[process_trigger] Not implemented for non github providers");
		return;
	}
	let access_token_opt = get_access_token(&None, &trigger_review.repo_provider).await;
	if access_token_opt.is_none() {
		log::error!("[process_trigger] Unable to retrieve access token, failing, message: {:?}",
			&trigger_review);
		return;
	}
	let access_token = access_token_opt.expect("Empty access_token_opt");
	// get pr information and update review object
	let pr_info_opt = get_and_store_pr_info(&trigger_review.repo_owner,
		&trigger_review.repo_name, &access_token, &trigger_review.pr_number).await;
	if pr_info_opt.is_none() {
		log::error!("[process_trigger] Unable to get pr info from provider");
		return;
	}
	let review_opt = get_review_obj(&trigger_review, &access_token).await;
	if review_opt.is_none() {
		log::error!("[process_trigger] Unable to get review details: {:?}", &trigger_review);
		return;
	}
	let review = review_opt.expect("Empty review_opt");
	// commit_check
	commit_check(&review, &access_token).await;
	// process_review_changes
	let hunkmap_opt = process_review_changes(&review).await;
	// send_hunkmap
	send_hunkmap(&hunkmap_opt, &review, &repo_config, &access_token, &None).await;
}

fn parse_field(field_name: &str, msg: &Value) -> Option<String> {
	let field_val_opt = msg.get(field_name);
	if field_val_opt.is_none() {
		log::error!("[parse_field] {} not found in {}", field_name, msg);
		return None;
	}
	let field_val = field_val_opt.expect("Empty field_val_opt");
	return Some(field_val.to_string().trim_matches('"').to_string());
}

fn parse_message_fields(msg: &Value) -> Option<TriggerReview> {
	let repo_provider_opt = parse_field("repo_provider", msg);
	let repo_owner_opt = parse_field("repo_owner", msg);
	let repo_name_opt = parse_field("repo_name", msg);
	let pr_number_opt = parse_field("pr_number", msg);
	if repo_provider_opt.is_none() || repo_owner_opt.is_none() 
		|| repo_name_opt.is_none() || pr_number_opt.is_none() {
		log::error!("[parse_message_fields] Could not parse {:?}, {:?}, {:?}, {:?}", 
			repo_provider_opt, repo_owner_opt, repo_name_opt, pr_number_opt);
		return None;
	}
	let repo_provider = repo_provider_opt.expect("Empty repo_provider_opt");
	let repo_owner = repo_owner_opt.expect("Empty repo_provider_opt");
	let repo_name = repo_name_opt.expect("Empty repo_provider_opt");
	let pr_number = pr_number_opt.expect("Empty repo_provider_opt");
	return Some(TriggerReview { repo_provider, repo_owner, repo_name, pr_number });
}

fn parse_trigger_msg(message_data: &Vec<u8>) -> Option<(TriggerReview, RepoConfig)> {
	let data_res = serde_json::from_slice::<Value>(&message_data);
	if data_res.is_err() {
		let e = data_res.expect_err("No error in data_res");
		log::error!("[parse_trigger_msg] Incoming message does not contain valid triggers: {:?}", e);
		return None;
	}
	let deserialized_data = data_res.expect("Uncaught error in deserializing message_data");
	log::debug!("[parse_trigger_msg] deserialized_data == {:?}", &deserialized_data);
	let trigger_review_opt = parse_message_fields(&deserialized_data);
	if trigger_review_opt.is_none() {
		log::error!("[parse_trigger_msg] Unable to parse message fields: {:?}", &deserialized_data);
		return None;
	}
	let trigger_review = trigger_review_opt.expect("Empty trigger_review_opt");
	let repo_config_res = serde_json::from_value(deserialized_data["repo_config"].clone());
	if let Err(e) = &repo_config_res {
		log::error!("[parse_trigger_msg] Error in parsing repo config: {:?}", e);
		return None;
	}
	let repo_config: RepoConfig = repo_config_res.expect("Uncaught error in repo_config_res");
	save_repo_config_to_db(&repo_config, &trigger_review.repo_name, &trigger_review.repo_owner, &trigger_review.repo_provider);
	return Some((trigger_review, repo_config));
}

async fn get_review_obj(trigger_review: &TriggerReview, access_token: &str) -> Option<Review> {
	let pr_info_opt = get_and_store_pr_info(&trigger_review.repo_owner, &trigger_review.repo_name, access_token, &trigger_review.pr_number).await;
	if pr_info_opt.is_none() {
		log::error!("[get_review_obj] Unable to get and store pr info: {:?}", &trigger_review);
		return None;
	}
	let pr_info = pr_info_opt.expect("Empty pr_info_opt");
	let clone_opt = get_clone_url_clone_dir(&trigger_review.repo_provider, &trigger_review.repo_owner, &trigger_review.repo_name);
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
		trigger_review.pr_number.clone(),
		trigger_review.repo_name.clone(),
		trigger_review.repo_owner.clone(),
		trigger_review.repo_provider.clone(),
		format!("github/{}/{}/{}", 
			&trigger_review.repo_owner, &trigger_review.repo_name, &trigger_review.pr_number),
		clone_dir,
		clone_url,
		author,
		None,
	);
	return Some(review);
}