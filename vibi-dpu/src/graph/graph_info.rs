use std::{collections::HashMap, path::Path};

use crate::{graph::gitops::get_hunks_all_files, utils::{gitops::{git_checkout_commit, StatItem}, review::Review}};
use super::{function_call::{FunctionCallChunk, FunctionCallsOutput}, function_line_range::HunkFuncDef, function_name::{DefinitionIdentifier, FunctionDefinition, FunctionNameIdentifier}, gitops::HunkDiffMap, utils::{detect_language, read_file}};

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
    call_info: Vec<FunctionCallChunk>
}

impl FuncCall {
    pub fn call_info(&self) -> &Vec<FunctionCallChunk> {
        &self.call_info
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiffFuncCall {
    added_calls: FunctionCallsOutput,
    deleted_calls: FunctionCallsOutput
}

impl DiffFuncCall {

    pub fn added_calls(&self) -> &FunctionCallsOutput {
        &self.added_calls
    }

    pub fn deleted_calls(&self) -> &FunctionCallsOutput {
        &self.deleted_calls
    }
}

#[derive(Debug, Default, Clone)]
pub struct DiffGraph {
    pub hunk_diff_map: HunkDiffMap
}

impl DiffGraph {
    pub fn hunk_diff_map(&self) -> &HunkDiffMap {
        &self.hunk_diff_map
    }
}

pub async fn generate_diff_graph(review: &Review) -> Option<DiffGraph> {
    if let Some(mut hunk_diff_map) = get_hunks_all_files(review) {
        // get func defs for base commit for files in diff
        log::debug!("[generate_diff_graph] hunk diff map =======~~~~~~~~ {:#?}", &hunk_diff_map);
        let diff_graph_opt = process_hunk_diff(&mut hunk_diff_map, review).await;
        return diff_graph_opt;
    }
    return None;
}

async fn process_hunk_diff(hunk_diff_map: &mut HunkDiffMap, review: &Review) -> Option<DiffGraph> {
    // full graph func def and import info for diff selected files is required.
    let func_name_identifier_opt = FunctionNameIdentifier::new();
    if func_name_identifier_opt.is_none() {
        log::error!("[process_hunk_diff] Unable to initialize function name identifier");
        return None;
    }
    let mut func_name_identifier = func_name_identifier_opt.expect("Empty func_name_identifier_opt");
    let def_identifier_opt = DefinitionIdentifier::new();
    if def_identifier_opt.is_none() {
        log::error!("[process_hunk_diff] Unable to initialize definition identifier");
        return None;
    }
    let mut def_identifier = def_identifier_opt.expect("Empty def_identifier_opt");
    git_checkout_commit(review, review.pr_head_commit());
    set_func_def_info(hunk_diff_map, &mut func_name_identifier, true).await;
    let added_files_defs = set_func_def_whole_file(review, hunk_diff_map.added_files(), &mut def_identifier).await;
    hunk_diff_map.add_added_files_map(added_files_defs);
    git_checkout_commit(review, review.base_head_commit());
    set_func_def_info(hunk_diff_map, &mut func_name_identifier, false).await;
    let deleted_files_defs = set_func_def_whole_file(review, hunk_diff_map.deleted_files(), &mut def_identifier).await;
    hunk_diff_map.add_deleted_files_map(deleted_files_defs);
    let diff_graph = DiffGraph {
        hunk_diff_map: hunk_diff_map.to_owned()
    };
    return Some(diff_graph);
}

async fn set_func_def_whole_file(review: &Review, added_deleted_files: Vec<&String>, def_identifier: &mut DefinitionIdentifier) -> HashMap<String, Vec<FunctionDefinition>> {
    // send to prompt object and get vec of all defs, which should be stored in diff map? db?
    let mut file_def_map = HashMap::new();
    let base_path = Path::new(review.clone_dir());
    for rel_filepath in added_deleted_files {
        let rel_path = Path::new(rel_filepath);
        let abs_filepath = base_path.join(rel_path);
        let abs_filepath_str = abs_filepath.to_string_lossy();
        if let Some(lang) = detect_language(&abs_filepath_str) {
            log::debug!("[set_func_def_whole_file] file = {:?}, lang = {:?}", &abs_filepath_str, &lang);
            let func_defs = def_identifier.identify_defs_in_file(&abs_filepath_str, &lang).await;
            file_def_map.insert(abs_filepath_str.to_string(), func_defs);
        } else { log::error!("[set_func_def_whole_file] Unable to detect lang for {:?}", &abs_filepath_str);}
    }
    return file_def_map;
}

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
                    if let Some(line_number) = file_contents
                        .lines() // Split into lines
                        .enumerate() // Get (index, line)
                        .position(|(_, line)| line.contains(&func_line_raw)) // Find the position where the line matches
                        .map(|index| index + 1) // Convert 0-based index to 1-based line number
                    {
                        file_hunk.set_line_number(line_number);
                        if let Some(lang) = detect_language(filepath) {
                            if let Some(func_name) = func_name_identifier.function_name_in_line(&func_line_raw, &lang).await {
                                file_hunk.set_function_name(func_name.get_name().to_string());
                            } else { log:: debug!("[set_func_def_info] No func name for {}", &func_line_raw); }
                        } else { log::debug!("[set_func_def_info] language not detected for: {}", filepath); }
                    } else { log::debug!("[set_func_def_info] line not found: {} in file: {}", &func_line_raw, filepath); }
                }
            }
        }
    }
}