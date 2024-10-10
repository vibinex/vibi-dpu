use std::path::{Path, PathBuf};
use crate::utils::{gitops::git_checkout_commit, review::Review};

use super::{elements::MermaidGraphElements, file_imports::ImportIdentifier, function_call::{function_calls_search, FunctionCallIdentifier}, function_line_range::generate_function_map, graph_info::DiffGraph, utils::absolute_to_relative_path};

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
        lang,
        graph_elems,
        "green"
    ).await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_defs(
        review,
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
    let import_identifier_opt = ImportIdentifier::new();
    if import_identifier_opt.is_none() {
        log::debug!("[outgoing_edges] Unable to create import identifier");
        return;
    }
    let mut import_identifier = import_identifier_opt.expect("EMpty import_identifier_opt");
    git_checkout_commit(review, review.pr_head_commit());
    process_func_calls(
        &mut import_identifier,
        lang,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "green").await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_calls(&mut import_identifier,
        lang,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "red").await;
    
        // get func def

        // for source_func_call in func_calls.added_calls() {
        //     log::debug!("[outgoing_edges] source func call import info = {:#?}", source_func_call.import_info());
        //     // todo fixme - normalize dest_filename
        //     let dest_filename = source_func_call.import_info().import_path();
        //     let lines = source_func_call.call_info().iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
        //     // send this file for getting func defs
        //     // search in diff graph
        //     let diff_file_funcdefs = diff_graph.all_file_func_defs();
        //     // identify this particular func
        //     if let Some(func_defs) = diff_file_funcdefs.functions_in_file(dest_filename) {
        //         let source_func_defs = func_defs.funcs_for_lines(&lines);
        //         for dest_func_def in func_defs.functions() {
        //             if match_import_func( source_func_call.import_info(), dest_func_def) {
        //                 // add edge
        //                 log::debug!("[outgoing_edges] Adding edge");
        //                 for (line_num, source_func_def) in &source_func_defs {
        //                     graph_elems.add_edge("green",
        //                         line_num.to_owned(), 
        //                         source_func_def.name(), 
        //                         dest_func_def.name(),
        //                         &source_file_name,
        //                         dest_filename,
        //                         "green",
        //                         "",
        //                         source_func_def.line_start(),
        //                         dest_func_def.line_start()
        //                     );
        //                 }
        //             }
        //         }
        //     }
        //     // search in full graph
        //     let dest_filepath_opt = match_imported_filename_to_path(base_filepaths, dest_filename);
        //     if dest_filepath_opt.is_none() {
        //         log::error!("[outgoing_edges] Unable to find filename in all paths: {}", dest_filename);
        //         continue;
        //     }
        //     let dest_filepath = dest_filepath_opt.expect("EMpty dest_filepath_opt");
        //     if let Some(all_file_funcdefs) = generate_function_map(&vec![dest_filepath.clone()]).await {
        //         // identify this particular func
        //         let dest_filepath_key = dest_filepath.as_os_str().to_str().expect("Unable to deserialize dest_filepath");
        //         let mut dest_file_rel = dest_filepath_key.to_string();
        //         if let Some(dest_relative_filepath) = absolute_to_relative_path(&dest_file_rel, review) {
        //             dest_file_rel = dest_relative_filepath;
        //         }
        //         if let Some(func_defs) = all_file_funcdefs.functions_in_file(dest_filepath_key) {
        //             let source_func_defs = func_defs.funcs_for_lines(&lines);
        //             for dest_func_def in func_defs.functions() {
        //                 if match_import_func(source_func_call.import_info(), dest_func_def) {
        //                     // add edge
        //                     for (line_num, source_func_def) in &source_func_defs {
        //                         graph_elems.add_edge("green",
        //                             line_num.to_owned(), 
        //                             source_func_def.name(), 
        //                             dest_func_def.name(),
        //                             &source_file_name,
        //                             &dest_file_rel,
        //                             "green",
        //                             "",
        //                             source_func_def.line_start(),
        //                             dest_func_def.line_start()
        //                         );
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
        // // do same for deleted_calls
        // for source_func_call in func_calls.deleted_calls() {
        //     log::debug!("[outgoing_edges] source func call import info = {:#?}", source_func_call.import_info());
        //     // todo fixme - normalize dest_filename
        //     let dest_filename = source_func_call.import_info().import_path();
        //     let diff_file_funcdefs = diff_graph.all_file_func_defs();
        //     let lines = source_func_call.call_info().iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
        //     // identify this particular func
        //     if let Some(func_defs) = diff_file_funcdefs.functions_in_file(dest_filename) {
        //         let source_func_defs = func_defs.funcs_for_lines(&lines);
        //         for dest_func_def in func_defs.functions() {
        //             if match_import_func(source_func_call.import_info(), dest_func_def) {
        //                 // add edge
        //                 for (line_num, source_func_def) in &source_func_defs {
        //                     graph_elems.add_edge("red",
        //                         line_num.to_owned(), 
        //                         source_func_def.name(), 
        //                         dest_func_def.name(),
        //                         &source_file_name,
        //                         dest_filename,
        //                         "red",
        //                         "",
        //                         source_func_def.line_start(),
        //                         dest_func_def.line_start()
        //                     );
        //                 }
        //             }
        //         }
        //     }
        //     // send this file for getting func defs
        //     let dest_filepath_opt = match_imported_filename_to_path(base_filepaths, dest_filename);
        //     if dest_filepath_opt.is_none() {
        //         log::error!("[outgoing_edges] Unable to find filename in all paths: {}", dest_filename);
        //         continue;
        //     }
        //     let dest_filepath = dest_filepath_opt.expect("EMpty dest_filepath_opt");
        //     if let Some(all_file_funcdefs) = generate_function_map(&vec![dest_filepath.clone()]).await {
        //         // identify this particular func
        //         if let Some(src_file_funcs) = diff_graph.all_file_func_defs().functions_in_file(source_filepath) {
        //             let dest_filepath_key = dest_filepath.as_os_str().to_str().expect("Unable to deserialize dest_filepath");
        //             if let Some(dest_func_defs) = all_file_funcdefs.functions_in_file(dest_filepath_key) {
        //                 let mut rel_dest_filepath = dest_filepath_key.to_string();
        //                 if let Some(dest_file) =  absolute_to_relative_path(dest_filepath_key, review){
        //                     rel_dest_filepath = dest_file.clone();
        //                 }
        //             // TODO FIXME - func_defs is for dest, we need it for src file, check other places as well to fix this
        //                 let source_func_defs = src_file_funcs.funcs_for_lines(&lines);
        //                 log::debug!("[outgoing_edges] lines = {:?}, source_func_defs = {:#?} dest_func_defs = {:#?}", &lines, &source_func_defs, &dest_func_defs);
        //                 for dest_func_def in dest_func_defs.functions() {
        //                     if match_import_func(source_func_call.import_info(), dest_func_def) {
        //                         // add edge
        //                         for (line_num, source_func_def) in &source_func_defs {
        //                             log::debug!("[outgoing_edges] Adding edge for deleted func in full_graph");
        //                             graph_elems.add_edge("red",
        //                                 line_num.to_owned(), 
        //                                 source_func_def.name(), 
        //                                 dest_func_def.name(),
        //                                 &source_file_name,
        //                                 &rel_dest_filepath,
        //                                 "red",
        //                                 "",
        //                                 source_func_def.line_start(),
        //                                 dest_func_def.line_start()
        //                             );
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
    // }
}

async fn process_func_calls(import_identifier: &mut ImportIdentifier, lang: &str,
    review: &Review, diff_graph: &DiffGraph, base_filepaths: &Vec<PathBuf>,
    graph_elems: &mut MermaidGraphElements, edge_color: &str) 
{
    for (source_filepath, diff_func_calls) in diff_graph.diff_func_calls() {
        let mut source_file_name = source_filepath.to_owned();
        // get func calls
        if let Some(source_file) = absolute_to_relative_path(source_filepath, review) {
            source_file_name = source_file.clone();
        }
        let func_calls;
        if edge_color == "green" {
            func_calls = diff_func_calls.added_calls();
        } else {
            func_calls = diff_func_calls.deleted_calls();
        }
        for dest_func_call in func_calls.function_calls() {
            if let Some(import_filepath) = import_identifier.get_import_path_file(
                source_filepath, lang, dest_func_call.function_name()).await {
                // get file
                    // get diffgraph all files and see if they contain filepath
                    let possible_diff_file_paths: Vec<&String> =  diff_graph.all_file_func_defs().all_files().into_iter()
                        .filter(|file_path| file_path.contains(import_filepath.get_matching_import().possible_file_path())).collect();
                    if possible_diff_file_paths.is_empty() {
                        // get all filepaths base or head or both and see contains among them
                        let possible_file_pathbufs: Vec<&PathBuf> = base_filepaths.iter()
                            .filter(|file_path| 
                                file_path.to_string_lossy().contains(import_filepath.get_matching_import().possible_file_path())).collect();
                        if !possible_file_pathbufs.is_empty() {
                            for possible_file_pathbuf in possible_file_pathbufs {
                                if let Some(func_defs) = diff_graph.all_file_func_defs()
                                        .functions_in_file(&possible_file_pathbuf.to_string_lossy()) 
                                {
                                    for dest_func_def in func_defs.functions() {
                                        if dest_func_def.name().contains(dest_func_call.function_name()) {
                                            // find src func def
                                            if let Some(file_func_map) = diff_graph.all_file_func_defs().functions_in_file(source_filepath) {
                                                if let Some(src_func_def) = file_func_map.funcs_for_func_call(dest_func_call) {
                                                    // TODO - recheck colors logic
                                                    graph_elems.add_edge(
                                                        edge_color,
                                                        dest_func_call.line_number().to_owned() as usize,
                                                        src_func_def.name(), 
                                                        dest_func_call.function_name(), 
                                                        &source_file_name,
                                                        &possible_file_pathbuf.to_string_lossy(),
                                                        edge_color,
                                                        "",
                                                        src_func_def.line_start(),
                                                        dest_func_def.line_start());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
    
                    } else {
                        // get file func defs
                        for possible_file_path in possible_diff_file_paths {
                            if let Some(func_defs) = diff_graph.all_file_func_defs()
                                    .functions_in_file(possible_file_path) {
                                let possible_file_name;
                                let possible_file_name_opt = absolute_to_relative_path(possible_file_path, review);
                                if possible_file_name_opt.is_none() {
                                    possible_file_name = possible_file_path.to_string();
                                } else {
                                    possible_file_name = possible_file_name_opt.expect("Empty possible_file_name_opt");
                                }

                                for dest_func_def in func_defs.functions() {
                                    if dest_func_def.name().contains(dest_func_call.function_name()) {
                                        // TODO - add edge
                                        if let Some(file_func_map) = diff_graph.all_file_func_defs().functions_in_file(source_filepath) {
                                            if let Some(src_func_def) = file_func_map.funcs_for_func_call(dest_func_call) {
                                                // TODO - recheck colors logic
                                                graph_elems.add_edge(
                                                    edge_color,
                                                    dest_func_call.line_number().to_owned() as usize,
                                                    src_func_def.name(), 
                                                    dest_func_call.function_name(), 
                                                    &source_file_name,
                                                    &possible_file_name,
                                                    edge_color,
                                                    "",
                                                    src_func_def.line_start(),
                                                    dest_func_def.line_start());
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
    lang: &str, graph_elems: &mut MermaidGraphElements, edge_color: &str) 
{
    for (dest_filename, diff_func_defs) in diff_graph.diff_func_defs() {
        let mut dest_file_rel = dest_filename.to_string();
        if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
            dest_file_rel = dest_file_relative_path;
        }
        let func_defs;
        if edge_color == "red" {
            func_defs = diff_func_defs.deleted_func_defs();
        } else {
            func_defs = diff_func_defs.added_func_defs();
        }
        for dest_func in func_defs {
            // filter files with ripgrep
            if let Some(possible_filepaths) = function_calls_search(review, dest_func.func_def().name()) {
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
                        if let Some(func_map) = generate_function_map(&vec![possible_pathbuf]).await {
                            for func_call in func_calls.function_calls() {
                                if let Some(func_file_map) = func_map.functions_in_file(&possible_filepath) {
                                    // find correct func def
                                    if let Some(src_func_def) = func_file_map.funcs_for_func_call(func_call) {
                                        if let Some(source_filename) = absolute_to_relative_path(&possible_filepath, review) {
                                            // add edge
                                            
                                            graph_elems.add_edge(edge_color,
                                            func_call.line_number().to_owned() as usize,
                                            func_call.function_name(),
                                            dest_func.func_def().name(),
                                            &source_filename,
                                            &dest_file_rel,
                                            "",
                                            edge_color,
                                            src_func_def.line_start(),
                                            dest_func.func_def().line_start());
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