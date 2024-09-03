use crate::graph::mermaid_elements::generate_mermaid_flowchart;
use crate::utils::user::ProviderEnum;
use crate::utils::review::Review;
use crate::core::github;
use crate::utils::gitops::StatItem;

pub async fn send_diff_graph(review: &Review, excluded_files: &Vec<StatItem>, small_files: &Vec<StatItem>, access_token: &str) {
	let comment = diff_graph_comment_text(excluded_files, small_files, review).await;
	// add comment for GitHub
	if review.provider().to_string() == ProviderEnum::Github.to_string() {
		log::info!("Inserting comment on repo {}...", review.repo_name());
		github::comment::add_comment(&comment, review, &access_token).await;
	}

	// TODO: add comment for Bitbucket
}

async fn diff_graph_comment_text(excluded_files: &Vec<StatItem>, small_files: &Vec<StatItem>, review: &Review) -> String {
    let mut comment = "Relevant users for this PR:\n\n".to_string();  
    
    let all_diff_files: Vec<StatItem> = excluded_files
        .iter()
        .chain(small_files.iter())
        .cloned()  // Clone the StatItem instances since `iter` returns references
        .collect(); // Collect into a new vector
    if let Some(mermaid_text) = mermaid_comment(&all_diff_files, review).await {
        comment += mermaid_text.as_str();
    }
    comment += "To modify DiffGraph settings, go to [your Vibinex settings page.](https://vibinex.com/settings)\n";
    return comment;
}

async fn mermaid_comment(diff_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let flowchart_str_opt = generate_mermaid_flowchart(diff_files, review).await;
    if flowchart_str_opt.is_none() {
        log::error!("[mermaid_comment] Unable to generate flowchart for review: {}", review.id());
        return None;
    }
    let flowchart_str = flowchart_str_opt.expect("Empty flowchart_str_opt");
    let mermaid_comment = format!(
        "### Call Stack Diff\n```mermaid\n{}\n```",
        flowchart_str,
    );
    return Some(mermaid_comment);
}

