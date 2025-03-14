
use crate::{graph::{elements::MermaidGraphElements, graph_edges::graph_edges, graph_info::generate_diff_graph}, utils::{gitops::{get_file_modification_status, git_checkout_commit, StatItem}, review::Review}};

use super::utils::all_code_files;


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
        }} }}%%\n \
        \tflowchart LR\n{}",
        &flowchart_content
    );
    return Some(flowchart_str);
}

async fn generate_flowchart_elements(diff_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    git_checkout_commit(review, review.base_head_commit());
    log::debug!("[generate_flowchart_elements] before review obj = {:#?}", review);
    let base_filepaths_opt = all_code_files(review.clone_dir(), diff_files);
    if base_filepaths_opt.is_none() {
        log::error!(
            "[generate_flowchart_elements] Unable to get file paths: {}", review.clone_dir());
        return None;
    }
    let base_filepaths = base_filepaths_opt.expect("Empty base_filepaths_opt");
    let diff_graph_opt = generate_diff_graph(review).await;
    log::debug!("[generate_flowchart_elements] diff_graph_opt = {:#?}", &diff_graph_opt);
    if diff_graph_opt.is_none() {
        log::error!(
            "[generate_flowchart_elements] Unable to generate diff graph for review: {}",
            review.id());
        return None;
    }
    let diff_graph = diff_graph_opt.expect("Empty diff_graph_opt");
    let mut graph_elems = MermaidGraphElements::new();
    log::debug!("[generate_flowchart_elements] review obj = {:#?}", review);
    graph_nodes(review, &mut graph_elems);
    graph_edges(&base_filepaths, review, &diff_graph, &mut graph_elems).await;
    let elems_str = graph_elems.render_elements(review);
    return Some(elems_str);
}

fn graph_nodes(review: &Review, graph_elems: &mut MermaidGraphElements) {
    if let Some(mod_map) = get_file_modification_status(review.clone_dir(), 
        &format!("{}...{}", review.base_head_commit(), review.pr_head_commit())) {
            log::debug!("[graph_nodes] mod map = {:#?}", &mod_map);
            if !mod_map.is_empty() {
                for (color_key, file_list) in mod_map {
                    for file_name in file_list {
                        graph_elems.add_file_node(&file_name, &color_key);
                    }
                }
            }
    }
}