use std::{path::{Path, PathBuf}, str::FromStr};
use crate::utils::{gitops::git_checkout_commit, review::Review};

use super::{elements::MermaidGraphElements, file_imports::{AllFileImportInfo, ImportPath}, function_call::function_calls_in_file, function_line_range::{generate_function_map, FuncDefInfo, FunctionFileMap}, graph_info::DiffGraph, utils::match_overlap};

pub async fn graph_edges(review: &Review, all_import_info: &AllFileImportInfo, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    outgoing_edges(diff_graph, graph_elems).await;
    incoming_edges(review, all_import_info, diff_graph, graph_elems).await;
}

async fn incoming_edges(review: &Review, all_import_info: &AllFileImportInfo, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    for (dest_filename, func_defs) in diff_graph.diff_func_defs() {
        for dest_func in func_defs.added_func_defs() {
            git_checkout_commit(review, review.pr_head_commit());
            // search in diff graph
            for (source_filename, file_func_defs) in diff_graph.all_file_imports().file_import_map() {
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if match_import_condition(dest_filename, &file_import, dest_func) {
                        // find func call
                        let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
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
                                        &source_filename,
                                        dest_filename,
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
            git_checkout_commit(review, review.base_head_commit());
            // search in full graph
            for (source_filename, file_func_defs) in all_import_info.file_import_map() {
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if match_import_condition(dest_filename, &file_import, dest_func) {
                        // if found, create edge
                        let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
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
                                        &source_filename,
                                        dest_filename,
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
        for dest_func in func_defs.deleted_func_defs() {
            // search in diff graph
            for (source_filename, file_func_defs) in diff_graph.all_file_imports().file_import_map() {
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if match_import_condition(dest_filename, &file_import, dest_func) {
                        // find func call
                        git_checkout_commit(review, review.pr_head_commit());
                        let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
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
                                        &source_filename,
                                        dest_filename,
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
            // search in full graph
            for (source_filename, file_func_defs) in all_import_info.file_import_map() {
                let file_imports = file_func_defs.all_import_paths();
                for file_import in file_imports {
                    // search for correct import
                    if match_import_condition(dest_filename, &file_import, dest_func) {
                        // if found, create edge
                        let src_filepath = PathBuf::from_str(source_filename).expect("Unable to create pathbuf");
                        if let Some(func_call_vec) = function_calls_in_file(&src_filepath, dest_func.name()).await {
                            // call func in  that takes vec of lines and returns funcdefs
                            let lines = func_call_vec.iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
                            let source_func_defs = diff_graph.all_file_func_defs().functions_in_file(source_filename).expect("No source filename found").funcs_for_lines(&lines);
                            for (line_num, source_func_def) in source_func_defs {
                                if source_func_def != dest_func.to_owned() {
                                    graph_elems.add_edge("red",
                                        line_num.to_owned(),
                                        &source_func_def.name(), 
                                        &dest_func.name(),
                                        &source_filename,
                                        dest_filename,
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

fn match_import_condition(dest_filename: &str, import_obj: &ImportPath, dest_func_info: &FuncDefInfo) -> bool {
    match_overlap(
        &dest_filename,
        &import_obj.import_path(),
        0.5)
        && match_overlap(&dest_func_info.name(),
        &import_obj.imported(),
        0.5)
}

async fn outgoing_edges(diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    // TODO - git checkout
    for (source_filename, func_calls) in diff_graph.diff_func_calls() {
        for source_func_call in func_calls.added_calls() {
            let dest_filename = source_func_call.import_info().import_path();
            let lines = source_func_call.call_info().iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
            // send this file for getting func defs
            // search in diff graph
            let diff_file_funcdefs = diff_graph.all_file_func_defs();
            // identify this particular func
            if let Some(func_defs) = diff_file_funcdefs.functions_in_file(dest_filename) {
                let source_func_defs = func_defs.funcs_for_lines(&lines);
                for dest_func_def in func_defs.functions() {
                    if match_import_condition(dest_filename, source_func_call.import_info(), dest_func_def) {
                        // add edge
                        for (line_num, source_func_def) in &source_func_defs {
                            graph_elems.add_edge("green",
                                line_num.to_owned(), 
                                source_func_def.name(), 
                                dest_func_def.name(),
                                source_filename,
                                dest_filename,
                                "green",
                                "",
                                source_func_def.line_start(),
                                dest_func_def.line_start()
                            );
                        }
                    }
                }
            }
            // search in full graph
            let dest_filepath = PathBuf::from_str(dest_filename).expect("Unable to get path");
            if let Some(all_file_funcdefs) = generate_function_map(&vec![dest_filepath]).await {
                // identify this particular func
                if let Some(func_defs) = all_file_funcdefs.functions_in_file(dest_filename) {
                    let source_func_defs = func_defs.funcs_for_lines(&lines);
                    for dest_func_def in func_defs.functions() {
                        if match_import_condition(dest_filename, source_func_call.import_info(), dest_func_def) {
                            // add edge
                            for (line_num, source_func_def) in &source_func_defs {
                                graph_elems.add_edge("green",
                                    line_num.to_owned(), 
                                    source_func_def.name(), 
                                    dest_func_def.name(),
                                    source_filename,
                                    dest_filename,
                                    "green",
                                    "",
                                    source_func_def.line_start(),
                                    dest_func_def.line_start()
                                );
                            }
                        }
                    }
                }
            }
        }
        // do same for deleted_calls
        for source_func_call in func_calls.deleted_calls() {
            let dest_filename = source_func_call.import_info().import_path();
            let diff_file_funcdefs = diff_graph.all_file_func_defs();
            let lines = source_func_call.call_info().iter().flat_map(|chunk| chunk.function_calls()).cloned().collect();
            // identify this particular func
            if let Some(func_defs) = diff_file_funcdefs.functions_in_file(dest_filename) {
                let source_func_defs = func_defs.funcs_for_lines(&lines);
                for dest_func_def in func_defs.functions() {
                    if match_import_condition(dest_filename, source_func_call.import_info(), dest_func_def) {
                        // add edge
                        for (line_num, source_func_def) in &source_func_defs {
                            graph_elems.add_edge("red",
                                line_num.to_owned(), 
                                source_func_def.name(), 
                                dest_func_def.name(),
                                source_filename,
                                dest_filename,
                                "red",
                                "",
                                source_func_def.line_start(),
                                dest_func_def.line_start()
                            );
                        }
                    }
                }
            }
            // send this file for getting func defs
            let dest_filepath = PathBuf::from_str(dest_filename).expect("Unable to get path");
            if let Some(all_file_funcdefs) = generate_function_map(&vec![dest_filepath]).await {
                // identify this particular func
                if let Some(func_defs) = all_file_funcdefs.functions_in_file(dest_filename) {
                    let source_func_defs = func_defs.funcs_for_lines(&lines);
                    for dest_func_def in func_defs.functions() {
                        if match_import_condition(dest_filename, source_func_call.import_info(), dest_func_def) {
                            // add edge
                            for (line_num, source_func_def) in &source_func_defs {
                                graph_elems.add_edge("red",
                                    line_num.to_owned(), 
                                    source_func_def.name(), 
                                    dest_func_def.name(),
                                    source_filename,
                                    dest_filename,
                                    "red",
                                    "",
                                    source_func_def.line_start(),
                                    dest_func_def.line_start()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}