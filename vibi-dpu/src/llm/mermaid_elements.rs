use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::{gitops::StatItem, review::Review};

use super::{function_info::{extract_function_calls, extract_function_import_path, extract_function_lines, CalledFunction, CalledFunctionPath, FunctionLineMap}, gitops::get_changed_files, utils::read_file};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct MermaidSubgraph {
    subgraph_str: Option<String>,
    nodes: HashMap<String, MermaidNode>
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct MermaidNode {
    node_str: Option<String>,
    function_name: String
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct MermaidEdge {
    edge_str: Option<String>,
    caller_function: String,
    called_function: String,
    color: String
}

pub async fn generate_mermaid_flowchart(small_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let flowchart_content_res = generate_flowchart_elements(small_files, review).await;
    if flowchart_content_res.is_none() {
        log::error!("[generate_mermaid_flowchart] Unable to generate flowchart content, review: {}", review.id());
        return None;
    }
    let flowchart_content = flowchart_content_res.expect("Empty flowchart_content_res");
    let flowchart_str = format!(
        "%%{{init: {{\"flowchart\": {{\"htmlLabels\": false}}}} }}%%\nflowchart LR{}\n",
        &flowchart_content
    );
    return Some(flowchart_str);
}

async fn generate_flowchart_elements(small_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let (file_lines_del_map, file_lines_add_map) = get_changed_files(small_files, review);
    let subgraph_map = HashMap::<String, MermaidSubgraph>::new();
    let mut edges_vec = Vec::<MermaidEdge>::new();
    let files: Vec<String> = small_files.iter().map(|item| item.filepath.clone()).collect();
    for file in files {
        generate_mermaid_content(
            &subgraph_map,
            review,
            &file,
            &file_lines_del_map,
            &file_lines_add_map,
            &mut edges_vec,
        ).await;
    }
    // Render content string
    return None;
}

async fn generate_mermaid_content(
    subgraph_map: &HashMap<String,MermaidSubgraph>, review: &Review, file: &str,
    file_lines_del_map: &HashMap<String, Vec<(usize, usize)>>,
    file_lines_add_map: &HashMap<String, Vec<(usize, usize)>>,
    edges_vec: &mut Vec<MermaidEdge>
) {
    if !file.ends_with(".rs") {
        log::debug!("[mermaid_comment] File extension not valid: {}", &file);
        return;
    }
    let file_path = format!("{}/{}", review.clone_dir(), &file);
    let file_contents_res = read_file(&file_path);
    if file_contents_res.is_none() {
        log::error!(
            "[generate_mermaid_content] Unable to read changed file content: {}", &file_path);
        return;
    }
    let file_contents = file_contents_res.expect("Empty file_contents_res");
    let numbered_content = file_contents
        .lines()
        .enumerate()
        .map(|(index, line)| format!("{} {}", index + 1, line))
        .collect::<Vec<String>>()
        .join("\n");
    let flinemap_opt = extract_function_lines(
        &numbered_content,
        file
    ).await;
    if flinemap_opt.is_none() {
        log::debug!(
            "[generate_mermaid_content] Unable to generate function line map for file: {}", file);
        return;
    }
    let flinemap = flinemap_opt.expect("Empty flinemap_opt");
    // deleted lines
    let called_info_del_opt = generate_called_function_info(
        file_lines_del_map, &numbered_content, file).await;
    if called_info_del_opt.is_none() {
        log::error!("[generate_mermaid_content] Unable to generate called functions info");
        return;
    }
    let (called_funcs_del, called_func_paths_del) = called_info_del_opt.expect("Empty called_info_opt");
    generate_callee_nodes(&called_func_paths_del, subgraph_map);
    generate_caller_elements(file, &file_lines_del_map[file], &flinemap, &called_funcs_del, edges_vec, "red");
    // added lines
    let called_info_del_opt = generate_called_function_info(
        file_lines_add_map, &numbered_content, file).await;
    if called_info_del_opt.is_none() {
        log::error!("[generate_mermaid_content] Unable to generate called functions info");
        return;
    }
    let (called_funcs_add, called_func_paths_add) = called_info_del_opt.expect("Empty called_info_opt");
    generate_callee_nodes(&called_func_paths_add, subgraph_map);
    generate_caller_elements(file, &file_lines_del_map[file], &flinemap, &called_funcs_del, edges_vec, "green");
    return;
}

fn generate_caller_elements(filename: &str,
    hunk_lines: &Vec<(usize, usize)>,
    flinemap: &Vec<FunctionLineMap>,
    called_funcs_del: &Vec<CalledFunction>, edges_vec: &mut Vec<MermaidEdge>, color: &str
    ) 
{
    let mut relevant_funcs = Vec::<String>::new();
    for cf in called_funcs_del {
        let func_name_opt = get_func_from_line(hunk_lines, cf.line, flinemap);
        if func_name_opt.is_none() {
            log::debug!("[generate_caller_elements] Unable to get func name for line: {:?}", cf.line);
            continue;
        }
        let func_name = func_name_opt.expect("Empty func_name_opt");
        relevant_funcs.push(func_name.clone());
        edges_vec.push(MermaidEdge{ 
            edge_str: None,
            caller_function: func_name,
            called_function: cf.name.to_string(),
            color: color.to_string()
        })
    }
    for rf in relevant_funcs {
        // Add mermaid node for func in correct mermaid subgraph
    }
}

fn get_func_from_line(hunk_lines: &[(usize, usize)], line: usize, flinemaps: &[FunctionLineMap]) -> Option<String> {
    for flinemap in flinemaps {
        if flinemap.line_start >= line as i32 && flinemap.line_end <= line as i32 {
            return Some(flinemap.name.to_string());
        }
    }
    return None;
}

fn generate_callee_nodes(
    called_funcs_path: &[CalledFunctionPath],
    subgraph_map: &HashMap<String, MermaidSubgraph>) 
{
    for cfp in called_funcs_path {
        if let Some(subgraph) = subgraph_map.to_owned().get_mut(&cfp.path) {
            subgraph.nodes.insert(
                cfp.function_name.to_string(),
                MermaidNode { node_str: None, function_name: cfp.function_name.to_string()}
            );          
        } else {
            // Create new subgraph
            // Create new node
            // Add to subgraph_map
        }
    } 
    return;
}

async fn generate_called_function_info(file_lines_map: &HashMap<String, Vec<(usize, usize)>>,
    numbered_content: &str, filename: &str
)
    -> Option<(Vec<CalledFunction>, Vec<CalledFunctionPath>)>
{
    let del_lines = &file_lines_map[filename];
    let called_funcs_opt = extract_function_calls(
        del_lines,
        &numbered_content,
        filename
    ).await;
    if called_funcs_opt.is_none() {
        log::error!("[generate_called_function_info] Unable to get called functions for file: {}", filename);
        return None;
    }
    let called_funcs = called_funcs_opt.expect("Empty called_funcs_opt");
    let called_func_paths_opt = extract_function_import_path(
        &called_funcs,
        &numbered_content,
        filename
    ).await;
    if called_func_paths_opt.is_none() {
        log::error!("[generate_called_function_info] Unable to get called functions for file: {}", filename);
        return None;
    }
    let called_func_paths = called_func_paths_opt.expect("Empty called_func_paths_opt");
    return Some((called_funcs, called_func_paths));
}