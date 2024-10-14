use std::{collections::HashSet, path::{Path, PathBuf}};
use crate::utils::{gitops::git_checkout_commit, review::Review};

use super::{elements::MermaidGraphElements, file_imports::ImportIdentifier, function_call::{function_calls_search, FunctionCallIdentifier}, function_line_range::{generate_function_map, get_function_def_for_func_call, FunctionDefIdentifier}, graph_info::DiffGraph, utils::{absolute_to_relative_path, detect_language}};

pub async fn graph_edges(base_filepaths: &Vec<PathBuf>, review: &Review, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    outgoing_edges(base_filepaths, diff_graph, graph_elems, review).await;
    incoming_edges(review, diff_graph, graph_elems).await;
}

async fn incoming_edges(review: &Review, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    let func_def_identifier_opt = FunctionDefIdentifier::new();
    if func_def_identifier_opt.is_none() {
        log::debug!("[outgoing_edges] Unable to create func def identifier");
        return;
    }
    let mut funcdef_identifier = func_def_identifier_opt.expect("Empty func_def_identifier_opt");
    let func_call_identifier_opt = FunctionCallIdentifier::new();
    if func_call_identifier_opt.is_none() {
        log::error!("[incoming_edges] Unable to create new FunctionCallIdentifier");
        return;
    }
    let mut func_call_identifier = func_call_identifier_opt.expect("Empty func_call_identifier_opt");
    git_checkout_commit(review, review.pr_head_commit());
    process_func_defs(
        review,
        diff_graph,
        &mut func_call_identifier,
        graph_elems,
        "green"
    ).await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_defs(
        review,
        diff_graph,
        &mut func_call_identifier,
        graph_elems,
        "red"
    ).await;
}

async fn outgoing_edges(base_filepaths: &Vec<PathBuf>, diff_graph: &DiffGraph,
    graph_elems: &mut MermaidGraphElements, review: &Review) 
{
    let func_call_identifier_opt = FunctionCallIdentifier::new();
    if func_call_identifier_opt.is_none() {
        log::error!("[outgoing_edges] Unable to create new FunctionCallIdentifier");
        return;
    }
    let mut func_call_identifier = func_call_identifier_opt.expect("Empty func_call_identifier_opt");
    let import_identifier_opt = ImportIdentifier::new();
    if import_identifier_opt.is_none() {
        log::debug!("[outgoing_edges] Unable to create import identifier");
        return;
    }
    let mut import_identifier = import_identifier_opt.expect("Empty import_identifier_opt");
    let func_def_identifier_opt = FunctionDefIdentifier::new();
    if func_def_identifier_opt.is_none() {
        log::debug!("[outgoing_edges] Unable to create func def identifier");
        return;
    }
    let mut funcdef_identifier = func_def_identifier_opt.expect("Empty func_def_identifier_opt");
    git_checkout_commit(review, review.pr_head_commit());
    process_func_calls(
        &mut import_identifier,
        &mut func_call_identifier,
        &mut funcdef_identifier,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "green").await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_calls(&mut import_identifier,
        &mut func_call_identifier,
        &mut funcdef_identifier,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "red").await;
}

async fn process_func_calls(import_identifier: &mut ImportIdentifier, func_call_identifier: &mut FunctionCallIdentifier,
    funcdef_identifier: &mut FunctionDefIdentifier,
    review: &Review, diff_graph: &DiffGraph, base_filepaths: &Vec<PathBuf>,
    graph_elems: &mut MermaidGraphElements, edge_color: &str) 
{
    for (source_filepath, src_file_hunks) in diff_graph.hunk_diff_map().file_line_map() {
        let lang_opt = detect_language(source_filepath);
        if lang_opt.is_none() {
            log::error!("[process_func_calls] Unable to determine language: {}", source_filepath);
            continue;
        }
        let mut source_file_name = source_filepath.to_owned();
        // get func calls
        if let Some(source_file) = absolute_to_relative_path(source_filepath, review) {
            source_file_name = source_file.clone();
        }
        let diff_hunks;
        if edge_color == "green" {
            diff_hunks = src_file_hunks.added_hunks();
        } else {
            diff_hunks = src_file_hunks.deleted_hunks();
        }
        log::debug!("[process_func_calls] file name: {}\n, diff_hunks: {:?}, edge: {}", &source_file_name, diff_hunks, edge_color);
        let lang = lang_opt.expect("Empty lang_opt");
        let source_file_path = Path::new(source_filepath);
        let source_file_pathbuf = source_file_path.to_path_buf(); 
        if let Some(hunk_func_calls) = func_call_identifier.
                function_calls_in_hunks(&source_file_pathbuf, &lang, diff_hunks).await {
            for (hunk_lines, func_call_output) in hunk_func_calls {
                if let Some(src_func_name) = hunk_lines.function_line() {
                    if let Some(src_func_line_number) = hunk_lines.line_number() {
                        for dest_func_call in func_call_output.function_calls() {
                            if let Some(import_filepath) = import_identifier.get_import_path_file(
                                source_filepath, &lang, dest_func_call.function_name()).await {
                                // get file
                                // get diffgraph all files and see if they contain filepath
                                let possible_diff_file_paths: Vec<&String> =  diff_graph.hunk_diff_map().all_files().into_iter()
                                    .filter(|file_path| file_path.contains(import_filepath.get_matching_import().possible_file_path())).collect();
                                log::debug!("[process_func_calls] possible_diff_file_paths = {:?}", &possible_diff_file_paths);
                                if !possible_diff_file_paths.is_empty() {
                                    for possible_diff_file_path in possible_diff_file_paths {
                                        if diff_graph.hunk_diff_map().all_files().contains(&possible_diff_file_path)
                                        {
                                            log::debug!("[process_func_calls] possible_diff_file_path ={}", &possible_diff_file_path);
                                            if let Some(possible_file_rel) = absolute_to_relative_path(possible_diff_file_path, review) {
                                                let hunks_for_func = diff_graph.hunk_diff_map().file_line_map()
                                                    .get(possible_diff_file_path).expect("Empty entry in file_line_map");
                                                if let Some(dest_func_def_line) = hunks_for_func.is_func_in_hunks(dest_func_call.function_name(), edge_color) {
                                                    graph_elems.add_edge(
                                                        edge_color,
                                                        dest_func_call.line_number().to_owned() as usize,
                                                        src_func_name, 
                                                        dest_func_call.function_name(), 
                                                        &source_file_name,
                                                        &possible_file_rel,
                                                        "yellow",
                                                        "",
                                                        src_func_line_number,
                                                        dest_func_def_line);
                                                }                                            
                                            }
                                        }
                                    }                                
                                } else {
                                    // search all files
                                    // TODO - see if git checkout is needed
                                    let possible_file_pathbufs: Vec<&PathBuf> = base_filepaths.iter()
                                        .filter(|file_path| 
                                            file_path.to_string_lossy().contains(import_filepath.get_matching_import().possible_file_path())).collect();
                                    log::debug!("[process_func_calls] possible_file_pathbufs = {:?}", &possible_file_pathbufs);
                                    if !possible_file_pathbufs.is_empty() {
                                        for possible_file_pathbuf in possible_file_pathbufs {
                                            let possible_file_path: String = possible_file_pathbuf.to_string_lossy().to_string();
                                            if let Some(possible_file_rel) = 
                                                                absolute_to_relative_path(&possible_file_path, review) {
                                                // search only for func def with specific name
                                                // if something comes up, add edge!
                                                if let Some(func_def) = funcdef_identifier.function_defs_in_file(
                                                    possible_file_pathbuf, &lang, dest_func_call.function_name()).await {
                                                        log::debug!("[process_func_calls] func_def ={:#?}", &func_def);
                                                    if let Some(dest_func_def_line) = func_def.get_function_line_number() {
                                                        graph_elems.add_edge(
                                                            edge_color,
                                                            dest_func_call.line_number().to_owned() as usize,
                                                            src_func_name, 
                                                            dest_func_call.function_name(), 
                                                            &source_file_name,
                                                            &possible_file_rel,
                                                            "yellow",
                                                            "",
                                                            src_func_line_number,
                                                            &dest_func_def_line);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }    
                        }
                    }
                }
            }
        }    
    }
    // get import and path
}

async fn process_func_defs(review: &Review,
    diff_graph: &DiffGraph, func_call_identifier: &mut FunctionCallIdentifier,
    graph_elems: &mut MermaidGraphElements, edge_color: &str) 
{
    for (dest_filename, dest_file_hunks) in diff_graph.hunk_diff_map().file_line_map() {
        let dest_lang_opt = detect_language(&dest_filename);
        if dest_lang_opt.is_none() {
            log::error!("[process_func_defs] Unable to detect language: {}", dest_filename);
            continue;
        }
        let dest_lang = dest_lang_opt.expect("Empty dest_lang_opt");
        let func_defs;
        if edge_color == "red" {
            func_defs = dest_file_hunks.deleted_hunks();
        } else {
            func_defs = dest_file_hunks.added_hunks();
        }
        let mut repeated_funcs = HashSet::<String>::new();
        for dest_func in func_defs {
            if let Some(dest_func_name) = dest_func.function_name() {
                if repeated_funcs.get(dest_func_name).is_some()  {
                    continue;
                } else {
                    repeated_funcs.insert(dest_func_name.to_string());
                }
                if let Some(dest_funcdef_line) = dest_func.line_number() {
                    if let Some(possible_filepaths) = 
                    function_calls_search(review, dest_func_name, &dest_lang)
                {
                    if possible_filepaths.is_empty() {
                        log::debug!("[process_func_defs] No files detected having function call");
                        continue;
                    }
                    for possible_filepath in possible_filepaths {
                        if possible_filepath == *dest_filename {
                            continue;
                        }
                        let lang_opt = detect_language(&possible_filepath);
                        if lang_opt.is_none() {
                            log::debug!("[process_func_defs] Unable to determine language: {}", &possible_filepath);
                            continue;
                        }
                        let lang = lang_opt.expect("Empty lang_opt");
                        if lang != dest_lang {
                            log::debug!("[process_func_defs] Different languages: {}, {}", &lang, &dest_lang);
                            continue;
                        }
                        let possible_path = Path::new(&possible_filepath);
                        let possible_pathbuf = possible_path.to_path_buf();
                        // get func call
                        if let Some(func_calls) = func_call_identifier.functions_in_file(&possible_pathbuf, &lang).await {
                            // get func def
                            for func_call in func_calls.function_calls() {
                                if let Some(src_func_def) = get_function_def_for_func_call(
                                    &possible_pathbuf, func_call.line_number().to_owned() as usize
                                ).await {
                                    if let Some(source_filename) = absolute_to_relative_path(&possible_filepath, review) {
                                        // add edge
                                        let mut dest_file_rel = dest_filename.to_string();
                                        if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
                                            dest_file_rel = dest_file_relative_path;
                                        }
                                        graph_elems.add_edge(edge_color,
                                        func_call.line_number().to_owned() as usize,
                                        src_func_def.name(),
                                        dest_func_name,
                                        &source_filename,
                                        &dest_file_rel,
                                        "",
                                        "yellow",
                                        src_func_def.line_start(),
                                        dest_funcdef_line);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }   
}
}