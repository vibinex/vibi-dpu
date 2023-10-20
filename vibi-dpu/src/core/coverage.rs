use std::collections::{HashMap, HashSet};

use crate::{utils::hunk::{HunkMap, PrHunkItem}, db::user::get_workspace_user_from_db, bitbucket::{comment::add_comment, reviewer::add_reviewers}};
use crate::utils::review::Review;
use crate::bitbucket::auth::get_access_token_review;

pub async fn process_coverage(hunkmap: &HunkMap, review: &Review) {
    let access_token_opt = get_access_token_review(review).await;
    if access_token_opt.is_none() {
        eprintln!("Unable to acquire access_token in process_coverage");
        return;
    }
    let access_token = access_token_opt.expect("Empty access_token_opt");
    for prhunk in hunkmap.prhunkvec() {
        // calculate number of hunks for each userid
        let coverage_map = calculate_coverage(&hunkmap.repo_owner(), prhunk);
        if !coverage_map.is_empty() {
            // get user for each user id
            // add reviewers
            let mut author_set: HashSet<String> = HashSet::new();
            author_set.insert(prhunk.author().to_string());
            for blame in prhunk.blamevec() {
                let blame_author_id = author_uuid_from_commit(blame.commit(), hunkmap.repo_name(), hunkmap.repo_owner());
                if author_set.contains(blame.author()) {
                    continue;
                }
                author_set.insert(blame.author().to_string());
                let author_id = blame.author();
                add_reviewers(blame.author(), review, &access_token).await;
            }
             // create comment text
            let comment = comment_text(coverage_map);
            // add comment
            add_comment(&comment, review, &access_token).await; 
            // TODO - implement settings
        }    
    }
}

fn calculate_coverage(repo_owner: &str, prhunk: &PrHunkItem) -> HashMap<String, String>{
    let mut coverage_map = HashMap::<String, String>::new();
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
    if total <= 0.0 {
        return coverage_map;
    } 
    for (key, value) in coverage_floatmap.iter_mut() {
        *value = *value / total * 100.0;
        let formatted_value = format!("{:.2}", *value);
        let user = get_workspace_user_from_db(key);
        if user.is_none() {
            eprintln!("No user name found for {}", key);
            coverage_map.insert(key.to_string(), formatted_value);
            continue;
        }
        let user_val = user.expect("user is empty");
        let coverage_key = user_val.display_name().to_owned();
        coverage_map.insert(coverage_key, formatted_value);
    }
    return coverage_map;
}

fn comment_text(coverage_map: HashMap<String, String>) -> String {
    let mut comment = "Relevant users for this PR:\n\n".to_string();  // Added two newlines
    comment += "| Contributor Name/Alias  | Code Coverage |\n";  // Added a newline at the end
    comment += "| -------------- | --------------- |\n";  // Added a newline at the end

    for (key, value) in coverage_map.iter() {
        comment += &format!("| {} | {}% |\n", key, value);  // Added a newline at the end
    }
    comment += "\n\n";
    comment += "Code coverage is calculated based on the git blame information of the PR. To know more, hit us up at contact@vibinex.com.\n\n";  // Added two newlines
    comment += "To change comment and auto-assign settings, go to [your Vibinex settings page.](https://vibinex.com/settings)\n";  // Added a newline at the end

    return comment;
}
