use std::{path::{Path, PathBuf}, str::FromStr};
use crate::utils::{gitops::git_checkout_commit, review::Review};

use super::{elements::MermaidGraphElements, file_imports::{AllFileImportInfo, ImportPath}, function_call::function_calls_in_file, function_line_range::{generate_function_map, FuncDefInfo, FunctionFileMap}, graph_info::DiffGraph, utils::match_overlap};

pub async fn graph_edges(review: &Review, all_import_info: &AllFileImportInfo, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    incoming_edges(review, all_import_info, diff_graph, graph_elems).await;
    outgoing_edges(all_import_info, diff_graph, graph_elems).await;
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
                                    graph_elems.add_edge("green",
                                        line_num.to_owned(), 
                                        &source_func_def.name(), 
                                        &dest_func.name(),
                                        &source_filename,
                                        dest_filename);
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
                                    graph_elems.add_edge("green",
                                        line_num.to_owned(), 
                                        &source_func_def.name(), 
                                        &dest_func.name(),
                                        &source_filename,
                                        dest_filename);
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
                                    graph_elems.add_edge("red",
                                        line_num.to_owned(), 
                                        &source_func_def.name(), 
                                        &dest_func.name(),
                                        &source_filename,
                                        dest_filename);
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
                                        dest_filename);
                                }
                            }
                        }
                    }
                }
            } 
        }
    }
}

// async fn generate_incoming_edges(modified_funcs: &HashMap<String, Vec<FuncDefInfo>>, full_graph: &GraphInfo, diff_graph: &GraphInfo, color: &str, graph_elems: &mut MermaidGraphElements) {
//     for (dest_filename, dest_func_info_vec) in modified_funcs.iter() {
//         for dest_func_info in dest_func_info_vec {
//             search_imports_in_graph(&dest_filename, dest_func_info,
//                 full_graph, color, graph_elems).await;
//             search_imports_in_graph(&dest_filename, dest_func_info,
//                 diff_graph, color, graph_elems).await;
//         }
//     }
// }

// async fn search_imports_in_graph(dest_filename: &str, dest_func_info: &FuncDefInfo, search_graph: &GraphInfo, color: &str, graph_elems: &mut MermaidGraphElements) {
//     for source_filename in search_graph.import_info().files() {
//         if let Some(source_file_imports) = search_graph.import_info().file_import_info(source_filename) {
//             let file_imports = source_file_imports.all_import_paths();
//             for import_obj in file_imports {
//                 if match_import_condition(dest_filename, &import_obj, dest_func_info) {
//                     if let Some(source_func_file_map) = search_graph.function_info().functions_in_file(source_filename) {
//                         add_edge_for_file(source_filename, source_func_file_map, dest_filename, dest_func_info, color, graph_elems).await;
//                     }
//                 }
//             }
//         }
//     }
// }

fn match_import_condition(dest_filename: &str, import_obj: &ImportPath, dest_func_info: &FuncDefInfo) -> bool {
    match_overlap(
        &dest_filename,
        &import_obj.import_path(),
        0.5)
        && match_overlap(&dest_func_info.name(),
        &import_obj.imported(),
        0.5)
}

async fn add_edge_for_file(source_filename: &str, source_func_def: &FuncDefInfo, dest_filename: &str, dest_func_info: &FuncDefInfo, color: &str, graph_elems: &mut MermaidGraphElements) {
    // TODO FIXME - do git commit checkout
    let filepath = Path::new(source_filename);
    let file_pathbuf = filepath.to_path_buf();
    if let Some(func_call_chunk) = 
        function_calls_in_file(&file_pathbuf, &dest_func_info.name()).await 
    {
        for source_chunk_call in func_call_chunk {
            for source_func_line in source_chunk_call.function_calls() {
                if source_func_def != dest_func_info {
                    graph_elems.add_edge(color,
                        source_func_line.to_owned(), 
                        &source_func_def.name(), 
                        &dest_func_info.name(),
                        &source_filename,
                        dest_filename);
                }
            }
        }
    }
}

async fn outgoing_edges(all_import_info: &AllFileImportInfo, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    // TODO - git checkout
    for (source_filename, func_calls) in diff_graph.diff_func_calls() {
        for source_func_call in func_calls.added_calls() {
            let dest_filename = source_func_call.import_info().import_path();
            let func_name = source_func_call.import_info().imported();
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
                                dest_filename);
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
                                    dest_filename);
                            }
                        }
                    }
                }
            }
        }
        // do same for deleted_calls
        for source_func_call in func_calls.deleted_calls() {
            let dest_filename = source_func_call.import_info().import_path();
            let func_name = source_func_call.import_info().imported();
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
                                dest_filename);
                        }
                        // add_edge_for_file(source_filename, _, 
                        //     dest_filename, dest_func_def, "red", graph_elems).await;
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
                                    dest_filename);
                            }
                            // add_edge_for_file(source_filename, _, 
                            //     dest_filename, dest_func_def, "red", graph_elems).await;
                        }
                    }
                }
            }
        }
    }
}

// async fn generate_outgoing_edges(modified_imports: &HashMap<String, Vec<ImportPath>>, full_graph: &GraphInfo, diff_graph: &GraphInfo, color: &str, graph_elems: &mut MermaidGraphElements) {
//     for (dest_filename, dest_import_info) in modified_imports.iter() {
//         let filepath = Path::new(dest_filename);
//         let file_pathbuf = filepath.to_path_buf();
//         for dest_import in dest_import_info {
//             search_funcs_in_graph(full_graph, dest_import, &file_pathbuf, color, dest_filename, graph_elems).await;
//             // TODO FIXME - think about similar edges being searched from both full and diff graph. How to avoid adding them repeatedly?
//             search_funcs_in_graph(diff_graph, dest_import, &file_pathbuf, color, dest_filename, graph_elems).await;
//         }
//     }
// }

// async fn search_funcs_in_graph(search_graph: &GraphInfo, dest_import: &ImportPath, file_pathbuf: &PathBuf, color: &str, dest_file: &str, graph_elems: &mut MermaidGraphElements) {
//     for source_file in search_graph.function_info().all_files() {
//         if match_overlap(&source_file, &dest_import.imported(), 0.5) {
//             if let Some(source_file_func_calls) = 
//                 function_calls_in_file(&file_pathbuf, &dest_import.imported()).await
//             {
//                 if let Some(func_file_map) = 
//                         search_graph.function_info().functions_in_file(source_file) 
//                 {
//                     for func_call_chunk in source_file_func_calls {
//                         for source_file_line in func_call_chunk.function_calls() {
//                             if let Some(source_func_def) = func_file_map.func_at_line(source_file_line.to_owned()) {
//                                 if source_func_def.name() != dest_import.imported() {
//                                     graph_elems.add_edge(color, source_file_line.to_owned(), &source_func_def.name(), &dest_import.imported(), source_file, dest_file)
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

async fn edge_nodes() {
    // render all edges and their nodes
}