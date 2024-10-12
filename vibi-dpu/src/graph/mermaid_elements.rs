
use crate::{graph::{elements::MermaidGraphElements, graph_edges::graph_edges, graph_info::generate_diff_graph}, utils::{gitops::{git_checkout_commit, StatItem}, review::Review}};

use super::{function_call::FunctionCallChunk, function_line_range::{AllFileFunctions, FuncDefInfo, FunctionFileMap}, graph_info::{DiffFuncCall, DiffFuncDefs, DiffGraph, FuncCall}, utils::all_code_files};


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
    // generate full graph for base commit id
    git_checkout_commit(review, review.base_head_commit());
    let base_filepaths_opt = all_code_files(review.clone_dir(), diff_files);
    if base_filepaths_opt.is_none() {
        log::error!(
            "[generate_flowchart_elements] Unable to get file paths: {}", review.clone_dir());
        return None;
    }
    let base_filepaths = base_filepaths_opt.expect("Empty base_filepaths_opt");
    // let base_commit_import_info = get_test_import_info();
    let diff_graph_opt = generate_diff_graph(diff_files, review).await;
    log::debug!("[generate_flowchart_elements] diff_graph_opt = {:#?}", &diff_graph_opt);
    if diff_graph_opt.is_none() {
        log::error!(
            "[generate_flowchart_elements] Unable to generate diff graph for review: {}",
            review.id());
        return None;
    }
    let diff_graph = diff_graph_opt.expect("Empty diff_graph_opt");
    // let diff_graph = get_test_diff_graph();
    // let diff_info = generate_diff_info(&full_graph, &diff_graph); 
    // git_checkout_commit(review, review.pr_head_commit());
    // let head_filepaths_opt = all_code_files(review.clone_dir());
    // if head_filepaths_opt.is_none() {
    //     log::error!(
    //         "[generate_flowchart_elements] Unable to get file paths: {}", review.clone_dir());
    //     return None;
    // }
    // let head_filepaths = head_filepaths_opt.expect("Empty head_filepaths_opt");
    let mut graph_elems = MermaidGraphElements::new();
    graph_edges(&base_filepaths, review, &diff_graph, &mut graph_elems).await;
    let elems_str = graph_elems.render_elements(review);
    return Some(elems_str);
}