use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}};
use crate::{graph::{function_call::associate_function_calls, function_name::FunctionDefinition}, utils::{gitops::git_checkout_commit, review::Review}};

use super::{elements::MermaidGraphElements, file_imports::{ImportDefIdentifier, ImportIdentifier, ImportLinesIdentifier}, function_call::{function_calls_search, FunctionCall, FunctionCallIdentifier, FunctionCallValidator}, function_line_range::{get_function_def_for_func_call, FunctionDefIdentifier}, graph_info::DiffGraph, utils::{absolute_to_relative_path, detect_language}};

pub async fn graph_edges(base_filepaths: &Vec<PathBuf>, review: &Review, diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements) {
    let func_call_identifier_opt = FunctionCallIdentifier::new();
    if func_call_identifier_opt.is_none() {
        log::error!("[graph_edges] Unable to create new FunctionCallIdentifier");
        return;
    }
    let mut func_call_identifier = func_call_identifier_opt.expect("Empty func_call_identifier_opt");
    let import_identifier_opt = ImportIdentifier::new();
    if import_identifier_opt.is_none() {
        log::debug!("[graph_edges] Unable to create import identifier");
        return;
    }
    let mut import_identifier = import_identifier_opt.expect("Empty import_identifier_opt");
    let func_def_identifier_opt = FunctionDefIdentifier::new();
    if func_def_identifier_opt.is_none() {
        log::debug!("[graph_edges] Unable to create func def identifier");
        return;
    }
    let mut funcdef_identifier = func_def_identifier_opt.expect("Empty func_def_identifier_opt");
    log::debug!("[graph_edges] review obj = {:#?}", review);
    outgoing_edges(base_filepaths, diff_graph, graph_elems, review, &mut func_call_identifier, &mut import_identifier, &mut funcdef_identifier).await;
    incoming_edges(review, diff_graph, graph_elems).await;
}

async fn incoming_edges(review: &Review, diff_graph: &DiffGraph,
    graph_elems: &mut MermaidGraphElements)
{
    let import_lines_identifier_opt = ImportLinesIdentifier::new();
    if import_lines_identifier_opt.is_none() {
        log::error!("[incoming_edges] Unable to initiate ImportLinesIdentifier");
        return;
    }
    let mut import_lines_identifier = import_lines_identifier_opt.expect("Empty import_lines_identifier_opt");
    let import_def_identifier_opt = ImportDefIdentifier::new();
    if import_def_identifier_opt.is_none() {
        log::error!("[incoming_edges] Unable to initiate ImportDefIdentifier");
        return;
    }
    let mut import_def_identifier = import_def_identifier_opt.expect("Empty import_def_identifier_opt");
    let func_call_validator_opt = FunctionCallValidator::new();
    if func_call_validator_opt.is_none() {
        log::error!("[incoming_edges] Unable to initiate FunctionCallValidator");
        return;
    }
    let mut func_call_validator = func_call_validator_opt.expect("Empty func_call_validator_opt");
    git_checkout_commit(review, review.pr_head_commit());
    process_func_defs(
        review,
        diff_graph,
        graph_elems,
        &mut import_lines_identifier,
        &mut import_def_identifier,
        &mut func_call_validator,
        "green"
    ).await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_defs(
        review,
        diff_graph,
        graph_elems,
        &mut import_lines_identifier,
        &mut import_def_identifier,
        &mut func_call_validator,
        "red"
    ).await;
    log::debug!("[incoming_edges] Incoming edges processed");
}

async fn outgoing_edges(base_filepaths: &Vec<PathBuf>, diff_graph: &DiffGraph,
    graph_elems: &mut MermaidGraphElements, review: &Review,
    func_call_identifier: &mut FunctionCallIdentifier,
    import_identifier: &mut ImportIdentifier,
    funcdef_identifier: &mut FunctionDefIdentifier) 
{
    log::debug!("[outgoing_edges] review obj = {:#?}", review);
    git_checkout_commit(review, review.pr_head_commit());
    process_func_calls(
        import_identifier,
        func_call_identifier,
        funcdef_identifier,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "green").await;
    git_checkout_commit(review, review.base_head_commit());
    process_func_calls(import_identifier,
        func_call_identifier,
        funcdef_identifier,
        review,
        diff_graph,
        base_filepaths,
        graph_elems,
        "red").await;
    log::debug!("[outgoing_edges] Outgoing edges processed");
}

async fn process_func_calls(import_identifier: &mut ImportIdentifier, func_call_identifier: &mut FunctionCallIdentifier,
    funcdef_identifier: &mut FunctionDefIdentifier,
    review: &Review, diff_graph: &DiffGraph, base_filepaths: &Vec<PathBuf>,
    graph_elems: &mut MermaidGraphElements, edge_color: &str)
{
    log::debug!("[process_func_calls] review obj = {:#?}", review);
    let files_def_map;
    if edge_color == "green" {
        files_def_map = diff_graph.hunk_diff_map().added_files_map();
    } else {
        files_def_map = diff_graph.hunk_diff_map().deleted_files_map();
    }
    for (source_filepath, func_def) in files_def_map {
        // for each chunk, find function calls
        let lang_opt = detect_language(source_filepath);
        if lang_opt.is_none() {
            log::error!("[process_func_calls] Unable to determine language: {}", source_filepath);
            continue;
        }
        let lang = lang_opt.expect("Empty lang_opt");
        let src_filepath = Path::new(source_filepath);
        let src_file_pathbuf = src_filepath.to_path_buf();
        if let Some(func_calls) = func_call_identifier.functions_in_file(&src_file_pathbuf, &lang).await {
            log::debug!("[process_func_calls] func_calls = {:#?}", &func_calls);
            let mut func_def_call_map = associate_function_calls(func_def,
                func_calls.function_calls());
            if func_def_call_map.is_empty() {
                for func_call in func_calls.function_calls() {
                    let fake_func_def = FunctionDefinition {
                        line_number: (func_call.line_number().to_owned() as usize),
                        structure_name: format!("{}", func_call.line_number()),
                    };
                    func_def_call_map.insert(fake_func_def, vec![func_call.to_owned()]);
                }
            }
            log::debug!("[process_func_calls] func_def_call_map = {:#?}", &func_def_call_map);
            // for each function call, try to find import and dest func def eventutally
            for (func_def, func_calls_vec) in func_def_call_map {
                search_func_call(&func_calls_vec, source_filepath, import_identifier, &lang, diff_graph, 
                    review, edge_color, graph_elems, source_filepath, base_filepaths,
                    funcdef_identifier, &func_def.structure_name, &func_def.line_number).await;
            }
        }
    }
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
            log::debug!("[process_func_calls] hunk_funk_calls = {:#?}", &hunk_func_calls);
            for (hunk_lines, func_call_output) in hunk_func_calls {
                if let Some(src_func_name) = hunk_lines.function_name() {
                    if let Some(src_func_line_number) = hunk_lines.line_number() {
                        search_func_call(func_call_output.function_calls(), source_filepath, import_identifier, 
                            &lang, diff_graph, review, edge_color, graph_elems, &source_file_name,
                            base_filepaths, funcdef_identifier, src_func_name, src_func_line_number).await;
                    }
                }
            }
        }    
    }
}

async fn search_func_call(func_calls: &Vec<FunctionCall>, 
        source_filepath: &str, import_identifier: &mut ImportIdentifier, lang: &str, diff_graph: &DiffGraph,
        review: &Review, edge_color: &str, graph_elems: &mut MermaidGraphElements, source_file_name: &str,
        base_filepaths: &Vec<PathBuf>, funcdef_identifier: &mut FunctionDefIdentifier, src_func_name: &str,
        src_func_line_number: &usize
) {
    log::debug!("[search_func_call] funcalls = {:#?}, filename - {}", &func_calls ,source_filepath);
    for dest_func_call in func_calls {
        if let Some(import_filepath) = import_identifier.get_import_path_file(
            source_filepath, &lang, dest_func_call.function_name()).await {
            // get file
            // get diffgraph all files and see if they contain filepath
            log::debug!("[search_func_call] import filepath = {:#?}, filename - {}", &import_filepath, source_filepath);
            let possible_diff_file_paths: Vec<&String> =  diff_graph.hunk_diff_map().all_files().into_iter()
                .filter(|file_path| file_path.contains(import_filepath.get_matching_import().possible_file_path())).collect();
            log::debug!("[search_func_call] possible_diff_file_paths = {:?}", &possible_diff_file_paths);
            let mut edge_added = false;
            if !possible_diff_file_paths.is_empty() {
                for possible_diff_file_path in possible_diff_file_paths {
                    if diff_graph.hunk_diff_map().all_files().contains(&possible_diff_file_path)
                    {
                        log::debug!("[search_func_call] possible_diff_file_path ={}", &possible_diff_file_path);
                        if let Some(possible_file_rel) = absolute_to_relative_path(possible_diff_file_path, review) {
                            let hunks_for_func = diff_graph.hunk_diff_map().file_line_map()
                                .get(possible_diff_file_path).expect("Empty entry in file_line_map");
                            if let Some(dest_func_def_line) = hunks_for_func.is_func_in_hunks(dest_func_call.function_name(), edge_color) {
                                edge_added = true;
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
            }
            if !edge_added {
                // if let Some(possible_file_rel) = absolute_to_relative_path(import_filepath.get_matching_import().possible_file_path(), review) {
                graph_elems.add_edge(
                    edge_color,
                    dest_func_call.line_number().to_owned() as usize,
                    src_func_name, 
                    dest_func_call.function_name(), 
                    &source_file_name,
                    import_filepath.get_matching_import().possible_file_path(),
                    "yellow",
                    "",
                    src_func_line_number,
                    &0);
    
                // }
                    // for possible_file_pathbuf in possible_file_pathbufs {
                    //     let possible_file_path: String = possible_file_pathbuf.to_string_lossy().to_string();
                    //     if let Some(possible_file_rel) = 
                    //                         absolute_to_relative_path(&possible_file_path, review) {
                    //         // search only for func def with specific name
                    //         // if something comes up, add edge!
                    //         // thread::sleep(Duration::from_secs(1));
                    //         if let Some(func_def) = funcdef_identifier.function_defs_in_file(
                    //             possible_file_pathbuf, &lang, dest_func_call.function_name()).await {
                    //                 log::debug!("[search_func_call] func_def ={:#?}", &func_def);
                    //             if let Some(dest_func_def_line) = func_def.get_function_line_number() {
                                    
                    //         }
                    //     }
                    // }
                // }
            }
        }    
    }
}

async fn process_func_defs(review: &Review,
    diff_graph: &DiffGraph, graph_elems: &mut MermaidGraphElements,
    import_lines_identifier: &mut ImportLinesIdentifier, import_def_identifier: &mut ImportDefIdentifier,
    func_call_validator: &mut FunctionCallValidator, edge_color: &str)
{
    let files_def_map;
    if edge_color == "green" {
        files_def_map = diff_graph.hunk_diff_map().added_files_map();
    } else {
        files_def_map = diff_graph.hunk_diff_map().deleted_files_map();
    }
    for (dest_filename, func_defs) in files_def_map {
        for func_def in func_defs {
            let dest_lang_opt = detect_language(&dest_filename);
            if dest_lang_opt.is_none() {
                log::error!("[process_func_defs] Unable to detect language: {}", dest_filename);
                continue;
            }
            let dest_lang = dest_lang_opt.expect("Empty dest_lang_opt");
            let dest_func_name = &func_def.structure_name;
            let dest_funcdef_line = &func_def.line_number;
            if let Some(possible_filepaths) = 
                    function_calls_search(review, dest_func_name, &dest_lang)
            {
                search_func_defs(&possible_filepaths, dest_filename, &dest_lang,
                    graph_elems, review,
                    import_lines_identifier, import_def_identifier,
                    func_call_validator, dest_func_name, dest_funcdef_line
                ).await;
            }
        }
    }
    
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
                    // TODO FIXME - get one file name only once
                    search_func_defs(&possible_filepaths, dest_filename, &dest_lang,
                        graph_elems, review,
                        import_lines_identifier, import_def_identifier,
                        func_call_validator, dest_func_name, dest_funcdef_line
                    ).await;
                }
            }
        }
    }   
}
}

async fn search_func_defs(possible_filepaths: &HashMap<String, Vec<(usize, String)>>, dest_filename: &str,dest_lang: &str,
    graph_elems: &mut MermaidGraphElements, review: &Review,
    import_lines_identifier: &mut ImportLinesIdentifier, import_def_identifier: &mut ImportDefIdentifier,
    func_call_validator: &mut FunctionCallValidator, dest_func_name: &str, dest_funcdef_line: &usize
) {
    for (possible_filepath, lines_info) in possible_filepaths {
        if possible_filepath == dest_filename {
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
    
        // TODO FIXME - filter line_num for being in import range
        for (line_num, line_content) in lines_info {
            if let Some(import_hunks) = import_lines_identifier.import_lines_range_in_file(&possible_pathbuf, &lang).await {
                if let Some(import_def) = import_def_identifier.identify_import_def(&possible_pathbuf, &dest_func_name, &lang, &import_hunks).await {
                    log::debug!("[process_func_defs] import_def = {:#?}, possible file names - {}, filename - {}", &import_def , &possible_filepath, dest_filename);
                    if func_call_validator.valid_func_calls_in_file(&possible_pathbuf, &lang, &dest_func_name, &line_content, &import_def).await {
                        if let Some(source_filename) = absolute_to_relative_path(&possible_filepath, review) {
                            if let Some(src_func_def) = get_function_def_for_func_call(
                                &possible_pathbuf, line_num.to_owned()
                            ).await {
                                // add edge
                                log::debug!("[process_func_defs] src_func_def = {:#?}, filename = {}", &src_func_def, dest_filename);
                                let mut dest_file_rel = dest_filename.to_string();
                                if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
                                    dest_file_rel = dest_file_relative_path;
                                }
                                graph_elems.add_edge("",
                                line_num.to_owned(),
                                src_func_def.name(),
                                dest_func_name,
                                &source_filename,
                                &dest_file_rel,
                                "",
                                "yellow",
                                src_func_def.line_start(),
                                dest_funcdef_line);
                            } else {
                                // Add edge for file subgroup
                                let src_func_def_name = format!("{}", line_num);
                                let src_func_def_line = line_num;
                                let mut dest_file_rel = dest_filename.to_string();
                                if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
                                    dest_file_rel = dest_file_relative_path;
                                }
                                graph_elems.add_edge("",
                                line_num.to_owned(),
                                &src_func_def_name,
                                dest_func_name,
                                &source_filename,
                                &dest_file_rel,
                                "",
                                "yellow",
                                src_func_def_line,
                                dest_funcdef_line);
                            }
                        }
                    }
                } else {
                    // Add edge for file subgroup
                    if let Some(source_filename) = absolute_to_relative_path(&possible_filepath, review) {
                        let src_func_def_name = format!("{}", line_num);
                        let src_func_def_line = line_num;
                        let mut dest_file_rel = dest_filename.to_string();
                        if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
                            dest_file_rel = dest_file_relative_path;
                        }
                        graph_elems.add_edge("",
                        line_num.to_owned(),
                        &src_func_def_name,
                        dest_func_name,
                        &source_filename,
                        &dest_file_rel,
                        "",
                        "yellow",
                        src_func_def_line,
                        dest_funcdef_line);
                    }
                }
            } else {
                if let Some(source_filename) = absolute_to_relative_path(&possible_filepath, review) {
                    let src_func_def_name = format!("{}", line_num);
                    let src_func_def_line = line_num;
                    let mut dest_file_rel = dest_filename.to_string();
                    if let Some(dest_file_relative_path) = absolute_to_relative_path(&dest_filename, review) {
                        dest_file_rel = dest_file_relative_path;
                    }
                    graph_elems.add_edge("",
                    line_num.to_owned(),
                    &src_func_def_name,
                    dest_func_name,
                    &source_filename,
                    &dest_file_rel,
                    "",
                    "yellow",
                    src_func_def_line,
                    dest_funcdef_line);
                }
            }
        }
    }
}