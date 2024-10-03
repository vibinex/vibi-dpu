use std::{collections::HashMap, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{gitops::HunkDiffMap, utils::{call_llm_api, numbered_content, read_file}};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FunctionCallChunk {
    function_calls: Vec<usize>,
    #[serde(skip_deserializing)]
    function_name: Option<String>
}

impl FunctionCallChunk {
    pub fn new(function_calls: Vec<usize>, function_name: String) -> Self {
        Self { function_calls, function_name: Some(function_name) }
    }
    pub fn function_calls(&self) -> &Vec<usize> {
        &self.function_calls
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FunctionCallInput {
    pub language: String,
    pub chunk: String,
    pub function_name: String
}

pub async fn function_calls_in_chunk(chunk: &str, func_name: &str) -> Option<FunctionCallChunk>{
    let system_prompt_opt = read_file("/app/prompts/prompt_function_call");
    if system_prompt_opt.is_none() {
        log::error!("[function_calls_in_chunk] Unable to read prompt_function_call");
        return None;
    }
    let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
    let func_call_input = FunctionCallInput{
        language: "rust".to_string(),
        chunk: chunk.to_string(),
        function_name: func_name.to_string() };
    let func_call_input_res = serde_json::to_string(&func_call_input);
    if func_call_input_res.is_err() {
        let e = func_call_input_res.expect_err("Empty error in func_call_input_res");
        log::error!("[function_calls_in_chunk] Error serializing func call input: {:?}", e);
        return None;
    }
    let func_call_input_str = func_call_input_res.expect("Uncaught error in func_call_input_res");
    let prompt = format!("{}\n\n### User Message\nInput -\n{}\nOutput -",
        system_prompt_lines, &func_call_input_str);
    let prompt_response_opt =  call_llm_api(prompt).await;
    if prompt_response_opt.is_none() {
        log::error!("[function_calls_in_chunk] Unable to call llm for chunk: {:?}", chunk);
        return None;
    }
    let prompt_response = prompt_response_opt.expect("Empty prompt_response_opt");
    let deserialized_response = serde_json::from_str(&prompt_response);
    if deserialized_response.is_err() {
        let e = deserialized_response.expect_err("Empty error in deserialized_response");
        log::error!("[function_calls_in_chunk] Error in deserializing response: {:?}", e);
        return None;
    }
    let func_call_chunk: FunctionCallChunk = deserialized_response.expect("Uncuaght error in deserialized_response");
    if func_call_chunk.function_calls.is_empty() {
        log::debug!("No function calls in this chunk: {:?}", chunk);
        return None;
    }
    return Some(func_call_chunk);
}

pub async fn function_calls_in_file(filepath: &PathBuf, func_name: &str) -> Option<Vec<FunctionCallChunk>> {
    let mut all_func_calls = Vec::<FunctionCallChunk>::new();
    let file_contents = std::fs::read_to_string(filepath.clone()).ok()?;
    let numbered_content = numbered_content(file_contents);
    let chunks = numbered_content.chunks(50);
    for chunk in chunks {
        let chunk_str = chunk.join("\n");
        let func_call_chunk_opt = function_calls_in_chunk(&chunk_str, func_name).await;
        if func_call_chunk_opt.is_none() {
            log::debug!("[function_calls_in_file] No function call in chunk for file: {:?}", filepath);
            continue;
        }
        let func_call_chunk = func_call_chunk_opt.expect("Empty func_call_chunk_opt");
        all_func_calls.push(func_call_chunk);
    }
    if all_func_calls.is_empty() {
        log::debug!("[function_calls_in_file] No function call in file: {:?}, {:?}", filepath, func_name);
        return None;
    }
    return Some(all_func_calls);
}

// pub async fn function_calls_in_hunks(hunk_file_map: &HunkDiffMap) -> Option<HashMap<String, HashMap<String, Vec<usize>>>> {
//     let mut file_func_call_map: HashMap<String, HashMap<String, Vec<usize>>> = HashMap::new();
//     for (file, hunk_lines_vec) in hunk_file_map.file_line_map() {
//         let file_contents_res = std::fs::read_to_string(file.clone());
//         if file_contents_res.is_err() {
//             let e = file_contents_res.expect_err("Empty error in file_contents_res");
//             log::error!("[function_calls_in_hunks] Error in getting file contents: {:?}", e);
//             continue;
//         }
//         let file_contents = file_contents_res.expect("Uncaught error in file_contents_res");
//         let numbered_content = numbered_content(file_contents);
//         let file_path = Path::new(file);
//         let file_vec = vec![file_path.to_path_buf()];
//         let imports_opt = get_import_lines(&file_vec).await;
//         if imports_opt.is_none() {
//             log::debug!("[function_calls_in_hunks] No imports in file: {:?}", file);
//             continue;
//         }
//         let file_imports = imports_opt.expect("Empty imports_opt");
//         let file_import_info = file_imports.file_import_info(file).expect("Empty file_import_info");
//         let mut func_call_map: HashMap<String, Vec<usize>> = HashMap::new();
//         for import_info in file_import_info.all_import_paths() {
//             let func_name = import_info.imported();
//             // TODO FIXME - get numbered content for hunk
//             for hunk_lines in hunk_lines_vec {
//                 let mut func_call_vec: Vec<usize> = Vec::new();
//                 let hunk_chunk_vec = &numbered_content[hunk_lines.start_line().. hunk_lines.end_line()];
//                 for hunk_chunk in hunk_chunk_vec.chunks(30) {
//                     let hunk_str = hunk_chunk.join("\n");
//                     if let Some(func_calls) = function_calls_in_chunk(&hunk_str, func_name).await {
//                         func_call_vec.extend(func_calls.function_calls());
//                     }
//                 }
//                 if !func_call_vec.is_empty() {
//                     func_call_map.entry(func_name.to_string()).or_insert_with(Vec::new).extend(func_call_vec);
//                 } 
//                 // get func name from imports
//                 // TODO - git checkout before function call
                

//             }
//         }
//         if !func_call_map.is_empty() {
//             file_func_call_map.insert(file.to_string(), func_call_map);
//         }
//     }
//     if file_func_call_map.is_empty() {
//         return None;
//     }
//     return Some(file_func_call_map);
// }
#[derive(Serialize, Deserialize, Debug)]
struct InputSchema {
    code_chunk: String,
    language: String,
    file_path: String,
}

// Structure for function calls in the output schema
#[derive(Serialize, Deserialize, Debug)]
struct FunctionCall {
    line_number: u32,
    function_name: String,
}

// Output schema structure
#[derive(Serialize, Deserialize, Debug)]
struct FunctionCallsOutput {
    function_calls: Vec<FunctionCall>,
    notes: Option<String>,
}

// Instruction structure
#[derive(Serialize, Deserialize, Debug)]
struct Instructions {
    input_schema: InputSchemaDescription,
    output_schema: OutputSchemaDescription,
    task_description: String,
}

// Description of input schema
#[derive(Serialize, Deserialize, Debug)]
struct InputSchemaDescription {
    code_chunk: String,
    language: String,
    file_path: String,
}

// Description of output schema
#[derive(Serialize, Deserialize, Debug)]
struct OutputSchemaDescription {
    function_calls: Vec<FunctionCallDescription>,
    notes: String,
}

// Description for each function call in output
#[derive(Serialize, Deserialize, Debug)]
struct FunctionCallDescription {
    line_number: String,
    function_name: String,
}

// Complete structure for JSON input and output
#[derive(Serialize, Deserialize, Debug)]
struct JsonStructure {
    instructions: Instructions,
    sample_input: InputSchema,
    expected_output: FunctionCallsOutput,
    input: Option<InputSchema>,
}

impl JsonStructure {
    fn set_input(&mut self, input: InputSchema) {
        self.input = Some(input);
    }
}

pub struct FunctionCallIdentifier {
    prompt: JsonStructure
}

impl FunctionCallIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_function_call");
        if system_prompt_opt.is_none() {
            log::error!("[function_calls_in_chunk] Unable to read prompt_function_call");
            return None;
        }
        let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
        let prompt_json_res = serde_json::from_str(&system_prompt_lines);
        if prompt_json_res.is_err() {
            log::error!("[FunctionCallIdentifier/new] Unable to deserialize prompt_json: {:?}",
                prompt_json_res.expect("Empty bprompt_json_res"));
            return None;
        }
        let prompt_json: JsonStructure = prompt_json_res.expect("Empty error in prompt_json_res");
        return Some(Self { prompt: prompt_json});
    }

    pub async fn functions_in_file(&mut self, filepath: &PathBuf, lang: &str) -> Option<FunctionCallsOutput> {
        // concatenate functioncallsoutput for all chunks
        let mut all_func_calls: FunctionCallsOutput = FunctionCallsOutput{ function_calls: vec![], notes: None };
        // TODO
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
            if let Some(mut func_calls) = self.functions_in_chunk(&chunk_str, filepath, lang).await {
                all_func_calls.function_calls.append(&mut func_calls.function_calls);
            }
        }
        if all_func_calls.function_calls.is_empty() {
            return None;
        }
        return Some(all_func_calls);
    }

    pub async fn functions_in_chunk(&mut self, chunk: &str, filepath: &PathBuf, lang: &str) -> Option<FunctionCallsOutput> {
        let input = InputSchema{ code_chunk: chunk.to_string(), language: lang.to_string(),
            file_path: filepath.to_str().expect("Empty filepath").to_string() };
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
        let prompt_response = prompt_response_opt.expect("Empty prompt_response_opt");
        let deserialized_response = serde_json::from_str(&prompt_response);
        if deserialized_response.is_err() {
            let e = deserialized_response.expect_err("Empty error in deserialized_response");
            log::error!("[FunctionCallIdentifier/functions_in_chunk] Error in deserializing response: {:?}", e);
            return None;
        }
        let func_calls: FunctionCallsOutput = deserialized_response.expect("Empty error in deserialized_response");
        return Some(func_calls);
    }
}