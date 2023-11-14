use std::collections::{HashMap, HashSet};

use crate::{utils::{hunk::{HunkMap, PrHunkItem}, user::{BitbucketUser, WorkspaceUser, Provider, ProviderEnum}}, db::user::{get_workspace_user_from_db}, bitbucket::{user::author_from_commit, reviewer::add_reviewers, self}, core::github};
use crate::utils::review::Review;
use crate::utils::repo_config::RepoConfig;
use crate::bitbucket::auth::get_access_token_review;

pub async fn process_coverage(hunkmap: &HunkMap, review: &Review, repo_config: &mut RepoConfig) {
    let access_token_opt = get_access_token_review(review).await;
    if access_token_opt.is_none() {
        eprintln!("Unable to acquire access_token in process_coverage");
        return;
    }
    let access_token = access_token_opt.expect("Empty access_token_opt");
    for prhunk in hunkmap.prhunkvec() {
        // calculate number of hunks for each userid
        let coverage_map = calculate_coverage(&hunkmap.repo_owner(),
            prhunk, &review.provider());
        let coverage_cond = !coverage_map.is_empty();
        println!("!coverage_map.is_empty() = {:?}", &coverage_cond);
        println!("repo_config.comment() = {:?}", repo_config.comment());
        println!("repo_config.auto_assign() = {:?}", repo_config.auto_assign());
        if coverage_map.is_empty() {
            continue;
        }
        if review.provider().to_string() == ProviderEnum::Github.to_string() {
            repo_config.set_auto_assign(false);
        }
        if repo_config.comment() {
            println!("Inserting comment...");
            // create comment text
            let comment = comment_text(coverage_map, repo_config.auto_assign());
            // add comment
            if review.provider().to_string() == ProviderEnum::Bitbucket.to_string() {
                bitbucket::comment::add_comment(&comment, review, &access_token).await;
            }
            if review.provider().to_string() == ProviderEnum::Github.to_string() {
                github::comment::add_comment(&comment, review, &access_token).await;
            }
            
        }
        if repo_config.auto_assign() {
            let mut author_set: HashSet<String> = HashSet::new();
            author_set.insert(prhunk.author().to_string());
            for blame in prhunk.blamevec() {
                let blame_author_opt = author_from_commit(blame.commit(),
                    hunkmap.repo_name(), hunkmap.repo_owner()).await;
                if blame_author_opt.is_none() {
                    eprintln!("[process_coverage] Unable to get blame author from bb for commit: {}", &blame.commit());
                    continue;
                }
                let blame_author = blame_author_opt.expect("Empty blame_author_opt");
                let author_uuid = blame_author.uuid();
                if author_set.contains(author_uuid) {
                    continue;
                }
                add_reviewers(&blame_author, review, &access_token).await;
                author_set.insert(author_uuid.to_string());
            }
        }  
    }
}

fn calculate_coverage(repo_owner: &str, prhunk: &PrHunkItem, repo_provider: &str) -> HashMap<String, String>{
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
    let mut coverage_map = HashMap::<String, String>::new();
    if total <= 0.0 {
        return coverage_map;
    } 
    for (blame_author, coverage) in coverage_floatmap.iter_mut() {
        *coverage = *coverage / total * 100.0;
        let formatted_value = format!("{:.2}", *coverage);
        let coverage_key: String;
        if repo_provider.to_string() == ProviderEnum::Bitbucket.to_string() {
            let user = get_workspace_user_from_db(blame_author);
            if user.is_none() {
                eprintln!("No user name found for {}", blame_author);
                coverage_map.insert(blame_author.to_string(), formatted_value);
                continue;
            }
            let user_val = user.expect("user is empty");
            coverage_key = user_val.display_name().to_owned();
        }
        else {
            coverage_key = blame_author.to_string(); // TODO - get github user id and username here
        }
        coverage_map.insert(coverage_key, formatted_value);
    }
    return coverage_map;
}

fn comment_text(coverage_map: HashMap<String, String>, auto_assign: bool) -> String {
    let mut comment = "Relevant users for this PR:\n\n".to_string();  // Added two newlines
    comment += "| Contributor Name/Alias  | Code Coverage |\n";  // Added a newline at the end
    comment += "| -------------- | --------------- |\n";  // Added a newline at the end

    for (key, value) in coverage_map.iter() {
        comment += &format!("| {} | {}% |\n", key, value);  // Added a newline at the end
    }
    if auto_assign {
        comment += "\n\n";
        comment += "Auto assigning to all relevant reviewers";
    }
    comment += "\n\n";
    comment += "Code coverage is calculated based on the git blame information of the PR. To know more, hit us up at contact@vibinex.com.\n\n";  // Added two newlines
    comment += "To change comment and auto-assign settings, go to [your Vibinex settings page.](https://vibinex.com/settings)\n";  // Added a newline at the end

    return comment;
}
