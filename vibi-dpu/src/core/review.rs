use std::env;

use serde_json::Value;

use crate::{
	utils::{hunk::{HunkMap, PrHunkItem}, 
			review::Review,
			repo_config::RepoConfig, 
			gitops::{commit_exists, 
					git_pull, 
					get_excluded_files, 
					generate_diff, 
					process_diffmap, 
					generate_blame}}, 
	db::{hunk::{get_hunk_from_db, store_hunkmap_to_db}, 
		repo::get_clone_url_clone_dir, 
		review::save_review_to_db,
		repo_config::save_repo_config_to_db},
	bitbucket::config::get_client,
	core::coverage::process_coverage};

pub async fn process_review(message_data: &Vec<u8>) {
	let review_opt = parse_review(message_data);
	if review_opt.is_none() {
		eprintln!("Unable to deserialize review message and repo config");
		return;
	}
	let (review, repo_config) = review_opt.expect("parse_opt is empty");
	println!("deserialized repo_config, review = {:?}, {:?}", &repo_config, &review);
	if hunk_already_exists(&review) {
		return;
	}
	println!("Processing PR : {}", &review.id());
	commit_check(&review).await;
	let hunkmap_opt = process_review_changes(&review).await;
	send_hunkmap(&hunkmap_opt, &review, &repo_config).await;
}

async fn send_hunkmap(hunkmap_opt: &Option<HunkMap>, review: &Review, repo_config: &RepoConfig) {
	if hunkmap_opt.is_none() {
		eprintln!("Empty hunkmap in send_hunkmap");
		return;
	}
	let hunkmap = hunkmap_opt.to_owned().expect("empty hunkmap_opt");
	println!("HunkMap = {:?}", &hunkmap);
	store_hunkmap_to_db(&hunkmap, review);
	publish_hunkmap(&hunkmap);
	let hunkmap_async = hunkmap.clone();
	let review_async = review.clone();
	let repo_config_clone = repo_config.clone();
	process_coverage(&hunkmap_async, &review_async, &repo_config_clone).await;
}

fn hunk_already_exists(review: &Review) -> bool {
	let hunk_opt = get_hunk_from_db(&review);
	if hunk_opt.is_none() {
		eprintln!("No hunk from get_hunk_from_db");
		return false;
	}
	let hunkmap = hunk_opt.expect("empty hunk from get_hunk_from_db");
	publish_hunkmap(&hunkmap);
	println!("Hunk already in db!");
	return true;
}
async fn process_review_changes(review: &Review) -> Option<HunkMap>{
	let mut prvec = Vec::<PrHunkItem>::new();
	let fileopt = get_excluded_files(&review);
	println!("fileopt = {:?}", &fileopt);
	if fileopt.is_none() {
		eprintln!("No files to review for PR {}", review.id());
		return None;
	}
	let (_, smallfiles) = fileopt.expect("fileopt is empty");
	let diffmap = generate_diff(&review, &smallfiles);
	println!("diffmap = {:?}", &diffmap);
	let linemap = process_diffmap(&diffmap);
	let blamevec = generate_blame(&review, &linemap).await;
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
	return Some(hunkmap);
}

async fn commit_check(review: &Review) {
	if !commit_exists(&review.base_head_commit(), &review.clone_dir()) 
		|| !commit_exists(&review.pr_head_commit(), &review.clone_dir()) {
		println!("Pulling repository {} for commit history", &review.repo_name());
		git_pull(review).await;
	}
}

fn parse_review(message_data: &Vec<u8>) -> Option<(Review, RepoConfig)>{
	let data_res = serde_json::from_slice::<Value>(&message_data);
	if data_res.is_err() {
		let e = data_res.expect_err("No error in data_res");
		eprintln!("Incoming message does not contain valid reviews: {:?}", e);
		return None;
	}
	let deserialized_data = data_res.expect("Uncaught error in deserializing message_data");
	println!("deserialized_data == {:?}", &deserialized_data["eventPayload"]["repository"]);
	let repo_provider = deserialized_data["repositoryProvider"].to_string().trim_matches('"').to_string();
	let repo_name = deserialized_data["eventPayload"]["repository"]["name"].to_string().trim_matches('"').to_string();
	println!("repo NAME == {}", &repo_name);
	let workspace_name = deserialized_data["eventPayload"]["repository"]["workspace"]["slug"].to_string().trim_matches('"').to_string();
	let clone_opt = get_clone_url_clone_dir(&repo_provider, &workspace_name, &repo_name);
	if clone_opt.is_none() {
		eprintln!("Unable to get clone url and directory");
		return None;
	}
	let (clone_url, clone_dir) = clone_opt.expect("Empty clone_opt");
	let pr_id = deserialized_data["eventPayload"]["pullrequest"]["id"].to_string().trim_matches('"').to_string();
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
	);
	println!("review = {:?}", &review);
	save_review_to_db(&review);
	let repo_config_res = serde_json::from_value(deserialized_data["repoConfig"].clone());
	if repo_config_res.is_err() {
		let e = repo_config_res.expect_err("No error in repo_config_res");
		eprintln!("Unable to deserialze repo_config_res: {:?}", e);
		let default_config = RepoConfig::default();
		return Some((review, default_config));
	}
	let repo_config = repo_config_res.expect("Uncaught error in repo_config_res");
	println!("repo_config = {:?}", &repo_config);
	save_repo_config_to_db(&repo_config, &review.repo_name(), &review.repo_owner(), &review.provider());
	return Some((review, repo_config));
}

fn publish_hunkmap(hunkmap: &HunkMap) {
	let client = get_client();
	let hunkmap_json = serde_json::to_string(&hunkmap).expect("Unable to serialize hunkmap");
	let key_clone = hunkmap.db_key().to_string();
	tokio::spawn(async move {
		let url = format!("{}/api/hunks",
			env::var("SERVER_URL").expect("SERVER_URL must be set"));
		println!("url for hunkmap publishing  {}", &url);
		match client
		.post(url)
		.json(&hunkmap_json)
		.send()
		.await {
			Ok(_) => {
				println!("[publish_hunkmap] Hunkmap published successfully for: {} !", &key_clone);
			},
			Err(e) => {
				eprintln!("[publish_hunkmap] Failed to publish hunkmap: {} for: {}", e, &key_clone);
			}
		};
	});
}
