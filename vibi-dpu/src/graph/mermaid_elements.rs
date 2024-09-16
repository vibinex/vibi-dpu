
use crate::{graph::{elements::MermaidGraphElements, graph_edges::graph_edges, graph_info::generate_diff_graph}, utils::{gitops::{git_checkout_commit, StatItem}, review::Review}};

use super::{file_imports::get_import_lines, utils::all_code_files};


pub async fn generate_mermaid_flowchart(diff_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let flowchart_content_res = generate_flowchart_elements(diff_files, review).await;
    if flowchart_content_res.is_none() {
        log::error!("[generate_mermaid_flowchart] Unable to generate flowchart content, review: {}", review.id());
        return None;
    }
    let flowchart_content = flowchart_content_res.expect("Empty flowchart_content_res");
    let flowchart_str = format!(
        "%%{{init: {{ \
            'theme': 'neutral', \
            'themeVariables': {{ \
                'fontSize': '20px' \
            }}, \
            'flowchart': {{ \
                'nodeSpacing': 100, \
                'rankSpacing': 100 \
            }} \
        }} }}%%\n{}",
        &flowchart_content
    );
    return Some(flowchart_str);
}

async fn generate_flowchart_elements(diff_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    // generate full graph for base commit id
    git_checkout_commit(review, review.base_head_commit());
    let repo_code_files_opt = all_code_files(review.clone_dir());
    if repo_code_files_opt.is_none() {
        log::error!(
            "[generate_flowchart_elements] Unable to get file paths: {}", review.clone_dir());
        return None;
    }
    let repo_code_files = repo_code_files_opt.expect("Empty repo_code_files_opt");
    let base_commit_import_info_opt = get_import_lines(&repo_code_files).await;
    log::debug!("[generate_flowchart_elements] ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ all_file_import_info_opt = {:#?}", &base_commit_import_info_opt);
    if base_commit_import_info_opt.is_none() {
        log::error!("[generate_flowchart_elements] Unable to get import info for source files: {:#?}", &repo_code_files);
        return None;
    }
    let base_commit_import_info = base_commit_import_info_opt.expect("Empty import_lines_opt");
    git_checkout_commit(review, review.pr_head_commit());
    let diff_graph_opt = generate_diff_graph(diff_files, review, &base_commit_import_info).await;
    log::debug!("[generate_flowchart_elements] diff_graph_opt = {:#?}", &diff_graph_opt);
    if diff_graph_opt.is_none() {
        log::error!(
            "[generate_flowchart_elements] Unable to generate diff graph for review: {}",
            review.id());
        return None;
    }
    let diff_graph = diff_graph_opt.expect("Empty diff_graph_opt");
    // let diff_info = generate_diff_info(&full_graph, &diff_graph); 
    let mut graph_elems = MermaidGraphElements::new();
    git_checkout_commit(review, review.base_head_commit());
    graph_edges(review, &base_commit_import_info, &diff_graph, &mut graph_elems).await;
    let elems_str = graph_elems.render_elements(review);
    return Some(elems_str);
}