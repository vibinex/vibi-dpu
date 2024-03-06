use std::collections::{HashMap, HashSet};

use crate::{bitbucket::{self, user::author_from_commit}, core::github, db::review::save_review_to_db, utils::{aliases::get_login_handles, relevance::Relevance, hunk::{HunkMap, PrHunkItem}, user::ProviderEnum}};
use crate::utils::review::Review;
use crate::utils::repo_config::RepoConfig;

pub async fn process_relevance(hunkmap: &HunkMap, review: &Review, repo_config: &mut RepoConfig, access_token: &str) {
    for prhunk in hunkmap.prhunkvec() {
        // calculate number of hunks for each userid
        let mut review_mut = review.clone();
        let relevance_vec_opt = calculate_relevance(
            prhunk, &mut review_mut).await;
        if relevance_vec_opt.is_none() {
            log::debug!("[process_relevance] Unable to calculate coverage obj");
            continue;
        }
        let relevance_vec = relevance_vec_opt.expect("Empty coverage_obj_opt");
        if repo_config.comment() {
            log::info!("[process_relevance] Inserting comment...");
            // create comment text
            let comment = comment_text(&relevance_vec, repo_config.auto_assign());
            // add comment
            if review.provider().to_string() == ProviderEnum::Bitbucket.to_string() {
                bitbucket::comment::add_comment(&comment, review, &access_token).await;
            }
            if review.provider().to_string() == ProviderEnum::Github.to_string() {
                github::comment::add_comment(&comment, review, &access_token).await;
            }
            
        }
        if repo_config.auto_assign() {
            log::info!("[process_relevance] Auto assigning reviewers...");
            log::debug!("[process_relevance] review.provider() = {:?}", review.provider());
            if review.provider().to_string() == ProviderEnum::Bitbucket.to_string() {
                add_bitbucket_reviewers(&prhunk, hunkmap, review, &access_token).await;
            }
            if review.provider().to_string() == ProviderEnum::Github.to_string() {
                add_github_reviewers(review, &relevance_vec, &access_token).await;
            }
        }  
    }
}

async fn add_github_reviewers(review: &Review, relevance_vec: &Vec<Relevance>, access_token: &str) {
    let mut reviewers: HashSet<String> = HashSet::new();
    for relevance_obj in relevance_vec {
        let provider_ids_opt = relevance_obj.handles();
        if provider_ids_opt.is_none() {
            continue;
        }
        let provider_ids = provider_ids_opt.to_owned().expect("Empty provider_ids_opt");
        let provider_id_opt = provider_ids.first();
        if provider_id_opt.is_none() {
            continue;
        }
        let provider_id = provider_id_opt.expect("Empty provider_id_opt");
        reviewers.insert(provider_id.to_owned());
    }
    if reviewers.is_empty() {
        return;
    }
    let reviewers_vec: Vec<String> = reviewers.into_iter().collect();
    github::reviewer::add_reviewers(&reviewers_vec, review, access_token).await;
}

async fn add_bitbucket_reviewers(prhunk: &PrHunkItem, hunkmap: &HunkMap, review: &Review, access_token: &str) {
    let mut author_set: HashSet<String> = HashSet::new();
    author_set.insert(prhunk.author().to_string());
    for blame in prhunk.blamevec() {
        let blame_author_opt = author_from_commit(blame.commit(),
            hunkmap.repo_name(), hunkmap.repo_owner()).await;
        if blame_author_opt.is_none() {
            log::error!("[process_relevance] Unable to get blame author from bb for commit: {}", &blame.commit());
            continue;
        }
        let blame_author = blame_author_opt.expect("Empty blame_author_opt");
        let author_uuid = blame_author.uuid();
        if author_set.contains(author_uuid) {
            continue;
        }
        bitbucket::reviewer::add_reviewers(&blame_author, review, &access_token).await;
        author_set.insert(author_uuid.to_string());
    }
}

async fn calculate_relevance(prhunk: &PrHunkItem, review: &mut Review) -> Option<Vec<Relevance>>{
    let mut relevance_floatmap = HashMap::<String, f32>::new();
    let mut total = 0.0;
    for blame in prhunk.blamevec() {
        let author_id = blame.author().to_owned();
        let num_lines: f32 = blame.line_end().parse::<f32>().expect("lines_end invalid float")
            - blame.line_start().parse::<f32>().expect("lines_end invalid float")
            + 1.0;
        total += num_lines;
        if relevance_floatmap.contains_key(&author_id) {
            let relevance = relevance_floatmap.get(&author_id).expect("unable to find coverage for author")
                + num_lines;
            relevance_floatmap.insert(author_id, relevance);
        }
        else {
            relevance_floatmap.insert(author_id, num_lines);
        }
    }
    let mut relevance_vec = Vec::<Relevance>::new();
    if total <= 0.0 {
        return None;
    } 
    for (blame_author, relevance) in relevance_floatmap.iter_mut() {
        *relevance = *relevance / total * 100.0;
        let formatted_value = format!("{:.2}", *relevance);
        let provider_ids = get_login_handles(blame_author, review).await;
        let relevance_obj = Relevance::new(
            review.provider().to_owned(),
            blame_author.to_owned(), 
            formatted_value.to_owned(), 
            *relevance, 
            provider_ids);
        relevance_vec.push(relevance_obj);
    }
    review.set_relevance(Some(relevance_vec.clone()));
    save_review_to_db(review);
    return Some(relevance_vec);
}

fn comment_text(relevance_vec: &Vec<Relevance>, auto_assign: bool) -> String {
    let mut comment = "Relevant users for this PR:\n\n".to_string();  // Added two newlines
    comment += "| Contributor Name/Alias  | Relevance |\n";  // Added a newline at the end
    comment += "| -------------- | --------------- |\n";  // Added a newline at the end

    let (deduplicated_relevance_map, unmapped_aliases) = deduplicated_relevance_vec_for_comment(relevance_vec);
    let mut deduplicated_relevance_vec: Vec<(&Vec<String>, &f32)> = deduplicated_relevance_map.iter().collect();
    deduplicated_relevance_vec.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // I couldn't find a way to avoid unwrap here :(
    
    for (provider_ids, relevance) in &deduplicated_relevance_vec {
        let provider_id_opt = provider_ids.iter().next();
        if provider_id_opt.is_some() {
            let provider_id = provider_id_opt.expect("Empty provider_id_opt");
            log::info!("[comment-text] provider_id: {:?}", provider_id);
            let formatted_relevance_value = format!("{:.2}", *relevance);
            comment += &format!("| {} | {}% |\n", provider_id, formatted_relevance_value);
        }
    }

    if !&unmapped_aliases.is_empty() {
        comment += "\n\n";
        comment += &format!("Missing profile handles for {} aliases. [Go to your Vibinex settings page](https://vibinex.com/settings) to map aliases to profile handles.", unmapped_aliases.len());
    }

    if auto_assign {
        comment += "\n\n";
        comment += "Auto assigning to relevant reviewers.";
    }
    comment += "\n\n";
    comment += "If you are a relevant reviewer, you can use the [Vibinex browser extension](https://chromewebstore.google.com/detail/vibinex-code-review/jafgelpkkkopeaefadkdjcmnicgpcncc) to see parts of the PR relevant to you\n";  // Added a newline at the end
    comment += "Relevance of the reviewer is calculated based on the git blame information of the PR. To know more, hit us up at contact@vibinex.com.\n\n";  // Added two newlines
    comment += "To change comment and auto-assign settings, go to [your Vibinex settings page.](https://vibinex.com/u)\n";  // Added a newline at the end

    return comment;
}

pub fn deduplicated_relevance_vec_for_comment(relevance_vec: &Vec<Relevance>) -> (HashMap<Vec<String>, f32>, Vec<String>) {
    let mut combined_relevance_map: HashMap<Vec<String>, f32> = HashMap::new();
    let mut unmapped_aliases = Vec::new();

    // Iterate through relevance_vec and handle entries with provider IDs
    for relevance_obj in relevance_vec {
        let provider_ids_opt = relevance_obj.handles();
        if let Some(provider_ids) = provider_ids_opt {
            // Check if combined relevance for handles set already exists
            let mut found = false;
            for (existing_handles, relevance) in combined_relevance_map.iter_mut() {
                let intersection: HashSet<_> = existing_handles.iter().cloned().collect();
                if !intersection.is_empty() && provider_ids.iter().any(|h| intersection.contains(h)) {
                    *relevance += relevance_obj.relevance_num(); // Add relevance to existing combined relevance
                    found = true;
                    break;
                }
            }

            // If no combined relevance found, add a new entry
            if !found {
                combined_relevance_map.insert(provider_ids.clone(), relevance_obj.relevance_num());
            }
        } else {
            // For entries without provider IDs, add them to the combined_relevance map
            let git_alias = relevance_obj.git_alias();
            let git_alias_vec: Vec<String> = vec![git_alias.to_owned()];
            combined_relevance_map.insert(git_alias_vec, relevance_obj.relevance_num());
            // Add the git alias to the unmapped aliases array
            unmapped_aliases.push(git_alias.to_string());
        }
    }

    (combined_relevance_map, unmapped_aliases)
}
