use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{db::graph_info::{get_import_info_from_db, save_import_info_to_db}, graph::{file_imports::get_import_lines, function_line_range::generate_function_map, utils::all_code_files}, utils::{gitops::StatItem, review::Review}};

use super::{file_imports::{AllFileImportInfo, ImportPath}, function_call::{function_calls_in_file, FunctionCallChunk}, function_line_range::{AllFileFunctions, FuncDefInfo}, gitops::{get_changed_hunk_lines, HunkDiffMap}, utils::source_diff_files};

// #[derive(Debug, Serialize, Default, Deserialize, Clone)]
// pub struct DiffInfo {
//     added_funcs: Option<HashMap<String, Vec<FuncDefInfo>>>, // key is filename
//     deleted_funcs: Option<HashMap<String, Vec<FuncDefInfo>>>, // key is filename
//     added_imports: Option<HashMap<String, Vec<ImportPath>>>, // key is filename
//     deleted_imports: Option<HashMap<String, Vec<ImportPath>>> // key is filename
// }

// impl DiffInfo {
//     pub fn added_funcs(&self) -> &Option<HashMap<String, Vec<FuncDefInfo>>> {
//         &self.added_funcs
//     }

//     pub fn deleted_funcs(&self) -> &Option<HashMap<String, Vec<FuncDefInfo>>> {
//         &self.deleted_funcs
//     }

//     pub fn added_imports(&self) -> &Option<HashMap<String, Vec<ImportPath>>> {
//         &self.added_imports
//     }

//     pub fn deleted_imports(&self) -> &Option<HashMap<String, Vec<ImportPath>>> {
//         &self.deleted_imports
//     }
// }

// async fn generate_graph_info(source_file_paths: &Vec<PathBuf>) -> Option<AllFileImportInfo> {
//     // let function_map_opt = generate_function_map(source_file_paths).await;
//     // if function_map_opt.is_none() {
//     //     log::error!("[generate_graph_info] Unable to generate function map");
//     //     return None;
//     // }
//     // let function_map = function_map_opt.expect("Empty function_map_opt");
//     // log::debug!("[generate_graph_info] func map = {:?}", &function_map);
//     let all_file_import_info_opt = get_import_lines(source_file_paths).await;
//     if all_file_import_info_opt.is_none() {
//         log::error!("[generate_graph_info] Unable to get import info for source files: {:#?}", source_file_paths);
//         return None;
//     }
//     let all_file_import_info = all_file_import_info_opt.expect("Empty import_lines_opt");
//     let graph_info = GraphInfo { function_info: function_map, 
//         import_info: all_file_import_info };
//     return Some(graph_info);
// }

// pub async fn generate_full_graph(repo_dir: &str, review_key: &str, commit_id: &str) -> Option<GraphInfo> {
//     // check for graph db
//     if let Some(graph_info) = get_import_info_from_db(review_key, commit_id) {
//         return Some(graph_info);
//     }
//     let repo_code_files_opt = all_code_files(repo_dir);
//     if repo_code_files_opt.is_none() {
//         log::error!("[generate_full_graph] Unable to get file paths: {}", repo_dir);
//         return None;
//     }
//     let repo_code_files = repo_code_files_opt.expect("Empty repo_code_files_opt");
//     let graph_info_opt = generate_graph_info(&repo_code_files).await;
//     if graph_info_opt.is_none() {
//         log::error!("[generate_full_graph] Unable to generate full graph for commit: {}", commit_id);
//         return None;
//     }
//     let graph_info = graph_info_opt.expect("Empty graph_info_opt");
//     // save all this to db
//     save_import_info_to_db(review_key, commit_id, &graph_info);
//     return Some(graph_info);
// }

#[derive(Debug, Default, Clone)]
pub struct DiffFuncDefs {
    added_func_defs: Vec<FuncDefInfo>,
    deleted_func_defs: Vec<FuncDefInfo>
}

impl DiffFuncDefs {
    pub fn extend_added_funcdefs(&mut self, add_funcdefs: Vec<FuncDefInfo>) {
        self.added_func_defs.extend(add_funcdefs);
    }

    pub fn extend_deleted_funcdefs(&mut self, del_funcdefs: Vec<FuncDefInfo>) {
        self.deleted_func_defs.extend(del_funcdefs);
    }

    pub fn added_func_defs(&self) -> &Vec<FuncDefInfo> {
        &self.added_func_defs
    }

    pub fn deleted_func_defs(&self) -> &Vec<FuncDefInfo> {
        &self.deleted_func_defs
    }
}
#[derive(Debug, Default, Clone)]
pub struct FuncCall {
    import_info: ImportPath,
    call_info: Vec<FunctionCallChunk>
}

impl FuncCall {
    pub fn import_info(&self) -> &ImportPath {
        &self.import_info
    }
    pub fn call_info(&self) -> &Vec<FunctionCallChunk> {
        &self.call_info
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiffFuncCall {
    added_calls: Vec<FuncCall>,
    deleted_calls: Vec<FuncCall>
}

impl DiffFuncCall {
    pub fn add_added_calls(&mut self, add_calls: FuncCall) {
        self.added_calls.push(add_calls);
    }
    
    pub fn add_deleted_calls(&mut self, del_calls: FuncCall) {
        self.deleted_calls.push(del_calls);
    }

    pub fn added_calls(&self) -> &Vec<FuncCall> {
        &self.added_calls
    }

    pub fn deleted_calls(&self) -> &Vec<FuncCall> {
        &self.deleted_calls
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiffGraph {
    diff_files_func_defs: AllFileFunctions,
    diff_files_imports: AllFileImportInfo,
    diff_func_defs: HashMap<String, DiffFuncDefs>,
    diff_func_calls: HashMap<String, DiffFuncCall>
}

impl DiffGraph {
    pub fn add_func_def(&mut self, filename: String, diff_func_defs: DiffFuncDefs) {
        self.diff_func_defs.insert(filename, diff_func_defs);
    }

    pub fn add_diff_func_calls(&mut self, filename: String, diff_func_calls: DiffFuncCall) {
        self.diff_func_calls.insert(filename, diff_func_calls);
    }

    pub fn all_file_func_defs(&self) -> &AllFileFunctions {
        &self.diff_files_func_defs
    }

    pub fn all_file_imports(&self) -> &AllFileImportInfo {
        &self.diff_files_imports
    }

    pub fn diff_func_defs(&self) -> &HashMap<String, DiffFuncDefs> {
        &self.diff_func_defs
    }

    pub fn diff_func_calls(&self) -> &HashMap<String, DiffFuncCall> {
        &self.diff_func_calls
    }
}

pub async fn generate_diff_graph(diff_files: &Vec<StatItem>, review: &Review) -> Option<DiffGraph> {
    let diff_code_files_opt = source_diff_files(diff_files);
    if diff_code_files_opt.is_none() {
        log::debug!("[generate_diff_graph] No relevant source diff files in: {:#?}", diff_files);
        return None;
    }
    let diff_code_files = diff_code_files_opt.expect("Empty diff_code_files_opt");
    let hunk_diff_map = get_changed_hunk_lines(&diff_code_files, review);
    let diff_graph_opt = process_hunk_diff(&hunk_diff_map).await;
    return diff_graph_opt;
    // let diff_code_files_pathbuf: Vec<PathBuf> = diff_code_files
    //     .iter()
    //     .filter_map(|s| {
    //         // Try to convert each &str to a PathBuf
    //         let s_pathbuf_res = PathBuf::from_str(&s.filepath);
    //         match s_pathbuf_res {
    //             Ok(pathbuf) => Some(pathbuf),
    //             Err(_) => None,
    //         }
    //     })
    //     .collect();
    // let graph_info_opt = generate_graph_info(&diff_code_files_pathbuf).await;
    // if graph_info_opt.is_none() {
    //     log::error!("[generate_diff_graph] Unable to generate diff graph");
    //     return (None, deleted_files_opt);
    // }
    // let graph_info = graph_info_opt.expect("Empty graph_info_opt");
    // // return (Some(graph_info), deleted_files_opt);
    // return None;
}

async fn process_hunk_diff(hunk_diff_map: &HunkDiffMap) -> Option<DiffGraph> {
    let all_files = hunk_diff_map.all_files_pathbuf();
    let all_file_func_defs_opt = generate_function_map(&all_files).await;
    let all_file_imports_opt = get_import_lines(&all_files).await;
    // TODO FIXME - opt logic
    if all_file_func_defs_opt.is_none() {
        log::debug!("[process_hunk_diff] Unable to generate func definitions diff map");
        return None;
    }
    if all_file_imports_opt.is_none() {
        log::debug!("[process_hunk_diff] Unable to generate func imports diff map");
        return None;
    }
    let all_file_func_defs = all_file_func_defs_opt.expect("Empty all_file_func_defs_opt)");
    let all_file_imports = all_file_imports_opt.expect("Empty all_file_imports_opt");
    let mut diff_graph = DiffGraph {
        diff_files_func_defs: all_file_func_defs,
        diff_files_imports: all_file_imports,
        diff_func_defs: HashMap::new(),
        diff_func_calls: HashMap::new(),
    };
    for filepath in all_files {
        let filename = filepath.to_str().expect("Unable to deserialize pathbuf");
        let mut diff_func_defs = DiffFuncDefs {
            added_func_defs: Vec::new(), deleted_func_defs: Vec::new()};
        let mut diff_func_calls = DiffFuncCall {
            added_calls: Vec::new(), deleted_calls: Vec::new()};
        if let Some(file_line_map) = hunk_diff_map.file_hunks(filename) {
            for hunk_diff in file_line_map.added_hunks() {
                if let Some(funcs_map) = diff_graph.all_file_func_defs().functions_in_file(filename) {
                    // find func_defs for files in hunks
                    let funcs_def_vec = funcs_map.funcs_in_hunk(hunk_diff);
                    if !funcs_def_vec.is_empty() {
                        // add func def vec to something with file as key
                        diff_func_defs.extend_added_funcdefs(funcs_def_vec);
                    }
                }
            }
            for hunk_diff in file_line_map.deleted_hunks() {
                if let Some(funcs_map) = diff_graph.all_file_func_defs().functions_in_file(filename) {
                    // find func_defs for files in hunks
                    let funcs_def_vec = funcs_map.funcs_in_hunk(hunk_diff);
                    if !funcs_def_vec.is_empty() {
                        // add func def vec to something with file as key
                        diff_func_defs.extend_deleted_funcdefs(funcs_def_vec);
                    }
                }
            }
            // find func call in hunks for each import
            if let Some(imports_info) = diff_graph.all_file_imports().file_import_info(filename) {
                for import_info in imports_info.all_import_paths() {
                    if let Some(func_calls) = function_calls_in_file(&filepath, import_info.imported()).await {
                        // add these func calls to something with file as key
                        let func_call = FuncCall{ import_info, call_info: func_calls };
                        diff_func_calls.add_added_calls(func_call);
                    }
                }
            }
        }
        diff_graph.add_func_def(filename.to_string(), diff_func_defs);
        diff_graph.add_diff_func_calls(filename.to_string(), diff_func_calls);
    }
    return Some(diff_graph);
}

// fn added_functions_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<FuncDefInfo>>> {
//     let mut added_funcs = HashMap::<String, Vec<FuncDefInfo>>::new();
//     for filename in diff_graph.function_info().all_files() {
//         let func_map_opt = full_graph.function_info().functions_in_file(filename);
//         if func_map_opt.is_none() {
//             if let Some(diff_func_map) = diff_graph.function_info().functions_in_file(filename) {
//                 let funcs_vec = diff_func_map.functions().to_owned();
//                 added_funcs.entry(filename.to_string())
//                     .or_insert_with(Vec::new)
//                     .extend(funcs_vec);
//             }
//         } else {
//             let full_func_map = func_map_opt.expect("Empty func_map_opt");
//             if let Some(diff_func_map) = diff_graph.function_info().functions_in_file(filename) {
//                 for func in diff_func_map.functions() {
//                     if !full_func_map.is_func_in_file(func) {
//                         added_funcs.entry(filename.to_string())
//                             .or_insert_with(Vec::new)
//                             .push(func.to_owned());
//                     }
//                 }    
//             }
//         }
//     }
//     if added_funcs.is_empty() {
//         return None;
//     }
//     return Some(added_funcs);
// }

// fn deleted_functions_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<FuncDefInfo>>> {
//     let mut deleted_funcs = HashMap::<String, Vec<FuncDefInfo>>::new();
//     for filename in diff_graph.function_info().all_files() {
//         // TODO - full file deleted?
//         let funcs_opt = full_graph.function_info().functions_in_file(filename);
//         if funcs_opt.is_none() {
//             // file added
//         }
//         let full_funcs = funcs_opt.expect("Empty funcs_opt");
//         let diff_funcs = diff_graph.function_info().functions_in_file(filename).expect("Empty diff_funcs");
//         for func in full_funcs.functions() {
//             if diff_funcs.is_func_in_file(func) {
//                 deleted_funcs.entry(filename.to_string())
//                     .or_insert_with(Vec::new)
//                     .push(func.to_owned());
//             }
//         }
//     }
//     if deleted_funcs.is_empty() {
//         return None;
//     }
//     return Some(deleted_funcs)
// }

// fn added_imports_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<ImportPath>>> {
//     let mut added_imports = HashMap::<String, Vec<ImportPath>>::new();
//     for filename in diff_graph.import_info().files() {
//         let diff_imports = diff_graph
//             .import_info()
//             .file_import_info(filename).expect("Empty diff imports");
//         let full_imports_opt = full_graph
//             .import_info().file_import_info(filename);
//         if full_imports_opt.is_none() {
//             added_imports.entry(filename.to_string())
//                 .or_insert_with(Vec::new)
//                 .extend(diff_imports.all_import_paths());
//         } else {
//             for import_path in diff_imports.all_import_paths() {
//                 if !full_graph.import_info().is_import_in_file(filename, &import_path) {
//                     added_imports.entry(filename.to_string())
//                         .or_insert_with(Vec::new)
//                         .push(import_path);
//                 }
//             }
//         }
//     }
//     if added_imports.is_empty() {
//         return None;
//     }
//     return Some(added_imports);
// }

// fn deleted_imports_diff(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> Option<HashMap<String, Vec<ImportPath>>> {
//     let mut deleted_imports = HashMap::<String, Vec<ImportPath>>::new();
//     // TODO - file deleted
//     for filename in diff_graph.import_info().files() {
//         let full_imports_opt = full_graph.import_info().file_import_info(filename);
//         if full_imports_opt.is_none() {
//             // file added
//         }
//         let full_imports = full_imports_opt.expect("Empty full_imports_opt");
//         for import_path in full_imports.all_import_paths() {
//             if !diff_graph.import_info().is_import_in_file(filename, &import_path) {
//                 deleted_imports.entry(filename.to_string())
//                     .or_insert_with(Vec::new)
//                     .push(import_path);
//             }
//         }
//     }
//     if deleted_imports.is_empty() {
//         return None;
//     }
//     return Some(deleted_imports);
// }

// pub fn generate_diff_info(full_graph: &GraphInfo, diff_graph: &GraphInfo) -> DiffInfo {
    // Get added funcs and imports
    // let added_funcs_opt = added_functions_diff(full_graph, diff_graph);
    // let deleted_funcs_opt = deleted_functions_diff(full_graph, diff_graph);
    // let added_imports_opt = added_imports_diff(full_graph, diff_graph);
    // let deleted_imports_opt = deleted_imports_diff(full_graph, diff_graph);
    // return DiffInfo {
    //     added_funcs: added_funcs_opt,
    //     deleted_funcs: deleted_funcs_opt,
    //     added_imports: added_imports_opt,
    //     deleted_imports: deleted_imports_opt
    // };
// }