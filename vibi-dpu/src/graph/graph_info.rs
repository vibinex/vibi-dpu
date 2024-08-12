use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{db::graph_info::{get_graph_info_from_db, save_graph_info_to_db}, graph::{file_imports::get_import_lines, function_line_range::generate_function_map, utils::all_code_files}, utils::gitops::StatItem};

use super::{file_imports::{AllFileImportInfo, ImportPath}, function_line_range::{AllFileFunctions, FuncDefInfo}, utils::source_diff_files};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct DiffInfo {
    added_funcs: Option<HashMap<String, Vec<FuncDefInfo>>>, // key is filename
    deleted_funcs: Option<HashMap<String, Vec<FuncDefInfo>>>, // key is filename
    added_imports: Option<HashMap<String, Vec<ImportPath>>>, // key is filename
    deleted_imports: Option<HashMap<String, Vec<ImportPath>>> // key is filename
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct GraphInfo {
    function_info: AllFileFunctions,
    import_info: AllFileImportInfo
}

impl GraphInfo {
    pub fn function_info(&self) -> &AllFileFunctions {
        &self.function_info
    }

    pub fn import_info(&self) -> &AllFileImportInfo {
        &self.import_info
    }
}

async fn generate_graph_info(source_file_paths: &Vec<PathBuf>) -> Option<GraphInfo> {
    let function_map_opt = generate_function_map(source_file_paths).await;
    if function_map_opt.is_none() {
        log::error!("[generate_graph_info] Unable to generate function map");
        return None;
    }
    let function_map = function_map_opt.expect("Empty function_map_opt");
    log::debug!("[generate_graph_info] func map = {:?}", &function_map);
    let all_file_import_info_opt = get_import_lines(source_file_paths).await;
    if all_file_import_info_opt.is_none() {
        log::error!("[generate_graph_info] Unable to get import info for source files: {:#?}", source_file_paths);
        return None;
    }
    let all_file_import_info = all_file_import_info_opt.expect("Empty import_lines_opt");
    let graph_info = GraphInfo { function_info: function_map, 
        import_info: all_file_import_info };
    return Some(graph_info);
}

pub async fn generate_full_graph(repo_dir: &str, review_key: &str, commit_id: &str) -> Option<GraphInfo> {
    // check for graph db
    if let Some(graph_info) = get_graph_info_from_db(review_key, commit_id) {
        return Some(graph_info);
    }
    let repo_code_files_opt = all_code_files(repo_dir);
    if repo_code_files_opt.is_none() {
        log::error!("[generate_full_graph] Unable to get file paths: {}", repo_dir);
        return None;
    }
    let repo_code_files = repo_code_files_opt.expect("Empty repo_code_files_opt");
    let graph_info_opt = generate_graph_info(&repo_code_files).await;
    if graph_info_opt.is_none() {
        log::error!("[generate_full_graph] Unable to generate full graph for commit: {}", commit_id);
        return None;
    }
    let graph_info = graph_info_opt.expect("Empty graph_info_opt");
    // save all this to db
    save_graph_info_to_db(review_key, commit_id, &graph_info);
    return Some(graph_info);
}

pub async fn generate_diff_graph(diff_files: &Vec<StatItem>) -> Option<GraphInfo> {
    let diff_code_files_opt = source_diff_files(diff_files);
    if diff_code_files_opt.is_none() {
        log::error!("[generate_diff_graph] Unable to get file paths for: {:#?}", diff_files);
        return None;
    }
    let diff_code_files = diff_code_files_opt.expect("Empty diff_code_files_opt");
    let graph_info_opt = generate_graph_info(&diff_code_files).await;
    if graph_info_opt.is_none() {
        log::error!("[generate_diff_graph] Unable to generate diff graph");
        return None;
    }
    let graph_info = graph_info_opt.expect("Empty graph_info_opt");
    return Some(graph_info);
}

fn added_functions_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<FuncDefInfo>>> {
    let mut added_funcs = HashMap::<String, Vec<FuncDefInfo>>::new();
    for filename in diff_graph.function_info().all_files() {
        let func_map_opt = full_graph.function_info().functions_in_file(filename);
        if func_map_opt.is_none() {
            if let Some(diff_func_map) = diff_graph.function_info().functions_in_file(filename) {
                let funcs_vec = diff_func_map.functions().to_owned();
                added_funcs.entry(filename.to_string())
                    .or_insert_with(Vec::new)
                    .extend(funcs_vec);
            }
        } else {
            let full_func_map = func_map_opt.expect("Empty func_map_opt");
            if let Some(diff_func_map) = diff_graph.function_info().functions_in_file(filename) {
                for func in diff_func_map.functions() {
                    if !full_func_map.is_func_in_file(func) {
                        added_funcs.entry(filename.to_string())
                            .or_insert_with(Vec::new)
                            .push(func.to_owned());
                    }
                }    
            }
        }
    }
    if added_funcs.is_empty() {
        return None;
    }
    return Some(added_funcs);
}

fn deleted_functions_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<FuncDefInfo>>> {
    let mut deleted_funcs = HashMap::<String, Vec<FuncDefInfo>>::new();
    for filename in diff_graph.function_info().all_files() {
        // TODO - full file deleted?
        let funcs_opt = full_graph.function_info().functions_in_file(filename);
        if funcs_opt.is_none() {
            // file added
        }
        let full_funcs = funcs_opt.expect("Empty funcs_opt");
        let diff_funcs = diff_graph.function_info().functions_in_file(filename).expect("Empty diff_funcs");
        for func in full_funcs.functions() {
            if diff_funcs.is_func_in_file(func) {
                deleted_funcs.entry(filename.to_string())
                    .or_insert_with(Vec::new)
                    .push(func.to_owned());
            }
        }
    }
    if deleted_funcs.is_empty() {
        return None;
    }
    return Some(deleted_funcs)
}

fn added_imports_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<ImportPath>>> {
    let mut added_imports = HashMap::<String, Vec<ImportPath>>::new();
    for filename in diff_graph.import_info().files() {
        let diff_imports = diff_graph
            .import_info()
            .file_import_info(filename).expect("Empty diff imports");
        let full_imports_opt = full_graph
            .import_info().file_import_info(filename);
        if full_imports_opt.is_none() {
            added_imports.entry(filename.to_string())
                .or_insert_with(Vec::new)
                .extend(diff_imports.all_import_paths());
        } else {
            for import_path in diff_imports.all_import_paths() {
                if !full_graph.import_info().is_import_in_file(filename, &import_path) {
                    added_imports.entry(filename.to_string())
                        .or_insert_with(Vec::new)
                        .push(import_path);
                }
            }
        }
    }
    if added_imports.is_empty() {
        return None;
    }
    return Some(added_imports);
}

fn deleted_imports_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<ImportPath>>> {
    let mut deleted_imports = HashMap::<String, Vec<ImportPath>>::new();
    // TODO - file deleted
    for filename in diff_graph.import_info().files() {
        let full_imports_opt = full_graph.import_info().file_import_info(filename);
        if full_imports_opt.is_none() {
            // file added
        }
        let full_imports = full_imports_opt.expect("Empty full_imports_opt");
        for import_path in full_imports.all_import_paths() {
            if !diff_graph.import_info().is_import_in_file(filename, &import_path) {
                deleted_imports.entry(filename.to_string())
                    .or_insert_with(Vec::new)
                    .push(import_path);
            }
        }
    }
    if deleted_imports.is_empty() {
        return None;
    }
    return Some(deleted_imports);
}

pub fn generate_diff_info(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> DiffInfo {
    // Get added funcs and imports
    let added_funcs_opt = added_functions_diff(full_graph, diff_graph);
    let deleted_funcs_opt = deleted_functions_diff(full_graph, diff_graph);
    let added_imports_opt = added_imports_diff(full_graph, diff_graph);
    let deleted_imports_opt = deleted_imports_diff(full_graph, diff_graph);
    return DiffInfo {
        added_funcs: added_funcs_opt,
        deleted_funcs: deleted_funcs_opt,
        added_imports: added_imports_opt,
        deleted_imports: deleted_imports_opt
    };
}