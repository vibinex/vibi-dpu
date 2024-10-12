use std::{collections::HashMap, path::PathBuf};
use crate::{graph::function_line_range::generate_function_map, utils::{gitops::{git_checkout_commit, StatItem}, review::Review}};
use super::{function_call::{FunctionCallChunk, FunctionCallIdentifier, FunctionCallsOutput}, function_line_range::{AllFileFunctions, HunkFuncDef}, function_name::FunctionNameIdentifier, gitops::{get_changed_hunk_lines, HunkDiffMap}, utils::{detect_language, numbered_content, read_file, source_diff_files}};

#[derive(Debug, Default, Clone)]
pub struct DiffFuncDefs {
    added_func_defs: Vec<HunkFuncDef>,
    deleted_func_defs: Vec<HunkFuncDef>
}

impl DiffFuncDefs {
    pub fn extend_added_funcdefs(&mut self, add_funcdefs: Vec<HunkFuncDef>) {
        self.added_func_defs.extend(add_funcdefs);
    }

    pub fn extend_deleted_funcdefs(&mut self, del_funcdefs: Vec<HunkFuncDef>) {
        self.deleted_func_defs.extend(del_funcdefs);
    }

    pub fn added_func_defs(&self) -> &Vec<HunkFuncDef> {
        &self.added_func_defs
    }

    pub fn deleted_func_defs(&self) -> &Vec<HunkFuncDef> {
        &self.deleted_func_defs
    }
}
#[derive(Debug, Default, Clone)]
pub struct FuncCall {
    // import_info: ImportPath,
    call_info: Vec<FunctionCallChunk>
}

impl FuncCall {
    // pub fn import_info(&self) -> &ImportPath {
    //     &self.import_info
    // }
    pub fn call_info(&self) -> &Vec<FunctionCallChunk> {
        &self.call_info
    }

    // pub fn func_call_hunk_lines(&self, hunk_diff: &HunkDiffLines) -> Option<FuncCall> {
    //     let mut hunk_func_calls_lines = Vec::<usize>::new();
    //     for func_call in self.call_info() {
    //         for call_line in func_call.function_calls() {
    //             if hunk_diff.start_line() <= call_line && hunk_diff.end_line() >= call_line {
    //                 hunk_func_calls_lines.push(call_line.to_owned());
    //             }
    //         }
    //     }
    //     if hunk_func_calls_lines.is_empty() {
    //         return None;
    //     }
    //     let hunk_func_call = FuncCall{
    //         import_info: self.import_info.clone(),
    //         call_info: vec![FunctionCallChunk::new(hunk_func_calls_lines, 
    //             self.import_info().imported().to_string())]};
    //     return Some(hunk_func_call);
    // }

    // pub fn function_name(&self) -> &String {
    //     self.import_info.imported()
    // }
}

#[derive(Debug, Default, Clone)]
pub struct DiffFuncCall {
    added_calls: FunctionCallsOutput,
    deleted_calls: FunctionCallsOutput
}

impl DiffFuncCall {
    // pub fn add_added_calls(&mut self, add_calls: FuncCall) {
    //     self.added_calls.push(add_calls);
    // }
    
    // pub fn add_deleted_calls(&mut self, del_calls: FuncCall) {
    //     self.deleted_calls.push(del_calls);
    // }

    pub fn added_calls(&self) -> &FunctionCallsOutput {
        &self.added_calls
    }

    pub fn deleted_calls(&self) -> &FunctionCallsOutput {
        &self.deleted_calls
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiffGraph {
    hunk_diff_map: HunkDiffMap
}

impl DiffGraph {
    pub fn hunk_diff_map(&self) -> &HunkDiffMap {
        &self.hunk_diff_map
    }
    // pub fn add_func_def(&mut self, filename: String, diff_func_defs: DiffFuncDefs) {
    //     self.diff_func_defs.insert(filename, diff_func_defs);
    // }

    // pub fn add_diff_func_calls(&mut self, filename: String, diff_func_calls: DiffFuncCall) {
    //     self.diff_func_calls.insert(filename, diff_func_calls);
    // }

    // pub fn all_file_func_defs(&self) -> &AllFileFunctions {
    //     &self.diff_files_func_defs
    // }

    // // pub fn all_file_imports(&self) -> &FilesImportInfo {
    // //     &self.diff_files_imports
    // // }

    // pub fn diff_func_defs(&self) -> &HashMap<String, DiffFuncDefs> {
    //     &self.diff_func_defs
    // }

    // pub fn diff_func_calls(&self) -> &HashMap<String, DiffFuncCall> {
    //     &self.diff_func_calls
    // }

    // pub fn func_calls_for_func(&self, function_name: &str, filename: &str) -> Option<&FuncCall> {
    //     if let Some(func_call_map) =  self.diff_files_func_calls.get(filename) {
    //         if let Some(func_call) = func_call_map.get(function_name) {
    //             return Some(func_call)
    //         }
    //     }
    //     return None;
    // }
}

pub async fn generate_diff_graph(diff_files: &Vec<StatItem>, review: &Review) -> Option<DiffGraph> {
    let diff_code_files_opt = source_diff_files(diff_files);
    if diff_code_files_opt.is_none() {
        log::debug!("[generate_diff_graph] No relevant source diff files in: {:#?}", diff_files);
        return None;
    }
    let diff_code_files = diff_code_files_opt.expect("Empty diff_code_files_opt");
    let mut hunk_diff_map = get_changed_hunk_lines(&diff_code_files, review);
    // get func defs for base commit for files in diff
    log::debug!("[generate_diff_graph] hunk diff map =======~~~~~~~~ {:#?}", &hunk_diff_map);
    let diff_graph_opt = process_hunk_diff(&mut hunk_diff_map, review).await;
    return diff_graph_opt;
}

async fn process_hunk_diff(hunk_diff_map: &mut HunkDiffMap, review: &Review) -> Option<DiffGraph> {
    // full graph func def and import info for diff selected files is required.
    let func_name_identifier_opt = FunctionNameIdentifier::new();
    if func_name_identifier_opt.is_none() {
        log::error!("[process_hunk_diff] Unable to initialize function name identifier");
        return None;
    }
    let mut func_name_identifier = func_name_identifier_opt.expect("Empty func_name_identifier_opt");
    git_checkout_commit(review, review.pr_head_commit());
    set_func_def_info(hunk_diff_map, &mut func_name_identifier, true).await;
    git_checkout_commit(review, review.base_head_commit());
    set_func_def_info(hunk_diff_map, &mut func_name_identifier, false).await;
    let diff_graph = DiffGraph {
        hunk_diff_map: hunk_diff_map.to_owned()
    };
    return Some(diff_graph);
    // let all_diff_files = hunk_diff_map.all_files_pathbuf(review.clone_dir());
    // // do generate function defs , only starting line
    // let base_commit_func_defs_opt = generate_function_map(&all_diff_files).await;
    // if base_commit_func_defs_opt.is_none() {
    //     log::debug!("[process_hunk_diff] Unable to generate func defs for base commit");
    //     return None;
    // }
    // let base_commit_func_defs = base_commit_func_defs_opt.expect("Empty let base_commit_func_defs_opt");
    // let base_func_calls_opt = diff_file_func_calls(&all_diff_files, hunk_diff_map, false).await;
    // if base_func_calls_opt.is_none() {
    //     log::debug!("[process_hunk_diff] Unable to calculate diff_file_func_calls");
    //     return None;
    // }
    // let base_func_calls = base_func_calls_opt.expect("Empty base_func_calls_opt");
    // git_checkout_commit(review, &review.pr_head_commit());
    // let diff_func_defs_opt = generate_function_map(&all_diff_files).await;
    // // let diff_imports_opt = get_import_lines(&all_diff_files).await;
    // // TODO FIXME - opt logic
    // if diff_func_defs_opt.is_none() {
    //     log::debug!("[process_hunk_diff] Unable to generate func definitions diff map");
    //     return None;
    // }
    // let diff_files_func_defs = diff_func_defs_opt.expect("Empty all_file_func_defs_opt)");
    // let diff_files_func_calls_opt = diff_file_func_calls(&all_diff_files, hunk_diff_map, true).await;
    // if diff_files_func_calls_opt.is_none() {
    //     log::debug!("[process_hunk_diff] Unable to calculate diff_file_func_calls");
    //     return None;
    // }
    // let diff_files_func_calls = diff_files_func_calls_opt.expect("Empty diff_files_func_calls_opt");
    
    // for filepath in &all_diff_files {
    //     let filename = filepath.to_str().expect("Unable to deserialize pathbuf");
    //     let mut diff_func_defs = DiffFuncDefs {
    //         added_func_defs: Vec::new(), deleted_func_defs: Vec::new()};
    //     // define base and diff func calls output for this filename
    //     if let Some(base_func_call) = base_func_calls.get(filename) {
    //         if let Some(diff_func_call) = diff_files_func_calls.get(filename) {
    //             // initialize and add DiffFuncCall to diff_func_calls_map
    //             let func_calls = DiffFuncCall {
    //                 added_calls: diff_func_call.to_owned(), deleted_calls: base_func_call.to_owned()};
    //                 diff_graph.add_diff_func_calls(filename.to_string(), func_calls);
    //         }
    //     };
    //     if let Some(file_line_map) = hunk_diff_map.file_hunks(filename) {
    //         for hunk_diff in file_line_map.added_hunks() {
    //             if let Some(funcs_map) = diff_graph.all_file_func_defs().functions_in_file(filename) {
    //                 // find func_defs for files in hunks
    //                 let funcs_def_vec = funcs_map.funcs_in_hunk(hunk_diff);
    //                 if !funcs_def_vec.is_empty() {
    //                     // add func def vec to something with file as key
    //                     diff_func_defs.extend_added_funcdefs(funcs_def_vec);
    //                 }
    //             }
    //         }
    //         for hunk_diff in file_line_map.deleted_hunks() {                
    //             if let Some(funcs_map) = base_commit_func_defs.functions_in_file(filename) {
    //                 // find func_defs for files in hunks
    //                 let funcs_def_vec = funcs_map.funcs_in_hunk(hunk_diff);
    //                 if !funcs_def_vec.is_empty() {
    //                     // add func def vec to something with file as key
    //                     diff_func_defs.extend_deleted_funcdefs(funcs_def_vec);
    //                 }
    //             }
    //         }
            // TODO FIXME - why no deleted func calls, and how is only diff part sent to find func calls?
            // find func call in hunks for each import
            // want to record not all func_calls but hunk specific line numbers
            // might need to reorder for loops to make sure repeated calcs are avoided
            // if let Some(imports_info) = diff_graph.all_file_imports().file_import_info(filename) {
            //     for import_info in imports_info.all_import_paths() {
            //         if let Some(func_call) =  diff_graph.func_calls_for_func(import_info.imported(), filename) {
            //             diff_func_calls_add.add_added_calls(func_call.to_owned());
            //         }
            //         // todo fixme - finding all func calls in file needs a different approach to add added and deleted calls
            //         // TODO FIXME - need function call calc for all diff files, need to search for funcdefs as well as imports
            //         // if let Some(func_calls) = function_calls_in_file(&filepath, import_info.imported()).await {
            //         //     // func_calls is basically all func calls of a function in the latest commit of the file
            //         //     if let Some(file_line_map) = hunk_diff_map.file_hunks(filename) {
            //         //         let func_call = FuncCall{ import_info, call_info: func_calls };
            //         //         for hunk_diff in file_line_map.added_hunks() {
            //         //             if let Some(hunk_func_call) =  func_call.func_call_hunk_lines(&hunk_diff) {
            //         //                 diff_func_calls_add.add_added_calls(hunk_func_call);    
            //         //             }
            //         //         }
            //         //     }
            //         // }
            //     }
            // }
            // // Use full graph's import info
            // do a git checkout to base commit
            // do the same thing as done for added_calls
        // }
        // diff_graph.add_func_def(filename.to_string(), diff_func_defs);
        // diff_func_calls_map.insert(filename.to_string(), diff_func_calls_add);
    }
    // git_checkout_commit(review, &review.base_head_commit());
    // for filepath in &all_diff_files {
    //     let filename = filepath.to_str().expect("Unable to deserialize pathbuf");
    //     let diff_func_call = diff_func_calls_map.entry(filename.to_string()).or_insert(DiffFuncCall { added_calls: Vec::new(), deleted_calls: Vec::new() });
        
    //     // if let Some(imports_info) = base_commit_import_info.file_import_info(filename) {
    //     //     for import_info in imports_info.all_import_paths() {
    //     //         // todo fixme - finding all func calls in file needs a different approach to add added and deleted calls
    //     //         if let Some(func_calls) = function_calls_in_file(&filepath, import_info.imported()).await {
    //     //             // func_calls is basically all func calls of a function in the latest commit of the file
    //     //             if let Some(file_line_map) = hunk_diff_map.file_hunks(filename) {
    //     //                 let func_call = FuncCall{ import_info, call_info: func_calls };
    //     //                 for hunk_diff in file_line_map.deleted_hunks() {
    //     //                     if let Some(hunk_func_call) =  func_call.func_call_hunk_lines(&hunk_diff) {
    //     //                         diff_func_call_del.add_deleted_calls(hunk_func_call);    
    //     //                     }
    //     //                 }
    //     //             }
    //     //         }
    //     //     }
    //     // }
    // }
    // // for (filename, diff_func_call) in diff_func_calls_map.iter() {
    //     diff_graph.add_diff_func_calls(filename.to_owned(), diff_func_call.to_owned());
    // }
//     return Some(diff_graph);
// } 

// async fn diff_file_func_calls(all_diff_files: &Vec<PathBuf>, hunk_diff_map: &HunkDiffMap, added: bool) -> Option<HashMap<String, Vec<HunkDiffLines, FunctionCallsOutput>>> {
//     // func calls made in diff hunks for all diff files
//     let mut func_call_file_map = HashMap::new();
//     let func_call_identifier_opt = FunctionCallIdentifier::new();
//     if func_call_identifier_opt.is_none() {
//         log::error!("[diff_file_func_calls] Unable to create FunctionCallIdentifier");
//         return None;
//     }
//     let mut func_call_identifier = func_call_identifier_opt.expect("Empty func_call_identifier_opt");
//     for filepathbuf in all_diff_files {
//         let filepath = filepathbuf.to_str().expect("Unable to deserialize pathbuf");
//         let hunk_diffs_opt = hunk_diff_map.file_hunks(filepath);
//         if hunk_diffs_opt.is_none() {
//             log::debug!("[diff_file_func_calls] No entry in hunk_diff_map for {}", filepath);
//             continue;
//         }
//         let hunk_diffs = hunk_diffs_opt.expect("Empty hunk_diffs_opt");
//         let file_hunks;
//         if added {
//             file_hunks = hunk_diffs.added_hunks();
//         } else {
//             file_hunks = hunk_diffs.deleted_hunks();
//         }
//         let func_calls_opt = func_call_identifier.function_calls_in_hunks(filepathbuf, "rust", file_hunks).await;
//         if func_calls_opt.is_none() {
//             log::debug!("[diff_file_func_calls] No function calls in hunks: {}, {:?}", filepath, hunk_diffs);
//             continue;
//         }
//         let func_calls = func_calls_opt.expect("Empty func_calls_opt");
//         func_call_file_map.insert(filepath.to_string(), func_calls);
//     }
//     return Some(func_call_file_map);
// }

async fn set_func_def_info(hunk_diff_map: &mut HunkDiffMap, func_name_identifier: &mut FunctionNameIdentifier, added: bool) {
    for (filepath, file_func_diff) in hunk_diff_map.file_line_map_mut() {
        let file_hunks;
        if added {
            file_hunks = file_func_diff.added_hunks_mut();
        } else {
            file_hunks = file_func_diff.deleted_hunks_mut();
        }
        for file_hunk in file_hunks {
            if let Some(func_line_raw) = file_hunk.function_line().clone() {
                // get line number
                if let Some(file_contents) = read_file(filepath) {
                    let line_number_opt = file_contents
                        .lines() // Split into lines
                        .enumerate() // Get (index, line)
                        .position(|(_, line)| line.contains(&func_line_raw)) // Find the position where the line matches
                        .map(|index| index + 1); // Convert 0-based index to 1-based line number

                    file_hunk.set_line_number(line_number_opt);
                    if let Some(lang) = detect_language(filepath) {
                        if let Some(func_name) = func_name_identifier.function_name_in_line(&func_line_raw, &lang).await {
                            file_hunk.set_function_name(func_name.get_function_name().to_string());
                        }    
                    }
                }
            }
        }
    }
}