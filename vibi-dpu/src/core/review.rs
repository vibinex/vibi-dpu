use std::{env, thread, time::Duration};

use serde_json::Value;

use crate::{
    core::{relevance::process_relevance, utils::get_access_token},
    db::{
        hunk::{get_hunk_from_db, store_hunkmap_to_db},
        repo::get_clone_url_clone_dir,
        repo_config::save_repo_config_to_db,
        review::{get_review_from_db, save_review_to_db},
    },
    utils::{
        gitops::{commit_exists, generate_blame, generate_diff, get_excluded_files, git_pull, process_diffmap, StatItem},
        hunk::{HunkMap, PrHunkItem},
        repo_config::RepoConfig,
        reqwest_client::get_client,
        review::Review,
        user::ProviderEnum,
    },
};

pub async fn process_review(message_data: &Vec<u8>) {
	let (review_opt, old_review_opt) = parse_review(message_data);
	if review_opt.is_none() {
		log::error!("[process_review] Unable to deserialize review message and repo config");
		return;
	}
	let (review, repo_config) = review_opt.expect("parse_opt is empty");
	log::debug!("[process_review] deserialized repo_config, review = {:?}, {:?}", &repo_config, &review);
	if hunk_already_exists(&review) {
		return;
	}
	log::info!("Parsed task for review no : {}...", &review.id());
	let access_token_opt = get_access_token(&Some(review.clone()), review.provider()).await;

	if access_token_opt.is_none() {
		log::error!("[process_review] Unable to retrieve access token, failing, message: {:?}",
			&review);
		return;
	}
	let access_token = access_token_opt.expect("Empty access_token_opt");
	commit_check(&review, &access_token).await;
	process_review_changes(&review, &repo_config, &access_token, &old_review_opt).await;
}

pub async fn process_review_changes(review: &Review, repo_config: &RepoConfig, access_token: &str, old_review_opt: &Option<Review>) -> Option<(HunkMap, Vec<StatItem>, Vec<StatItem>)>{
	log::info!("Processing changes in code...");
	let (excluded_files, smallfiles) = get_included_and_excluded_files(&review).await;
	if repo_config.comment() || repo_config.auto_assign() {
		let hunkmap_opt = calculate_hunkmap(&review, &smallfiles).await;
		send_hunkmap(&hunkmap_opt, &review, &repo_config, &access_token, &old_review_opt).await;
	}
	
	if let Some(mermaid_graph) = generate_mermaid_graph(&excluded_files, &small_files, &review).await {
		send_mermaid_graph(&mermaid_graph, &review, &access_token).await;
	}
}

pub async fn send_hunkmap(hunkmap_opt: &Option<(HunkMap, Vec<StatItem>, Vec<StatItem>)>, review: &Review,
	repo_config: &RepoConfig, access_token: &str, old_review_opt: &Option<Review>) {
	if hunkmap_opt.is_none() {
		log::error!("[send_hunkmap] Empty hunkmap in send_hunkmap");
		return;
	}
	let (hunkmap, excluded_files, small_files) = hunkmap_opt.as_ref().expect("empty hunkmap_opt");
	log::debug!("HunkMap = {:?}", &hunkmap);
	store_hunkmap_to_db(&hunkmap, review);
	publish_hunkmap(&hunkmap);
	let hunkmap_async = hunkmap.clone();
	let review_async = review.clone();
	let mut repo_config_clone = repo_config.clone();
	process_relevance(&hunkmap_async, excluded_files, small_files, &review_async,
		&mut repo_config_clone, access_token, old_review_opt).await;
}

fn hunk_already_exists(review: &Review) -> bool {
	let hunk_opt = get_hunk_from_db(&review);
	if hunk_opt.is_none() {
		log::debug!("[hunk_already_exists] No hunk from get_hunk_from_db");
		return false;
	}
	let hunkmap = hunk_opt.expect("empty hunk from get_hunk_from_db");
	publish_hunkmap(&hunkmap);
	log::debug!("[hunk_already_exists] Hunk already in db!");
	return true;
}

fn get_included_and_excluded_files(review: &Review) -> Option<(Vec<StatItem>, Vec<StatItem>)> {
	let fileopt = get_excluded_files(&review);
	log::debug!("[process_review_changes] fileopt = {:?}", &fileopt);
	if fileopt.is_none() {
		log::error!("[process_review_changes] No files to review for PR {}", review.id());
		return None;
	}
	let (excluded_files, smallfiles) = fileopt.expect("fileopt is empty");
	return Some(( excluded_files, smallfiles));
}

async fn calculate_hunkmap(review: &Review, smallfiles: Vec<StatItem>) -> Option<HunkMap> {
	let mut prvec = Vec::<PrHunkItem>::new();
	let diffmap = generate_diff(&review, &smallfiles);
	log::debug!("[process_review_changes] diffmap = {:?}", &diffmap);
	let linemap = process_diffmap(&diffmap);
	log::debug!("[process_review_changes] linemap = {:?}", &linemap);
	let blamevec = generate_blame(&review, &linemap).await;
	log::debug!("[process_review_changes] blamevec = {:?}", &blamevec);
	let hmapitem = PrHunkItem::new(
		review.id().to_string(),
		review.author().to_string(),
		blamevec,
	);
	prvec.push(hmapitem);
	let hunkmap = HunkMap::new(review.provider().to_string(),
		review.repo_owner().to_string(), 
		review.repo_name().to_string(), 
		prvec,
		format!("{}/hunkmap", review.db_key()),
	);
	log::debug!("[process_review_changes] hunkmap: {:?}", hunkmap);
	return Some(hunkmap);
}

async fn generate_mermaid_graph(excluded_files: &Vec<StatItem>, small_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let all_diff_files: Vec<StatItem> = excluded_files
        .iter()
        .chain(small_files.iter())
        .cloned()
        .collect();
    
    mermaid_comment(&all_diff_files, review).await
}

pub async fn commit_check(review: &Review, access_token: &str) {
	if !commit_exists(&review.base_head_commit(), &review.clone_dir()) 
		|| !commit_exists(&review.pr_head_commit(), &review.clone_dir()) {
		log::info!("Executing git pull on repo {}...", &review.repo_name());
		thread::sleep(Duration::from_secs(1));
		git_pull(review, access_token).await;
	}
}

fn parse_review(message_data: &Vec<u8>) -> (Option<(Review, RepoConfig)>, Option<Review>) {
	let data_res = serde_json::from_slice::<Value>(&message_data);
	if data_res.is_err() {
		let e = data_res.expect_err("No error in data_res");
		log::error!("[parse_review] Incoming message does not contain valid reviews: {:?}", e);
		return (None, None);
	}
	let deserialized_data = data_res.expect("Uncaught error in deserializing message_data");
	log::debug!("[parse_review] deserialized_data == {:?}", &deserialized_data["eventPayload"]["repository"]);
	let repo_provider = deserialized_data["repositoryProvider"].to_string().trim_matches('"').to_string();

	let (review_opt, old_review_opt): (Option<Review>, Option<Review>) = if repo_provider == ProviderEnum::Bitbucket.to_string().to_lowercase() {
		create_and_save_bitbucket_review_object(&deserialized_data)
	} else if repo_provider == ProviderEnum::Github.to_string().to_lowercase() {
		(create_and_save_github_review_object(&deserialized_data), None)
	} else {
		(None, None)
	};

	if review_opt.is_none() {
		log::error!("[parse_review] | empty review object");
		return (None, old_review_opt);
	}
	let review = review_opt.expect("Empty review_opt");

	let repo_config_res = serde_json::from_value(deserialized_data["repoConfig"].clone());
	if repo_config_res.is_err() {
		let e = repo_config_res.expect_err("No error in repo_config_res");
		log::error!("[parse_review] Unable to deserialze repo_config_res: {:?}", e);
		let default_config = RepoConfig::default();
		return (Some((review, default_config)), old_review_opt);
	}
	let repo_config = repo_config_res.expect("Uncaught error in repo_config_res");
	log::debug!("[parse_review] repo_config = {:?}", &repo_config);
	save_repo_config_to_db(&repo_config, &review.repo_name(), &review.repo_owner(), &review.provider());
	return (Some((review, repo_config)), old_review_opt);
}

fn publish_hunkmap(hunkmap: &HunkMap) {
	let client = get_client();
	let hunkmap_json = serde_json::to_string(&hunkmap).expect("Unable to serialize hunkmap");
	let key_clone = hunkmap.db_key().to_string();
	tokio::spawn(async move {
		let url = format!("{}/api/hunks",
			env::var("SERVER_URL").expect("SERVER_URL must be set"));
		log::debug!("[publish_hunkmap] url for hunkmap publishing  {}", &url);
		match client
		.post(url)
		.json(&hunkmap_json)
		.send()
		.await {
			Ok(_) => {
				log::info!("Published code changes successfully: {} !", &key_clone);
			},
			Err(e) => {
				log::error!("[publish_hunkmap] Failed to publish hunkmap: {} for: {}", e, &key_clone);
			}
		};
	});
}

fn create_and_save_bitbucket_review_object(deserialized_data: &Value) -> (Option<Review>, Option<Review>) {
	log::debug!("[create_and_save_bitbucket_review_object] deserialised_data {}", deserialized_data);
	let workspace_name = deserialized_data["eventPayload"]["repository"]["workspace"]["slug"].to_string().trim_matches('"').to_string();
	let repo_name = deserialized_data["eventPayload"]["repository"]["name"].to_string().trim_matches('"').to_string();
	let repo_provider = ProviderEnum::Bitbucket.to_string().to_lowercase();
	let pr_id = deserialized_data["eventPayload"]["pullrequest"]["id"].to_string().trim_matches('"').to_string();
	let old_review_opt = get_review_from_db(&repo_name, &workspace_name,
		&repo_provider, &pr_id);
	let clone_opt = get_clone_url_clone_dir(&repo_provider, &workspace_name, &repo_name);
	if clone_opt.is_none() {
		log::error!("[create_and_save_bitbucket_review_object] Unable to get clone url and directory for bitbucket review");
		return (None, old_review_opt);
	}
	let (clone_url, clone_dir) = clone_opt.expect("Empty clone_opt");
	let review = Review::new(
		deserialized_data["eventPayload"]["pullrequest"]["destination"]["commit"]["hash"].to_string().replace("\"", ""),
		deserialized_data["eventPayload"]["pullrequest"]["source"]["commit"]["hash"].to_string().replace("\"", ""),
		pr_id.clone(),
		repo_name.clone(),
		workspace_name.clone(),
		repo_provider.clone(),
		format!("bitbucket/{}/{}/{}", &workspace_name, &repo_name, &pr_id),
		clone_dir,
		clone_url,
		deserialized_data["eventPayload"]["pullrequest"]["author"]["uuid"].to_string().replace("\"", ""),
		None,
	);
	log::debug!("[create_and_save_bitbucket_review_object] bitbucket review object= {:?}", &review);
	save_review_to_db(&review);
	return (Some(review), old_review_opt);
}

fn create_and_save_github_review_object(deserialized_data: &Value) -> Option<Review> {
	log::debug!("[create_and_save_github_review_object] deserialised_data {}", deserialized_data);
	let repo_owner = deserialized_data["eventPayload"]["repository"]["owner"]["login"].to_string().trim_matches('"').to_string();
	let repo_name = deserialized_data["eventPayload"]["repository"]["name"].to_string().trim_matches('"').to_string();
	let repo_provider = ProviderEnum::Github.to_string().to_lowercase();
	let clone_opt = get_clone_url_clone_dir(&repo_provider, &repo_owner, &repo_name);
	if clone_opt.is_none() {
		log::error!("[create_and_save_github_review_object] Unable to get clone url and directory for bitbucket review");
		return None;
	}
	let (clone_url, clone_dir) = clone_opt.expect("Empty clone_opt");
	let pr_id = deserialized_data["eventPayload"]["pull_request"]["number"].to_string().trim_matches('"').to_string();
	let review = Review::new(
		deserialized_data["eventPayload"]["pull_request"]["base"]["sha"].to_string().replace("\"", ""),
		deserialized_data["eventPayload"]["pull_request"]["head"]["sha"].to_string().replace("\"", ""),
		pr_id.clone(),
		repo_name.clone(),
		repo_owner.clone(),
		repo_provider.clone(),
		format!("github/{}/{}/{}", &repo_owner, &repo_name, &pr_id),
		clone_dir,
		clone_url,
		deserialized_data["eventPayload"]["pull_request"]["user"]["id"].to_string().replace("\"", ""),
		None,
	);
	log::debug!("[create_and_save_github_review_object] github review object = {:?}", &review);
	save_review_to_db(&review);
	return Some(review);
}