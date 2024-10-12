use std::path::{Path, PathBuf};
use crate::utils::{gitops::git_checkout_commit, review::Review};

use super::{elements::MermaidGraphElements, file_imports::ImportIdentifier, function_call::{function_calls_search, FunctionCallIdentifier}, function_line_range::{generate_function_map, get_function_def_for_func_call, FunctionDefIdentifier}, graph_info::DiffGraph, utils::absolute_to_relative_path};

pub async fn graph_edges(base_filepaths: &Vec<PathBuf>, review: &Review, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements, lang: &str) {
    outgoing_edges(base_filepaths, diff_graph, graph_elems, review, lang).await;
    incoming_edges(review, diff_graph, graph_elems, lang).await;
}

async fn incoming_edges(review: &Review, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements, lang :&str) {
    // filter files with ripgrep
    // for each filtered file
        // get func call
        // get func def
    // for (dest_filename, func_defs) in diff_graph.diff_func_defs() {
    //     let mut dest_file_rel = dest_filename.to_string();
    //     if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
    //         dest_file_rel = dest_file_relative_path;
    //     }
    //     let all_files: Vec<&String> = diff_graph.diff_func_defs().keys().collect();    
        //     for (source_filename, file_func_defs) in diff_graph.all_file_imports().file_import_map() {
        //         let mut source_rel_path = source_filename.to_string();
        //         if let Some(src_relative_filepath) = absolute_to_relative_path(&source_rel_path, review) {
        //             source_rel_path = src_relative_filepath;
        //         }
        //         let file_imports = file_func_defs.all_import_paths();
        //         for file_import in file_imports {
        //             // search for correct import
        //             if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, &file_import.import_path()) {
        //                 if match_import_func(&file_import, dest_func) {
        //                     // find func call
        //                     let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
        //                     // TODO, FIXME - function_calls_in_file should have src_filename or src_filepath? - check other calls to the function as well
        //                     if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
        //                         // call func in  that takes vec of lines and returns funcdefs
        //                         let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
        //                         let source_func_defs = diff_graph.all_file_func_defs().functions_in_file(source_filename).expect("No source filename found").funcs_for_lines(&lines);
        //                         for (line_num, source_func_def) in source_func_defs {
        //                             if source_func_def != dest_func.to_owned() {
        //                                 graph_elems.add_edge("",
        //                                     line_num.to_owned(), 
        //                                     &source_func_def.name(), 
        //                                     &dest_func.name(),
        //                                     &source_rel_path,
        //                                     &dest_file_rel,
        //                                     "",
        //                                     "green",
        //                                     source_func_def.line_start(),
        //                                     dest_func.line_start()
        //                                 );
        //                             }
        //                         }
        //                     }
        //                 }    
        //             }
        //         }
        //     }
            
        //     // search in full graph
        //     for (source_filename, file_func_defs) in all_import_info.file_import_map() {
        //         let mut source_file_rel = source_filename.to_string();
        //         if let Some(src_relative_filepath) = absolute_to_relative_path(&source_file_rel, review) {
        //             source_file_rel = src_relative_filepath;
        //         } 
        //         let file_imports = file_func_defs.all_import_paths();
        //         for file_import in file_imports {
        //             // search for correct import
        //             if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, file_import.import_path()) {
        //                 if match_import_func(&file_import, dest_func) {
        //                     // if found, create edge
        //                     let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
        //                     if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
        //                         // call func in  that takes vec of lines and returns funcdefs
        //                         let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
        //                         let source_func_defs_opt = diff_graph.all_file_func_defs().functions_in_file(source_filename);
        //                         if source_func_defs_opt.is_none() {
        //                             log::debug!("[incoming_edges] No funcs for file: {}", source_filename);
        //                             continue;
        //                         }
        //                         let source_func_defs = source_func_defs_opt.expect("No source filename found").funcs_for_lines(&lines);
        //                         for (line_num, source_func_def) in source_func_defs {
        //                             if source_func_def != dest_func.to_owned() {
        //                                 graph_elems.add_edge("",
        //                                     line_num.to_owned(), 
        //                                     &source_func_def.name(), 
        //                                     &dest_func.name(),
        //                                     &source_file_rel,
        //                                     &dest_file_rel,
        //                                     "",
        //                                     "green",
        //                                     source_func_def.line_start(),
        //                                     dest_func.line_start()
        //                                 );
        //                             }
        //                         }
        //                     }
        //                 }    
        //             }
        //         }
        //     } 
        // }
        // for dest_func in func_defs.deleted_func_defs() {
        //     // search in diff graph
        //     for (source_filename, file_func_defs) in diff_graph.all_file_imports().file_import_map() {
        //         let mut source_file_rel = source_filename.to_string();
        //         if let Some(src_relative_filepath) = absolute_to_relative_path(&source_file_rel, review) {
        //             source_file_rel = src_relative_filepath;
        //         }
        //         let file_imports = file_func_defs.all_import_paths();
        //         for file_import in file_imports {
        //             // search for correct import
        //             if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, file_import.import_path()) {
        //                 if match_import_func(&file_import, dest_func) {
        //                     // find func call
        //                     git_checkout_commit(review, review.pr_head_commit());
        //                     let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
        //                     if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
        //                         // call func in  that takes vec of lines and returns funcdefs
        //                         let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
        //                         let source_func_defs_opt = diff_graph.all_file_func_defs().functions_in_file(source_filename);
        //                         if source_func_defs_opt.is_none() {
        //                             log::debug!("[incoming_edges] No funcs for file: {}", source_filename);
        //                             continue;
        //                         }
        //                         let source_func_defs = source_func_defs_opt.expect("No source filename found").funcs_for_lines(&lines);
        //                         for (line_num, source_func_def) in source_func_defs {
        //                             if source_func_def != dest_func.to_owned() {
        //                                 graph_elems.add_edge("",
        //                                     line_num.to_owned(), 
        //                                     &source_func_def.name(), 
        //                                     &dest_func.name(),
        //                                     &source_file_rel,
        //                                     &dest_file_rel,
        //                                     "",
        //                                     "red",
        //                                     source_func_def.line_start(),
        //                                     dest_func.line_start()
        //                                 );
        //                             }
        //                         }
        //                     }
        //                 }    
        //             }
        //         }
        //     }
        //     // search in full graph
        //     for (source_filename, file_func_defs) in all_import_info.file_import_map() {
        //         let mut source_file_rel = source_filename.to_string();
        //         if let Some(src_relative_filepath) = absolute_to_relative_path(&source_file_rel, review) {
        //             source_file_rel = src_relative_filepath;
        //         }
        //         let file_imports = file_func_defs.all_import_paths();
        //         for file_import in file_imports {
        //             // search for correct import
        //             if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, file_import.import_path()) {
        //                 if match_import_func(&file_import, dest_func) {
        //                     // if found, create edge
        //                     let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
        //                     if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
        //                         // call func in  that takes vec of lines and returns funcdefs
        //                         let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
        //                         let source_func_defs_opt = diff_graph.all_file_func_defs().functions_in_file(source_filename);
        //                         if source_func_defs_opt.is_none() {
        //                             log::debug!("[incoming_edges] No funcs for file: {}", source_filename);
        //                             continue;
        //                         }
        //                         let source_func_defs = source_func_defs_opt.expect("No source filename found").funcs_for_lines(&lines);
        //                         for (line_num, source_func_def) in source_func_defs {
        //                             if source_func_def != dest_func.to_owned() {
        //                                 graph_elems.add_edge("red",
        //                                     line_num.to_owned(),
        //                                     &source_func_def.name(), 
        //                                     &dest_func.name(),
        //                                     &source_file_rel,
        //                                     &dest_file_rel,
        //                                     "",
        //                                     "red",
        //                                     source_func_def.line_start(),
        //                                     dest_func.line_start()
        //                                 );
        //                             }
        //                         }
        //                     }
        //                 }    
        //             } 
        //         }
        //     } 
        // }
    // }
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
        &mut funcdef_identifier,
        diff_graph,
        &mut func_call_identifier,
        lang,
        graph_elems,
        "green"
    ).await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_defs(
        review,
        &mut funcdef_identifier,
        diff_graph,
        &mut func_call_identifier,
        lang,
        graph_elems,
        "red"
    ).await;
}

// fn match_import_func(import_obj: &ImportPath, dest_func_info: &FuncDefInfo) -> bool {
//     log::debug!("[match_import_condition] import_obj.imported = {}, dest_func_info = {:#?}", import_obj.imported(), dest_func_info);
//     // TODO FIXME - first condition doesn't make sense, it should always be true? - have to check for all calls of this function
//     match_overlap(&dest_func_info.name(),
//         &import_obj.imported(),
//         0.6)
//         || match_overlap(&dest_func_info.parent(),
//         &import_obj.imported(),
//         0.6)
// }

async fn outgoing_edges(base_filepaths: &Vec<PathBuf>, diff_graph: &DiffGraph,
    graph_elems: &mut MermaidGraphElements, review: &Review, lang: &str) 
{
    let func_call_identifier_opt = FunctionCallIdentifier::new();
    if func_call_identifier_opt.is_none() {
        log::error!("[incoming_edges] Unable to create new FunctionCallIdentifier");
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
        lang,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "green").await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_calls(&mut import_identifier,
        &mut func_call_identifier,
        &mut funcdef_identifier,
        lang,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "red").await;
}

async fn process_func_calls(import_identifier: &mut ImportIdentifier, func_call_identifier: &mut FunctionCallIdentifier,
    funcdef_identifier: &mut FunctionDefIdentifier,
    lang: &str, review: &Review, diff_graph: &DiffGraph, base_filepaths: &Vec<PathBuf>,
    graph_elems: &mut MermaidGraphElements, edge_color: &str) 
{
    for (source_filepath, src_file_hunks) in diff_graph.hunk_diff_map().file_line_map() {
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
        let source_file_path = Path::new(source_filepath);
        let source_file_pathbuf = source_file_path.to_path_buf(); 
        if let Some(hunk_func_calls) = func_call_identifier.
                function_calls_in_hunks(&source_file_pathbuf, lang, diff_hunks).await {
            for (hunk_lines, func_call_output) in hunk_func_calls {
                for dest_func_call in func_call_output.function_calls() {
                    if let Some(import_filepath) = import_identifier.get_import_path_file(
                        source_filepath, lang, dest_func_call.function_name()).await {
                        // get file
                            // get diffgraph all files and see if they contain filepath
                            let possible_diff_file_paths: Vec<&String> =  diff_graph.hunk_diff_map().all_files().into_iter()
                                .filter(|file_path| file_path.contains(import_filepath.get_matching_import().possible_file_path())).collect();
                            if !possible_diff_file_paths.is_empty() {
                                for possible_diff_file_path in possible_diff_file_paths {
                                    if diff_graph.hunk_diff_map().all_files().contains(&possible_diff_file_path)
                                    {
                                        let hunks_for_func = diff_graph.hunk_diff_map().file_line_map()
                                            .get(possible_diff_file_path).expect("Empty entry in file_line_map");
                                        if let Some(possible_file_rel) = absolute_to_relative_path(possible_diff_file_path, review) {
                                            if let Some(dest_func_def_line) = hunks_for_func.is_func_in_hunks(dest_func_call.function_name()) {
                                                if let Some(src_func_name) = hunk_lines.function_line() {
                                                    if let Some(src_func_line_number) = hunk_lines.line_number() {
                                                        graph_elems.add_edge(
                                                            edge_color,
                                                            dest_func_call.line_number().to_owned() as usize,
                                                            src_func_name, 
                                                            dest_func_call.function_name(), 
                                                            &source_file_name,
                                                            &possible_file_rel,
                                                            edge_color,
                                                            "",
                                                            src_func_line_number,
                                                            dest_func_def_line);
                                                    }
                                                }
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
                                if !possible_file_pathbufs.is_empty() {
                                    for possible_file_pathbuf in possible_file_pathbufs {
                                        let possible_file_path: String = possible_file_pathbuf.to_string_lossy().to_string();
                                        // search only for func def with specific name
                                        // if something comes up, add edge!
                                        if let Some(func_defs) = funcdef_identifier.function_defs_in_file(
                                            possible_file_pathbuf, lang, dest_func_call.function_name()).await {
                                            if let Some(dest_func_def_line) = func_defs.get_function_line_number() {
                                                if let Some(src_func_name) = hunk_lines.function_line() {
                                                    if let Some(src_func_line_number) = hunk_lines.line_number() {
                                                        if let Some(possible_file_rel) = 
                                                            absolute_to_relative_path(&possible_file_path, review) {
                                                                graph_elems.add_edge(
                                                                    edge_color,
                                                                    dest_func_call.line_number().to_owned() as usize,
                                                                    src_func_name, 
                                                                    dest_func_call.function_name(), 
                                                                    &source_file_name,
                                                                    &possible_file_rel,
                                                                    edge_color,
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

async fn process_func_defs(review: &Review, funcdef_identifier: &mut FunctionDefIdentifier,
    diff_graph: &DiffGraph, func_call_identifier: &mut FunctionCallIdentifier,
    lang: &str, graph_elems: &mut MermaidGraphElements, edge_color: &str) 
{
    for (dest_filename, dest_file_hunks) in diff_graph.hunk_diff_map().file_line_map() {
        let func_defs;
        if edge_color == "red" {
            func_defs = dest_file_hunks.deleted_hunks();
        } else {
            func_defs = dest_file_hunks.added_hunks();
        }
        for dest_func in func_defs {
            if let Some(dest_func_name) = dest_func.function_line() {
                if let Some(dest_funcdef_line) = dest_func.line_number() {
                    if let Some(possible_filepaths) = 
                    function_calls_search(review, dest_func_name) 
                {
                    if possible_filepaths.is_empty() {
                        log::debug!("[incoming_edges] No files detected having function call");
                        continue;
                    }
                    for possible_filepath in possible_filepaths {
                        if possible_filepath == *dest_filename {
                            continue;
                        }
                        let possible_path = Path::new(&possible_filepath);
                        let possible_pathbuf = possible_path.to_path_buf();
                        // get func call
                        if let Some(func_calls) = func_call_identifier.functions_in_file(&possible_pathbuf, lang).await {
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
                                        edge_color,
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