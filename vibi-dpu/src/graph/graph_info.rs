use std::{collections::HashMap, path::PathBuf};
use crate::{core::diff_graph, graph::{function_line_range::generate_function_map}, utils::{gitops::{git_checkout_commit, StatItem}, review::Review}};
use super::{function_call::{function_calls_in_file, FunctionCallChunk}, function_line_range::{AllFileFunctions, FuncDefInfo}, gitops::{get_changed_hunk_lines, HunkDiffLines, HunkDiffMap}, utils::source_diff_files};

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
    // diff_files_imports: FilesImportInfo,
    diff_files_func_calls: HashMap<String, HashMap<String, FuncCall>>,
    diff_func_defs: HashMap<String, DiffFuncDefs>,
    diff_func_calls: HashMap<String, DiffFuncCall>,
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

    // pub fn all_file_imports(&self) -> &FilesImportInfo {
    //     &self.diff_files_imports
    // }

    pub fn diff_func_defs(&self) -> &HashMap<String, DiffFuncDefs> {
        &self.diff_func_defs
    }

    pub fn diff_func_calls(&self) -> &HashMap<String, DiffFuncCall> {
        &self.diff_func_calls
    }

    pub fn func_calls_for_func(&self, function_name: &str, filename: &str) -> Option<&FuncCall> {
        if let Some(func_call_map) =  self.diff_files_func_calls.get(filename) {
            if let Some(func_call) = func_call_map.get(function_name) {
                return Some(func_call)
            }
        }
        return None;
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
    // get func defs for base commit for files in diff
    log::debug!("[generate_diff_graph] hunk diff map =======~~~~~~~~ {:#?}", &hunk_diff_map);
    let diff_graph_opt = process_hunk_diff(&hunk_diff_map, review).await;
    return diff_graph_opt;
}

async fn process_hunk_diff(hunk_diff_map: &HunkDiffMap, review: &Review) -> Option<DiffGraph> {
    // full graph func def and import info for diff selected files is required.
    let all_diff_files = hunk_diff_map.all_files_pathbuf(review.clone_dir());
    // do generate function defs , only starting line
    let base_commit_func_defs_opt = generate_function_map(&all_diff_files).await;
    if base_commit_func_defs_opt.is_none() {
        log::debug!("[process_hunk_diff] Unable to generate func defs for base commit");
        return None;
    }
    let base_commit_func_defs = base_commit_func_defs_opt.expect("Empty let base_commit_func_defs_opt");
    git_checkout_commit(review, &review.pr_head_commit());
    let diff_func_defs_opt = generate_function_map(&all_diff_files).await;
    // let diff_imports_opt = get_import_lines(&all_diff_files).await;
    // TODO FIXME - opt logic
    if diff_func_defs_opt.is_none() {
        log::debug!("[process_hunk_diff] Unable to generate func definitions diff map");
        return None;
    }
    // if diff_imports_opt.is_none() {
    //     log::debug!("[process_hunk_diff] Unable to generate func imports diff map");
    //     return None;
    // }
    let diff_files_func_defs = diff_func_defs_opt.expect("Empty all_file_func_defs_opt)");
    // let diff_files_imports = diff_imports_opt.expect("Empty all_file_imports_opt");
    let diff_files_func_calls = diff_file_func_calls(&all_diff_files, &diff_files_func_defs).await;
    let mut diff_graph = DiffGraph {
        diff_files_func_calls,
        diff_files_func_defs,
        // diff_files_imports,
        diff_func_defs: HashMap::new(),
        diff_func_calls: HashMap::new(),
    };
    let mut diff_func_calls_map: HashMap<String, DiffFuncCall> = HashMap::new();
    for filepath in &all_diff_files {
        let filename = filepath.to_str().expect("Unable to deserialize pathbuf");
        let mut diff_func_defs = DiffFuncDefs {
            added_func_defs: Vec::new(), deleted_func_defs: Vec::new()};
        let mut diff_func_calls_add = DiffFuncCall {
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
                if let Some(funcs_map) = base_commit_func_defs.functions_in_file(filename) {
                    // find func_defs for files in hunks
                    let funcs_def_vec = funcs_map.funcs_in_hunk(hunk_diff);
                    if !funcs_def_vec.is_empty() {
                        // add func def vec to something with file as key
                        diff_func_defs.extend_deleted_funcdefs(funcs_def_vec);
                    }
                }
            }
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
        }
        diff_graph.add_func_def(filename.to_string(), diff_func_defs);
        // diff_func_calls_map.insert(filename.to_string(), diff_func_calls_add);
    }
    // git_checkout_commit(review, &review.base_head_commit());
    // for filepath in &all_diff_files {
    //     let filename = filepath.to_str().expect("Unable to deserialize pathbuf");
    //     let diff_func_call_del = diff_func_calls_map.entry(filename.to_string()).or_insert(DiffFuncCall { added_calls: Vec::new(), deleted_calls: Vec::new() });
    //     if let Some(imports_info) = base_commit_import_info.file_import_info(filename) {
    //         for import_info in imports_info.all_import_paths() {
    //             // todo fixme - finding all func calls in file needs a different approach to add added and deleted calls
    //             if let Some(func_calls) = function_calls_in_file(&filepath, import_info.imported()).await {
    //                 // func_calls is basically all func calls of a function in the latest commit of the file
    //                 if let Some(file_line_map) = hunk_diff_map.file_hunks(filename) {
    //                     let func_call = FuncCall{ import_info, call_info: func_calls };
    //                     for hunk_diff in file_line_map.deleted_hunks() {
    //                         if let Some(hunk_func_call) =  func_call.func_call_hunk_lines(&hunk_diff) {
    //                             diff_func_call_del.add_deleted_calls(hunk_func_call);    
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }
    // for (filename, diff_func_call) in diff_func_calls_map.iter() {
    //     diff_graph.add_diff_func_calls(filename.to_owned(), diff_func_call.to_owned());
    // }
    return Some(diff_graph);
}

async fn diff_file_func_calls(all_diff_files: &Vec<PathBuf>, diff_imports: &FilesImportInfo, diff_file_funcs: &AllFileFunctions) -> HashMap<String, HashMap<String, FuncCall>>{
    let mut func_call_file_map = HashMap::new();
    for filepathbuf in all_diff_files {
        let filepath = filepathbuf.to_str().expect("Unable to deserialize pathbuf");
        let mut func_call_map = HashMap::<String, FuncCall>::new();
        // search using imports
        if let Some(imports_info) = diff_imports.file_import_info(filepath) {
            for import_info in imports_info.all_import_paths() {
                if let Some(func_calls) = function_calls_in_file(
                    &filepathbuf, import_info.imported()).await {
                        let func_call = FuncCall{ import_info, call_info: func_calls };
                        func_call_map.insert(
                            func_call.function_name().to_string(), func_call);
                }
            }
        }
        // search in func defs
        if let Some(func_def_map) = diff_file_funcs.functions_in_file(filepath) {
            for func_def in func_def_map.functions() {
                if let Some(func_calls) = function_calls_in_file(
                    &filepathbuf, func_def.name()).await {
                        let fake_import = ImportPath::new( 0, filepath.to_string(), func_def.name().to_string());
                        let func_call = FuncCall{import_info: fake_import, call_info: func_calls};
                        func_call_map.insert(
                            func_call.function_name().to_string(), func_call);
                }
            }
        }
        func_call_file_map.insert(filepath.to_string(), func_call_map);
    }
    return func_call_file_map;
} 