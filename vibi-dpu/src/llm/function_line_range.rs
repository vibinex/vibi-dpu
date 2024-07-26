use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::review::Review;

use super::utils::{call_llm_api, read_file};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct FuncDefInfo {
    name: String,
    line_start: usize,
    line_end: usize,
    parent: String,
}
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct FunctionFileMap {
    file_name: String,
    functions: Vec<FuncDefInfo>
    // implement a function which takes in starting and ending line numbers of a continous range
    // and returns the functions inside the range like Vec of ((start_line, end_line) function_name)
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct AllFileFunctions {
    func_map: HashMap<String, FunctionFileMap>  // file name will be key
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncDefInput {
    language: String,
    chunk: String
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncDefRequest {
    input: LlmFuncDefInput
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncDef {
    name: String,
    line_num: usize,
    parent: String
}
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncDefResponse {
    functions: Vec<LlmFuncDef>
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncBoundaryInput {
    language: String,
    func_declared: String,
    chunk: String

}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncBoundaryRequest {
    input: LlmFuncBoundaryInput
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncBoundaryResponse {
    function_boundary: i32
}

pub async fn generate_function_map(review: &Review) -> Option<AllFileFunctions> {
    let dir = review.clone_dir();
    let mut all_file_functions = AllFileFunctions { func_map: HashMap::new() };
    let system_prompt_opt = read_file("/app/prompts/prompt_function_lines");
    if system_prompt_opt.is_none() {
        log::error!("[mermaid_comment] Unable to read system prompt");
        return None;
    }
    let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
    let system_prompt_end_opt = read_file("/app/prompts/prompt_function_lines_end");
    if system_prompt_end_opt.is_none() {
        log::error!("[mermaid_comment] Unable to read system prompt");
        return None;
    }
    let system_prompt_lines_end = system_prompt_end_opt.expect("Empty system_prompt");
    let entries_res = std::fs::read_dir(dir);
    if entries_res.is_err() {
        let e = entries_res.expect_err("Empty error in entry_res");
        log::error!(
            "[generate_function_map] Error reading dir: {} error = {:?}", dir, e);
        return None;
    }
    let entries = entries_res.expect("Empty error in entry_res");
    for entry_res in entries {
        if entry_res.is_err() {
            let e = entry_res.expect_err("Empty error in entry_res");
            log::error!(
                "[generate_function_map] Error reading, skipping directory entry, error = {:?}", e);
            continue;
        }
        let entry = entry_res.expect("Empty entry_res");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(path.clone()).ok()?;
            let mut function_map = FunctionFileMap {
                file_name: path.to_str().unwrap().to_string(),
                functions: Vec::new(),
            };

            // Divide content into chunks of 30 lines
            let lines: Vec<&str> = content.lines().collect();
            // TODO - convert lines to numbered content
            let chunks = lines.chunks(50);

            for chunk in chunks {
                let chunk_str = chunk.join("\n");
                let function_defs_opt = get_function_defs_in_chunk(&chunk_str, &system_prompt_lines_end).await;
                if function_defs_opt.is_none() {
                    log::error!("[get_function_defs_in_chunk] Unable to get functions from llm");
                    continue;
                }
                let function_defs = function_defs_opt.expect("Empty function_defs");
                for func_def in function_defs.functions.iter() {
                    let func_boundary_opt = get_function_boundaries_in_chunk(&lines, func_def.line_num, &system_prompt_lines_end).await;
                    if func_boundary_opt.is_none() {
                        continue;
                    }
                    let func_boundary = func_boundary_opt.expect("Empty func_boundary_opt");
                    function_map.functions.push(FuncDefInfo {
                        name: func_def.name.clone(),
                        line_start: func_def.line_num,
                        line_end: func_boundary.function_boundary as usize,
                        parent: func_def.parent.clone(),
                    });
                }
            }
            all_file_functions.func_map.insert(path.to_str().unwrap().to_string(), function_map);
        }
    }
    return Some(all_file_functions);
}

async fn get_function_defs_in_chunk(chunk: &str, system_prompt: &str) -> Option<LlmFuncDefResponse> {
    let llm_req = LlmFuncDefRequest {
        input: LlmFuncDefInput {
                language: "rust".to_string(),
                chunk: chunk.to_string()
            } 
        };
    let llm_req_res = serde_json::to_string(&llm_req);
    if llm_req_res.is_err() {
        log::error!("[get_function_defs_in_chunk] Error in serializing llm req: {}", llm_req_res.expect_err("Empty error in llm_req_res"));
        return None;
    }
    let llm_req_prompt = llm_req_res.expect("Uncaught error in llm_req_res"); 
    let prompt = format!("{}\n\n### User Message\nInput -\n{}\nOutput -", system_prompt, llm_req_prompt);
    match call_llm_api(prompt).await {
        None => {
            log::error!("[mermaid_comment] Failed to call LLM API");
            return None;
        }
        Some(llm_response) => {
            let funcdefs_res = serde_json::from_str(&llm_response);
            if funcdefs_res.is_err() {
                log::error!(
                    "[get_function_defs_in_chunk] funcdefs error: {}",
                    funcdefs_res.expect_err("Empty error in funcdefs_res"));
                    return None;
            }
            let funcdefs: LlmFuncDefResponse = funcdefs_res.expect("Uncaught error in funcdefs_res");
            return Some(funcdefs);
        }
    }
}

async fn get_function_boundaries_in_chunk(file_lines_numbered: &Vec<&str>, func_def_line_num: usize, system_prompt: &str) -> Option<LlmFuncBoundaryResponse> {
    // divide lines into chunks and call with each chunk until line_end is found or files is empty
    let chunk_size = 70;
    let mut start = func_def_line_num;
    
    while start < file_lines_numbered.len() {
        let end = std::cmp::min(start + chunk_size, file_lines_numbered.len());
        let chunk: Vec<&str> = file_lines_numbered[start..end].to_vec();
        let chunk_str = chunk.join("\n");
        
        let input = LlmFuncBoundaryInput {
            language: "rust".to_string(), // Assuming Rust as language, you can modify this as needed
            func_declared: file_lines_numbered[func_def_line_num].to_string(),
            chunk: chunk_str,
        };
        let llm_req = LlmFuncBoundaryRequest { input };
        let llm_req_res = serde_json::to_string(&llm_req);
        if llm_req_res.is_err() {
            log::error!("[get_function_defs_in_chunk] Error in serializing llm req: {}", llm_req_res.expect_err("Empty error in llm_req_res"));
            return None;
        }
        let llm_req_prompt = llm_req_res.expect("Uncaught error in llm_req_res"); 
        let prompt = format!("{}\n\n### User Message\nInput -\n{}\nOutput -", system_prompt, llm_req_prompt);
        match call_llm_api(prompt).await {
            None => {
                log::error!("[mermaid_comment] Failed to call LLM API");
                return None;
            }
            Some(llm_response) => {
                let func_resp_res = serde_json::from_str(&llm_response);
                if func_resp_res.is_err() {
                    let e = func_resp_res.expect_err("Empty error func_resp_res");
                    log::error!("[get_function_boundaries_in_chunk] Unable to deserialize response");
                    return None;
                }
                let func_resp: LlmFuncBoundaryResponse = func_resp_res.expect("Uncaught error in func_resp_res");
                if func_resp.function_boundary == -1 {
                    start += chunk_size;
                    continue;
                }
                return Some(func_resp);
            }
        }
    }

    return None;
}