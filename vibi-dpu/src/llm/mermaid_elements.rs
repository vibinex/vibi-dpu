use std::collections::HashMap;

use crate::utils::{gitops::StatItem, review::Review};

use super::{elements::{MermaidEdge, MermaidEdges, MermaidNode, MermaidSubgraph}, function_info::{extract_function_calls, extract_function_import_path, extract_function_lines, CalledFunction, CalledFunctionPath, FunctionLineMap}, gitops::get_changed_files, utils::read_file};

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
    let mut subgraph_map = HashMap::<String, MermaidSubgraph>::new();
    let mut edges = MermaidEdges::new(Vec::<MermaidEdge>::new());
    let files: Vec<String> = small_files.iter().map(|item| item.filepath.clone()).collect();
    for file in files {
        generate_mermaid_content(
            &mut subgraph_map,
            review,
            &file,
            &file_lines_del_map,
            &file_lines_add_map,
            &mut edges,
        ).await;
    }
    // Render content string
    let subgraphs_str = subgraph_map.values().map(
        |subgraph| subgraph.render_subgraph()
    ).collect::<Vec<String>>().join("\n");
    let edges_str = edges.render_edges();
    let content_str = format!("{}\n{}", &subgraphs_str, &edges_str);
    return Some(content_str);
}

async fn generate_mermaid_content(
    subgraph_map: &mut HashMap<String,MermaidSubgraph>, review: &Review, file: &str,
    file_lines_del_map: &HashMap<String, Vec<(usize, usize)>>,
    file_lines_add_map: &HashMap<String, Vec<(usize, usize)>>,
    edges: &mut MermaidEdges
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
    let file_subgraph = MermaidSubgraph::new(
        file.to_string(), HashMap::<String,MermaidNode>::new());
    generate_caller_elements(
        subgraph_map,
        &file_lines_del_map[file],
        &flinemap,
        &called_funcs_del,
        &called_func_paths_del,
        &file_subgraph,
        edges,
        "red");
    // added lines
    let called_info_del_opt = generate_called_function_info(
        file_lines_add_map, &numbered_content, file).await;
    if called_info_del_opt.is_none() {
        log::error!("[generate_mermaid_content] Unable to generate called functions info");
        return;
    }
    let (called_funcs_add, called_func_paths_add) = called_info_del_opt.expect("Empty called_info_opt");
    generate_callee_nodes(&called_func_paths_add, subgraph_map);
    generate_caller_elements(
        subgraph_map,
        &file_lines_del_map[file],
        &flinemap,
        &called_funcs_add,
        &called_func_paths_add,
        &file_subgraph,
        edges,
        "green");
    subgraph_map.insert(file.to_string(), file_subgraph);
    return;
}

fn generate_caller_elements(subgraph_map: &HashMap<String,MermaidSubgraph>,
    hunk_lines: &Vec<(usize, usize)>,
    flinemap: &Vec<FunctionLineMap>,
    called_funcs: &Vec<CalledFunction>,
    called_funcs_path: &Vec<CalledFunctionPath>,
    file_subgraph: &MermaidSubgraph,
    edges: &mut MermaidEdges, 
    color: &str) 
{
    for cf in called_funcs {
        let func_name_opt = get_func_from_line(cf.line, flinemap);
        if func_name_opt.is_none() {
            log::debug!("[generate_caller_elements] Unable to get func name for line: {:?}", cf.line);
            continue;
        }
        let func_name = func_name_opt.expect("Empty func_name_opt");
        let caller_node = match file_subgraph.nodes().get(&func_name) {
            Some(node) => node.to_owned(),
            None => MermaidNode::new(func_name.clone())
        };
        for cfp in called_funcs_path {
            if cf.name == cfp.function_name {
                edges.add_edge(MermaidEdge::new(
                    cf.line,
                    caller_node.to_owned(),
                    subgraph_map[&cfp.path].nodes()[&cf.name].to_owned(),
                    color.to_string()
                ));
            }
        }        
    }
}

fn get_func_from_line(line: usize, flinemaps: &[FunctionLineMap]) -> Option<String> {
    for flinemap in flinemaps {
        if flinemap.line_start >= line as i32 && flinemap.line_end <= line as i32 {
            return Some(flinemap.name.to_string());
        }
    }
    return None;
}

fn generate_callee_nodes(
    called_funcs_path: &[CalledFunctionPath],
    subgraph_map: &mut HashMap<String, MermaidSubgraph>) 
{
    for cfp in called_funcs_path {
        if let Some(subgraph) = subgraph_map.get_mut(&cfp.path) {
            subgraph.add_node(
                MermaidNode::new(cfp.function_name.to_string())
            );          
        } else {
            // Create new subgraph
            // Create new node
            // Add to subgraph_map
            let mut node_map = HashMap::<String, MermaidNode>::new();
            node_map.insert(cfp.function_name.to_string(), MermaidNode::new(cfp.function_name.to_string()));
            let subgraph = MermaidSubgraph::new(
                cfp.path.to_string(),
                node_map 
            );
            subgraph_map.insert(cfp.path.to_string(), subgraph);
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