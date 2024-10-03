use std::{path::{Path, PathBuf}, str::FromStr};
use crate::{graph::utils::match_imported_filename_to_path, utils::{gitops::git_checkout_commit, review::Review}};

use super::{elements::MermaidGraphElements, file_imports::{FilesImportInfo, ImportPath}, function_call::function_calls_in_file, function_line_range::{generate_function_map, FuncDefInfo, FunctionFileMap}, graph_info::DiffGraph, utils::{absolute_to_relative_path, match_overlap}};

pub async fn graph_edges(base_filepaths: &Vec<PathBuf>, head_filepaths: &Vec<PathBuf>, review: &Review, all_import_info: &FilesImportInfo, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    outgoing_edges(base_filepaths, head_filepaths, diff_graph, graph_elems, review).await;
    incoming_edges(head_filepaths, review, all_import_info, diff_graph, graph_elems).await;
}

async fn incoming_edges(head_filepaths: &Vec<PathBuf>, review: &Review, all_import_info: &FilesImportInfo, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    // filter files with ripgrep
    // for each filtered file
        // get func call
        // get func def
    for (dest_filename, func_defs) in diff_graph.diff_func_defs() {
        let mut dest_file_rel = dest_filename.to_string();
        if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
            dest_file_rel = dest_file_relative_path;
        }
        for dest_func in func_defs.added_func_defs() {
            git_checkout_commit(review, review.pr_head_commit());
            // search in diff graph
            for (source_filename, file_func_defs) in diff_graph.all_file_imports().file_import_map() {
                let mut source_rel_path = source_filename.to_string();
                if let Some(src_relative_filepath) = absolute_to_relative_path(&source_rel_path, review) {
                    source_rel_path = src_relative_filepath;
                }
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, &file_import.import_path()) {
                        if match_import_func(&file_import, dest_func) {
                            // find func call
                            let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
                            // TODO, FIXME - function_calls_in_file should have src_filename or src_filepath? - check other calls to the function as well
                            if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
                                // call func in  that takes vec of lines and returns funcdefs
                                let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
                                let source_func_defs = diff_graph.all_file_func_defs().functions_in_file(source_filename).expect("No source filename found").funcs_for_lines(&lines);
                                for (line_num, source_func_def) in source_func_defs {
                                    if source_func_def != dest_func.to_owned() {
                                        graph_elems.add_edge("",
                                            line_num.to_owned(), 
                                            &source_func_def.name(), 
                                            &dest_func.name(),
                                            &source_rel_path,
                                            &dest_file_rel,
                                            "",
                                            "green",
                                            source_func_def.line_start(),
                                            dest_func.line_start()
                                        );
                                    }
                                }
                            }
                        }    
                    }
                }
            }
            git_checkout_commit(review, review.base_head_commit());
            // search in full graph
            for (source_filename, file_func_defs) in all_import_info.file_import_map() {
                let mut source_file_rel = source_filename.to_string();
                if let Some(src_relative_filepath) = absolute_to_relative_path(&source_file_rel, review) {
                    source_file_rel = src_relative_filepath;
                } 
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, file_import.import_path()) {
                        if match_import_func(&file_import, dest_func) {
                            // if found, create edge
                            let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
                            if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
                                // call func in  that takes vec of lines and returns funcdefs
                                let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
                                let source_func_defs_opt = diff_graph.all_file_func_defs().functions_in_file(source_filename);
                                if source_func_defs_opt.is_none() {
                                    log::debug!("[incoming_edges] No funcs for file: {}", source_filename);
                                    continue;
                                }
                                let source_func_defs = source_func_defs_opt.expect("No source filename found").funcs_for_lines(&lines);
                                for (line_num, source_func_def) in source_func_defs {
                                    if source_func_def != dest_func.to_owned() {
                                        graph_elems.add_edge("",
                                            line_num.to_owned(), 
                                            &source_func_def.name(), 
                                            &dest_func.name(),
                                            &source_file_rel,
                                            &dest_file_rel,
                                            "",
                                            "green",
                                            source_func_def.line_start(),
                                            dest_func.line_start()
                                        );
                                    }
                                }
                            }
                        }    
                    }
                }
            } 
        }
        for dest_func in func_defs.deleted_func_defs() {
            // search in diff graph
            for (source_filename, file_func_defs) in diff_graph.all_file_imports().file_import_map() {
                let mut source_file_rel = source_filename.to_string();
                if let Some(src_relative_filepath) = absolute_to_relative_path(&source_file_rel, review) {
                    source_file_rel = src_relative_filepath;
                }
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, file_import.import_path()) {
                        if match_import_func(&file_import, dest_func) {
                            // find func call
                            git_checkout_commit(review, review.pr_head_commit());
                            let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
                            if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
                                // call func in  that takes vec of lines and returns funcdefs
                                let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
                                let source_func_defs_opt = diff_graph.all_file_func_defs().functions_in_file(source_filename);
                                if source_func_defs_opt.is_none() {
                                    log::debug!("[incoming_edges] No funcs for file: {}", source_filename);
                                    continue;
                                }
                                let source_func_defs = source_func_defs_opt.expect("No source filename found").funcs_for_lines(&lines);
                                for (line_num, source_func_def) in source_func_defs {
                                    if source_func_def != dest_func.to_owned() {
                                        graph_elems.add_edge("",
                                            line_num.to_owned(), 
                                            &source_func_def.name(), 
                                            &dest_func.name(),
                                            &source_file_rel,
                                            &dest_file_rel,
                                            "",
                                            "red",
                                            source_func_def.line_start(),
                                            dest_func.line_start()
                                        );
                                    }
                                }
                            }
                        }    
                    }
                }
            }
            // search in full graph
            for (source_filename, file_func_defs) in all_import_info.file_import_map() {
                let mut source_file_rel = source_filename.to_string();
                if let Some(src_relative_filepath) = absolute_to_relative_path(&source_file_rel, review) {
                    source_file_rel = src_relative_filepath;
                }
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if let Some(dest_filepath) = match_imported_filename_to_path(head_filepaths, file_import.import_path()) {
                        if match_import_func(&file_import, dest_func) {
                            // if found, create edge
                            let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
                            if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
                                // call func in  that takes vec of lines and returns funcdefs
                                let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
                                let source_func_defs_opt = diff_graph.all_file_func_defs().functions_in_file(source_filename);
                                if source_func_defs_opt.is_none() {
                                    log::debug!("[incoming_edges] No funcs for file: {}", source_filename);
                                    continue;
                                }
                                let source_func_defs = source_func_defs_opt.expect("No source filename found").funcs_for_lines(&lines);
                                for (line_num, source_func_def) in source_func_defs {
                                    if source_func_def != dest_func.to_owned() {
                                        graph_elems.add_edge("red",
                                            line_num.to_owned(),
                                            &source_func_def.name(), 
                                            &dest_func.name(),
                                            &source_file_rel,
                                            &dest_file_rel,
                                            "",
                                            "red",
                                            source_func_def.line_start(),
                                            dest_func.line_start()
                                        );
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

fn match_import_func(import_obj: &ImportPath, dest_func_info: &FuncDefInfo) -> bool {
    log::debug!("[match_import_condition] import_obj.imported = {}, dest_func_info = {:#?}", import_obj.imported(), dest_func_info);
    // TODO FIXME - first condition doesn't make sense, it should always be true? - have to check for all calls of this function
    match_overlap(&dest_func_info.name(),
        &import_obj.imported(),
        0.6)
        || match_overlap(&dest_func_info.parent(),
        &import_obj.imported(),
        0.6)
}

async fn outgoing_edges(base_filepaths: &Vec<PathBuf>, head_filepaths: &Vec<PathBuf>, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements, review: &Review) {
    git_checkout_commit(review, review.base_head_commit());
    for (source_filepath, func_calls) in diff_graph.diff_func_calls() {
        let mut source_file_name = source_filepath.to_owned();
        if let Some(source_file) =  absolute_to_relative_path(source_filepath, review){
            source_file_name = source_file.clone();
        }

        // get func calls
        // get import and path
        // get file
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
    }
}