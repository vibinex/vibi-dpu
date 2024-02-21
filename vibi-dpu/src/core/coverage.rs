use std::collections::{HashMap, HashSet};

use crate::{bitbucket::{self, user::author_from_commit}, core::github, db::review::save_review_to_db, utils::{aliases::get_login_handles, hunk::{HunkMap, PrHunkItem}, user::ProviderEnum}};
use crate::utils::review::Review;
use crate::utils::repo_config::RepoConfig;

pub async fn process_coverage(hunkmap: &HunkMap, review: &Review, repo_config: &mut RepoConfig, access_token: &str) {
    for prhunk in hunkmap.prhunkvec() {
        // calculate number of hunks for each userid
        let mut review_mut = review.clone();
        let coverage_map = calculate_coverage(
            prhunk, &mut review_mut).await;
        let coverage_cond = !coverage_map.is_empty();
        log::debug!("[process_coverage] !coverage_map.is_empty() = {:?}", &coverage_cond);
        log::debug!("[process_coverage] repo_config.comment() = {:?}", repo_config.comment());
        log::debug!("[process_coverage] repo_config.auto_assign() = {:?}", repo_config.auto_assign());
        if coverage_map.is_empty() {
            continue;
        }
        if repo_config.comment() {
            log::info!("[process_coverage] Inserting comment...");
            // create comment text
            let comment = comment_text(&coverage_map, repo_config.auto_assign());
            // add comment
            if review.provider().to_string() == ProviderEnum::Bitbucket.to_string() {
                bitbucket::comment::add_comment(&comment, review, &access_token).await;
            }
            if review.provider().to_string() == ProviderEnum::Github.to_string() {
                github::comment::add_comment(&comment, review, &access_token).await;
            }
            
        }
        if repo_config.auto_assign() {
            log::info!("[process_coverage] Auto assigning reviewers...");
            log::debug!("[process_coverage] review.provider() = {:?}", review.provider());
            if review.provider().to_string() == ProviderEnum::Bitbucket.to_string() {
                add_bitbucket_reviewers(&prhunk, hunkmap, review, &access_token).await;
            }
            if review.provider().to_string() == ProviderEnum::Github.to_string() {
                add_github_reviewers(review, &coverage_map, &access_token).await;
            }
        }  
    }
}

async fn add_github_reviewers(review: &Review, coverage_map: &HashMap<String, (String, Option<Vec<String>>)>, access_token: &str) {
    let mut reviewers: HashSet<String> = HashSet::new();
    for (_, (_, provider_ids_opt)) in coverage_map.iter() {
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
            log::error!("[process_coverage] Unable to get blame author from bb for commit: {}", &blame.commit());
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

async fn calculate_coverage(prhunk: &PrHunkItem, review: &mut Review) -> HashMap<String, (String, Option<Vec<String>>)>{
    let mut coverage_floatmap = HashMap::<String, f32>::new();
    let mut total = 0.0;
    for blame in prhunk.blamevec() {
        let author_id = blame.author().to_owned();
        let num_lines: f32 = blame.line_end().parse::<f32>().expect("lines_end invalid float")
            - blame.line_start().parse::<f32>().expect("lines_end invalid float")
            + 1.0;
        total += num_lines;
        if coverage_floatmap.contains_key(&author_id) {
            let coverage = coverage_floatmap.get(&author_id).expect("unable to find coverage for author")
                + num_lines;
            coverage_floatmap.insert(author_id, coverage);
        }
        else {
            coverage_floatmap.insert(author_id, num_lines);
        }
    }
    let mut coverage_map = HashMap::<String, (String, Option<Vec<String>>)>::new();
    if total <= 0.0 {
        return coverage_map;
    } 
    for (blame_author, coverage) in coverage_floatmap.iter_mut() {
        *coverage = *coverage / total * 100.0;
        let formatted_value = format!("{:.2}", *coverage);
        let provider_id = get_login_handles(blame_author, review).await;
        coverage_map.insert(blame_author.to_string(), (formatted_value, provider_id));
    }
    review.set_coverage(Some(coverage_map.clone()));
    save_review_to_db(review);
    return coverage_map;
}

fn comment_text(coverage_map: &HashMap<String, (String, Option<Vec<String>>)>, auto_assign: bool) -> String {
    let mut comment = "Relevant users for this PR:\n\n".to_string();  // Added two newlines
    comment += "| Contributor Name/Alias  | Code Coverage |\n";  // Added a newline at the end
    comment += "| -------------- | --------------- |\n";  // Added a newline at the end
    let mut unmapped_aliases = Vec::new();
    for (git_alias, (coverage_val, provider_ids_opt)) in coverage_map.iter() {
        if provider_ids_opt.is_some() {
            let provider_ids = provider_ids_opt.to_owned().expect("Empty provider_ids_opt");
            let provider_id_opt = provider_ids.first();
            if provider_id_opt.is_some() {
                let provider_id = provider_id_opt.expect("Empty provider_id_opt");
                comment += &format!("| @{} | {}% |\n", provider_id, coverage_val);
                continue;
            }
        }
        comment += &format!("| {} | {}% |\n", git_alias, coverage_val);  // Added a newline at the end
        unmapped_aliases.push(git_alias);
    }

    if !unmapped_aliases.is_empty() {
        comment += "\n\n";
        comment += &format!("Missing profile handles for {} aliases. [Log in to Vibinex](https://vibinex.com) to map aliases to profile handles.", unmapped_aliases.len());
    }

    if auto_assign {
        comment += "\n\n";
        comment += "Auto assigning to relevant reviewers.";
    }
    comment += "\n\n";
    comment += "Code coverage is calculated based on the git blame information of the PR. To know more, hit us up at contact@vibinex.com.\n\n";  // Added two newlines
    comment += "To change comment and auto-assign settings, go to [your Vibinex settings page.](https://vibinex.com/settings)\n";  // Added a newline at the end

    return comment;
}
