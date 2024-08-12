use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{graph::utils::numbered_content, utils::review::Review};

use super::utils::{all_code_files, call_llm_api, read_file};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FuncDefInfo {
    name: String,
    line_start: usize,
    line_end: usize,
    parent: String,
}

impl PartialEq for FuncDefInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.line_start == other.line_start
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FunctionFileMap {
    file_name: String,
    functions: Vec<FuncDefInfo>
    // implement a function which takes in starting and ending line numbers of a continous range
    // and returns the functions inside the range like Vec of ((start_line, end_line) function_name)
}

impl FunctionFileMap {
    pub fn functions(&self) -> &Vec<FuncDefInfo> {
        &self.functions
    }

    pub fn is_func_in_file(&self, func: &FuncDefInfo) -> bool {
        self.functions.contains(func)
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct AllFileFunctions {
    func_map: HashMap<String, FunctionFileMap>  // file name will be key
}

impl AllFileFunctions {

    pub fn functions_in_file(&self, filename: &str) -> Option<&FunctionFileMap> {
        self.func_map.get(filename)
    }

    pub fn all_files(&self) -> Vec<&String> {
        self.func_map.keys().collect::<Vec<&String>>()
    }
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

pub async fn generate_function_map(file_paths: &Vec<PathBuf>) -> Option<AllFileFunctions> {
    let mut all_file_functions = AllFileFunctions { func_map: HashMap::new() };
    let system_prompt_opt = read_file("/app/prompts/prompt_function_lines");
    if system_prompt_opt.is_none() {
        log::error!("[generate_function_map] Unable to read prompt_function_lines");
        return None;
    }
    let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
    let system_prompt_end_opt = read_file("/app/prompts/prompt_function_boundary");
    if system_prompt_end_opt.is_none() {
        log::error!("[generate_function_map] Unable to read prompt_function_boundary");
        return None;
    }
    let system_prompt_lines_end = system_prompt_end_opt.expect("Empty system_prompt");
    for path in file_paths {
        log::debug!("[generate_function_map] path = {:?}", path);
        let mut function_map = FunctionFileMap {
            file_name: path.to_str().to_owned().unwrap_or("").to_string(),
            functions: Vec::new(),
        };
        let file_contents = std::fs::read_to_string(path.clone()).ok()?;
        let numbered_content = numbered_content(file_contents);
        let chunks = numbered_content.chunks(50);
        for chunk in chunks {
            let chunk_str = chunk.join("\n");
            let function_defs_opt = get_function_defs_in_chunk(&chunk_str, &system_prompt_lines).await;
            if function_defs_opt.is_none() {
                log::error!("[generate_function_map] Unable to get functions from llm");
                continue;
            }
            let function_defs = function_defs_opt.expect("Empty function_defs");
            for func_def in function_defs.functions.iter() {
                let func_boundary_opt = get_function_boundaries_in_chunk(&numbered_content, func_def.line_num, &system_prompt_lines_end).await;
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
        log::debug!("[generate_function_map] func_map = {:#?}", &function_map);
        all_file_functions.func_map.insert(path.to_str().unwrap().to_string(), function_map);
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
            log::error!("[get_function_defs_in_chunk] Failed to call LLM API");
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
    // let funcdefs = LlmFuncDefResponse{ functions: vec![LlmFuncDef{ name: "main".to_string(), line_num: 18, parent: "".to_string() }] };
    // return Some(funcdefs);
}

async fn get_function_boundaries_in_chunk(file_lines_numbered: &Vec<String>, func_def_line_num: usize, system_prompt: &str) -> Option<LlmFuncBoundaryResponse> {
    // divide lines into chunks and call with each chunk until line_end is found or files is empty
    let chunk_size = 70;
    let mut start = func_def_line_num;
    
    while start < file_lines_numbered.len() {
        let end = std::cmp::min(start + chunk_size, file_lines_numbered.len());
        let chunk: Vec<String> = file_lines_numbered[start..end].to_vec();
        let chunk_str = chunk.join("\n");
        
        let input = LlmFuncBoundaryInput {
            language: "rust".to_string(), // Assuming Rust as language, you can modify this as needed
            func_declared: file_lines_numbered[func_def_line_num].to_string(),
            chunk: chunk_str,
        };
        let llm_req = LlmFuncBoundaryRequest { input };
        let llm_req_res = serde_json::to_string(&llm_req);
        if llm_req_res.is_err() {
            log::error!("[get_function_boundaries_in_chunk] Error in serializing llm req: {}", llm_req_res.expect_err("Empty error in llm_req_res"));
            return None;
        }
        let llm_req_prompt = llm_req_res.expect("Uncaught error in llm_req_res"); 
        let prompt = format!("{}\n\n### User Message\nInput -\n{}\nOutput -", system_prompt, llm_req_prompt);
        match call_llm_api(prompt).await {
            None => {
                log::error!("[get_function_boundaries_in_chunk] Failed to call LLM API");
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
        // let func_resp = LlmFuncBoundaryResponse { function_boundary: 79 };
        // if func_resp.function_boundary == -1 {
        //     start += chunk_size;
        //     continue;
        // }
        // return Some(func_resp);
    }
    return None;
}