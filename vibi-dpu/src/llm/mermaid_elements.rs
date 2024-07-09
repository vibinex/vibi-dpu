use std::{borrow::BorrowMut, collections::HashMap};

use crate::utils::{gitops::{git_checkout_commit, StatItem}, review::Review};

use super::{elements::{MermaidEdge, MermaidEdges, MermaidNode, MermaidSubgraph}, function_info::{extract_function_calls, extract_function_import_path, extract_function_lines, CalledFunction, CalledFunctionPath, FunctionLineMap}, gitops::get_changed_files, utils::read_file};

pub async fn generate_mermaid_flowchart(small_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let flowchart_content_res = generate_flowchart_elements(small_files, review).await;
    if flowchart_content_res.is_none() {
        log::error!("[generate_mermaid_flowchart] Unable to generate flowchart content, review: {}", review.id());
        return None;
    }
    let flowchart_content = flowchart_content_res.expect("Empty flowchart_content_res");
    let flowchart_str = format!(
        "%%{{init: {{\"flowchart\": {{\"htmlLabels\": false}}}} }}%%\nflowchart LR\n{}\n",
        &flowchart_content
    );
    return Some(flowchart_str);
}

async fn generate_flowchart_elements(small_files: &Vec<StatItem>, review: &Review) -> Option<String> {
    let (file_lines_del_map, file_lines_add_map) = get_changed_files(small_files, review);
    let mut subgraph_map = HashMap::<String, MermaidSubgraph>::new();
    let mut edges = MermaidEdges::new(Vec::<MermaidEdge>::new());
    let files: Vec<String> = small_files.iter().map(|item| item.filepath.clone()).collect();
    for file in files.iter() {
        if file_lines_add_map.contains_key(file) {
            generate_mermaid_content(
                &mut subgraph_map,
                review,
                file,
                &file_lines_add_map,
                &mut edges,
                "green"
            ).await;
        }
    }
    git_checkout_commit(review, review.base_head_commit());
    for file in files.iter() {
        if file_lines_del_map.contains_key(file) {
            generate_mermaid_content(
                &mut subgraph_map,
                review,
                file,
                &file_lines_del_map,
                &mut edges,
                "red"
            ).await;
        }
    }
    log::debug!("[generate_flowchart_elements] subgraph_map = {:#?}", &subgraph_map);
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
    file_lines_map: &HashMap<String, Vec<(usize, usize)>>,
    edges: &mut MermaidEdges,
    color: &str
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
        .map(|(index, line)| format!("{} {}", index, line))
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
        file_lines_map, &numbered_content, file).await;
    if called_info_del_opt.is_none() {
        log::error!("[generate_mermaid_content] Unable to generate called functions info");
        return;
    }
    let (called_funcs_del, called_func_paths_del) = called_info_del_opt.expect("Empty called_info_opt");
    generate_callee_nodes(&called_func_paths_del, subgraph_map);
    generate_caller_elements(
        subgraph_map,
        &file_lines_map[file],
        &flinemap,
        &called_funcs_del,
        &called_func_paths_del,
        edges,
        &file,
        color);
    return;
}

fn generate_caller_elements(subgraph_map: &mut HashMap<String, MermaidSubgraph>,
    hunk_lines: &Vec<(usize, usize)>,
    flinemap: &Vec<FunctionLineMap>,
    called_funcs: &Vec<CalledFunction>,
    called_funcs_path: &Vec<CalledFunctionPath>,
    edges: &mut MermaidEdges,
    filename: &str,
    color: &str)
{
    for cf in called_funcs {
        let func_name_opt = get_func_from_line(cf.line, flinemap);
        if func_name_opt.is_none() {
            log::debug!("[generate_caller_elements] Unable to get func name for line: {:?}", cf.line);
            continue;
        }
        let func_name = func_name_opt.expect("Empty func_name_opt");
        let caller_node;
        
        // Borrow subgraph_map mutably to either retrieve or insert the subgraph
        let subgraph = subgraph_map.entry(filename.to_string()).or_insert_with(|| {
            MermaidSubgraph::new(filename.to_string(), HashMap::new())
        });
        
        // Borrow subgraph mutably to either retrieve or insert the node
        if let Some(node) = subgraph.nodes().get(&func_name) {
            caller_node = node.to_owned();
        } else {
            caller_node = MermaidNode::new(func_name.clone());
            subgraph.add_node(caller_node.clone());
        }

        log::debug!("[generate_caller_elements] subgraph_map = {:#?}", subgraph_map);
        
        for cfp in called_funcs_path {
            if cf.name == cfp.function_name {
                // Ensure we do not have an immutable borrow of subgraph_map while we borrow it immutably here
                if let Some(import_subgraph) = subgraph_map.get(&cfp.import_path) {
                    if let Some(called_node) = import_subgraph.nodes().get(&cf.name) {
                        edges.add_edge(MermaidEdge::new(
                            cf.line,
                            caller_node.clone(),
                            called_node.to_owned(),
                            color.to_string()
                        ));
                    }
                }
            }
        }
        log::debug!("[generate_caller_elements] edges = {:#?}", &edges);      
    }
}


fn get_func_from_line(line: usize, flinemaps: &[FunctionLineMap]) -> Option<String> {
    for flinemap in flinemaps {
        log::debug!("[get_func_from_line] flinemap = {:#?}, line: {}", &flinemap, line);
        log::debug!(
            "[get_func_from_line] condition = {:?}",
            (flinemap.line_start <= line as i32 && flinemap.line_end >= line as i32));
        if flinemap.line_start <= line as i32 && flinemap.line_end >= line as i32 {
            log::debug!("[get_func_from_line] inside if");
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
        if let Some(subgraph) = subgraph_map.get_mut(&cfp.import_path) {
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
                cfp.import_path.to_string(),
                node_map 
            );
            subgraph_map.insert(cfp.import_path.to_string(), subgraph);
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
        log::error!("[generate_called_function_info] Unable to get called function paths for file: {}", filename);
        return None;
    }
    let called_func_paths = called_func_paths_opt.expect("Empty called_func_paths_opt");
    return Some((called_funcs, called_func_paths));
}