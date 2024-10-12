use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::graph::utils::numbered_content;

use super::{function_call::FunctionCall, gitops::HunkDiffLines, utils::{call_llm_api, read_file, strip_json_prefix}};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FuncDefInfo {
    pub(crate) name: String,
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) parent: String,
}

impl PartialEq for FuncDefInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.line_start == other.line_start
    }
}

impl FuncDefInfo {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn line_start(&self) -> &usize {
        &self.line_start
    }

    pub fn line_end(&self) -> &usize {
        &self.line_end
    }

    pub fn parent(&self) -> &String {
        &self.parent
    }
}

#[derive(Debug, Default, Clone)]
pub struct HunkFuncDef {
    func_def: FuncDefInfo,
    hunk_info: HunkDiffLines
}

impl HunkFuncDef {
    pub fn func_def(&self) -> &FuncDefInfo {
        &self.func_def
    }
}

// Struct to represent function definition
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct FunctionDefinition {
    line_number: i32,
}

// Struct to represent the output schema
#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionDefOutput {
    function_definition: Option<FunctionDefinition>,
    notes: Option<String>,
}

impl FunctionDefOutput {
    pub fn get_function_line_number(&self) -> Option<usize> {
        if let Some(func_def) = &self.function_definition {
            return Some(func_def.line_number as usize)
        }
        return None;
    }
}

// Struct to represent the input schema
#[derive(Serialize, Deserialize, Debug)]
struct InputSchema {
    code_chunk: String,
    language: String,
    function_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct InputSchemaDescription {
    code_chunk: String,
    language: String,
    function_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OutputSchemaDescription {
    function_definition: FunctionDefinitionDescription,
    notes: String,
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct FunctionDefinitionDescription {
    line_number: String,
}

// Struct for instructions that hold input/output schemas
#[derive(Serialize, Deserialize, Debug)]
struct Instructions {
    input_schema: InputSchemaDescription,
    output_schema: OutputSchemaDescription,
    task_description: String,
}
// Struct for the entire JSON prompt
#[derive(Serialize, Deserialize, Debug)]
struct FunctionDefPrompt {
    instructions: Instructions,
    sample_input: InputSchema,
    expected_output: FunctionDefOutput,
    input: Option<InputSchema>,
}

impl FunctionDefPrompt {
    fn set_input(&mut self, input: InputSchema) {
        self.input = Some(input);
    }
}

pub struct FunctionDefIdentifier {
    prompt: FunctionDefPrompt
}

impl FunctionDefIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_function_def");
        if system_prompt_opt.is_none() {
            log::error!("[function_calls_in_chunk] Unable to read prompt_function_def");
            return None;
        }
        let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
        let prompt_json_res = serde_json::from_str(&system_prompt_lines);
        if prompt_json_res.is_err() {
            log::error!("[FunctionCallIdentifier/new] Unable to deserialize prompt_json: {:?}",
                prompt_json_res.expect("Empty prompt_json_res"));
            return None;
        }
        let prompt_json: FunctionDefPrompt = prompt_json_res.expect("Empty error in prompt_json_res");
        return Some(Self { prompt: prompt_json});
    }

    pub async fn function_defs_in_file(&mut self, filepath: &PathBuf, lang: &str, function_name: &str) -> Option<FunctionDefOutput> {
        // concatenate functioncallsoutput for all chunks
        let file_contents_res = std::fs::read_to_string(filepath.clone());
        if file_contents_res.is_err() {
            log::error!(
                "[FunctionCallIdentifier/functions_in_file] Unable to read file: {:?}, error: {:?}",
                &filepath, file_contents_res.expect_err("Empty error in file_contents_res")
            );
            return None;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_contents_res");
        let numbered_content = numbered_content(file_contents);
        let chunks = numbered_content.chunks(50);
        for chunk in chunks {
            let chunk_str = chunk.join("\n");
            if let Some(func_defs) = self.function_defs_in_chunk(&chunk_str, filepath, lang, function_name).await {
                return Some(func_defs);
            }
        }
        return None;
    }

    async fn function_defs_in_chunk(&mut self, chunk: &str, filepath: &PathBuf, lang: &str, function_name: &str) -> Option<FunctionDefOutput> {
        let input = InputSchema{ code_chunk: chunk.to_string(), language: lang.to_string(),
            function_name: function_name.to_string() };
        self.prompt.input = Some(input);
        let prompt_str_res = serde_json::to_string(&self.prompt);
        if prompt_str_res.is_err() {
            log::error!(
                "[FunctionCallIdentifier/functions_in_chunk] Unable to serialize prompt: {:?}",
                prompt_str_res.expect_err("Empty error in prompt_str_res"));
                return None;
        }
        let prompt_str = prompt_str_res.expect("Uncaught error in prompt_str_res");
        let final_prompt = format!("{}\nOutput - ", &prompt_str);
        let prompt_response_opt =  call_llm_api(final_prompt).await;
        if prompt_response_opt.is_none() {
            log::error!("[FunctionCallIdentifier/functions_in_chunk] Unable to call llm for chunk: {:?}", chunk);
            return None;
        }
        let mut prompt_response = prompt_response_opt.expect("Empty prompt_response_opt");
        if let Some(stripped_json) = strip_json_prefix(&prompt_response) {
            prompt_response = stripped_json.to_string();
        }
        let deserialized_response = serde_json::from_str(&prompt_response);
        if deserialized_response.is_err() {
            let e = deserialized_response.expect_err("Empty error in deserialized_response");
            log::error!("[FunctionCallIdentifier/functions_in_chunk] Error in deserializing response: {:?}", e);
            return None;
        }
        let func_calls: FunctionDefOutput = deserialized_response.expect("Empty error in deserialized_response");
        return Some(func_calls);
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FunctionFileMap {
    pub(crate) file_name: String,
    pub(crate) functions: Vec<FuncDefInfo>
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

    pub fn func_def(&self, func_name: &str) -> Option<&FuncDefInfo> {
        self.functions.iter().find(|f| f.name == func_name)
    }

    pub fn func_at_line(&self, line_num: usize) -> Option<&FuncDefInfo> {
        self.functions.iter().find(
            |f| f.line_start <= line_num && line_num <= f.line_end)
    }

    pub fn funcs_in_hunk(&self, hunk: &HunkDiffLines) -> Vec<HunkFuncDef> {
        let hunk_func_vec: Vec<HunkFuncDef> = self.functions
            .iter()
            .filter_map(|func| {
                // Check if the function's start or end line falls within the hunk's start and end line range
                if (func.line_start() >= hunk.start_line() && func.line_start() <= hunk.end_line()) ||
                (func.line_end() >= hunk.start_line() && func.line_end() <= hunk.end_line()) ||
                // Additionally check if the function completely spans over the hunk range
                (func.line_start() <= hunk.start_line() && func.line_end() >= hunk.end_line())
                {
                    let hunkfuncdef = HunkFuncDef {
                        func_def: func.clone(),
                        hunk_info: hunk.clone(),
                    };
                    return Some(hunkfuncdef);
                }
                return None;
            }).collect();
        return hunk_func_vec;
    }

    pub fn funcs_for_func_call(&self, func_call: &FunctionCall) -> Option<&FuncDefInfo>{
        let line_num = func_call.line_number().to_owned() as usize;
        return self.func_at_line(line_num);
    }

    // pub fn funcs_for_lines(&self, lines: &Vec<usize>) -> HashMap<usize, FuncDefInfo> {
    //     let mut line_funcdef_map = HashMap::new();

    //     for line in lines {
    //         for func in &self.functions {
    //             if func.line_start <= *line && *line <= func.line_end {
    //                 line_funcdef_map.entry(*line).or_insert(func.clone());
    //             }
    //         }
    //     }
    //     return line_funcdef_map;
    // }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct AllFileFunctions {
    pub(crate) func_map: HashMap<String, FunctionFileMap>  // file name will be key
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
pub struct LlmFuncDef {
    #[serde(default)]
    name: String,
    #[serde(default)]
    line_start: usize,
    #[serde(default)]
    parent: String
}

impl LlmFuncDef {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn line_start(&self) -> &usize {
        &self.line_start
    }
}
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFuncDefResponse {
    #[serde(default)]
    functions: Vec<LlmFuncDef>
}

impl LlmFuncDefResponse {
    pub fn functions(&self) -> &Vec<LlmFuncDef> {
        &self.functions
    }
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
        let file_contents_res = std::fs::read_to_string(path.clone());
        if file_contents_res.is_err() {
            log::error!("[generate_function_map] Error in reading file contents: {:?}",
                file_contents_res.expect_err("Empty error"));
            continue;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
        let numbered_content = numbered_content(file_contents);
        let chunks = numbered_content.chunks(30);
        for chunk in chunks {
            let chunk_str = chunk.join("\n");
            let function_defs_opt = get_function_defs_in_chunk(&chunk_str, &system_prompt_lines).await;
            if function_defs_opt.is_none() {
                log::error!("[generate_function_map] Unable to get functions from llm");
                continue;
            }
            let function_defs = function_defs_opt.expect("Empty function_defs");
            for func_def in function_defs.functions.iter() {
                if func_def.name.len() == 0 {
                    log::debug!("[generate_function_map] No valid name for func_def {:?}", &func_def);
                    continue;
                }
                let func_boundary_opt = get_function_boundaries_in_chunk(&numbered_content, func_def.line_start, &system_prompt_lines_end).await;
                if func_boundary_opt.is_none() {
                    log::debug!("[generate_function_map] No function end detected for func: {:?}", &func_def);
                    continue;
                }
                let func_boundary = func_boundary_opt.expect("Empty func_boundary_opt");
                function_map.functions.push(FuncDefInfo {
                    name: func_def.name.clone(),
                    line_start: func_def.line_start,
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

pub async fn get_function_def_for_func_call(filepath: &PathBuf, func_call_line_num: usize) -> Option<LlmFuncDef> {
    let system_prompt_opt = read_file("/app/prompts/prompt_function_lines");
    if system_prompt_opt.is_none() {
        log::error!("[get_function_def_for_func_call] Unable to read prompt_function_lines");
        return None;
    }
    let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
    let file_contents_res = std::fs::read_to_string(filepath.clone());
    if file_contents_res.is_err() {
        log::error!("[get_function_def_for_func_call] Error in reading file contents: {:?}",
            file_contents_res.expect_err("Empty error"));
        return None;
    }
    let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
    let numbered_content = numbered_content(file_contents);
    let mut current_line = func_call_line_num;
    let chunk_size = 30;
    // Loop until we reach the beginning of the file
    while current_line > 0 {
        // Determine the start and end for the current chunk
        let start = if current_line >= chunk_size {
            current_line - chunk_size
        } else {
            0
        };
        
        // Extract the chunk
        let chunk_str: String = numbered_content[start..=current_line].join("\n");
        // Process the chunk
        let function_defs_opt = get_function_defs_in_chunk(&chunk_str, &system_prompt_lines).await;
        if function_defs_opt.is_none() {
            log::error!("[generate_function_map] Unable to get functions from llm");
            continue;
        }
        let function_defs = function_defs_opt.expect("Empty function_defs");
        if let Some(func_def) = function_defs.functions().first() {
            return Some(func_def.to_owned());
        }
        // Move the current line up by the chunk size
        current_line = start;
    }
    return None;
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
                    "[get_function_defs_in_chunk] funcdefs error: {:?}",
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
    let chunk_size = 40;
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