use serde_json::Value;

use crate::{core::utils::get_access_token, db::{repo_config::save_repo_config_to_db, review::get_review_from_db}, github::prs::get_and_store_pr_info, utils::{repo_config::RepoConfig, review::Review, user::ProviderEnum}};

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
	// get pr information and update review object
	let pr_info_opt = get_and_store_pr_info(&trigger_repo.repo_owner,
		&trigger_repo.repo_name, &access_token, &trigger_repo.pr_number).await;
	if pr_info_opt.is_none() {
		log::error!("[process_trigger] Unable to get pr info from provider");
		return;
	}
	// commit_check
	// process_review_changes
	// send_hunkmap
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
	let review_opt = get_review_from_db(&trigger_review.repo_name,
		&trigger_review.repo_owner, &trigger_review.repo_provider, &trigger_review.pr_number);
	if review_opt.is_none() {
		log::error!("[parse_trigger_msg] Unable to get review from db: {:?}", &deserialized_data);
		return None;
	}
	let review = review_opt.expect("Empty review_opt");
	log::debug!("[parse_review] repo_config = {:?}", &repo_config);
	save_repo_config_to_db(&repo_config, &review.repo_name(), &review.repo_owner(), &review.provider());
	return Some((trigger_review, repo_config));
}