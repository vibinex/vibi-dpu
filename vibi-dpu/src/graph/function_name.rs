use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use super::utils::{call_llm_api, read_file, strip_json_prefix};

// Struct to represent the output schema
#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionNameOutput {
    function_name: String,
    notes: Option<String>,
}

impl FunctionNameOutput {
    pub fn get_function_name(&self) -> &String {
        &self.function_name
    }
}

// Struct to represent the input schema
#[derive(Serialize, Deserialize, Debug)]
struct InputSchema {
    code_line: String,
    language: String,
}

// Struct for instructions that hold input/output schemas
#[derive(Serialize, Deserialize, Debug)]
struct Instructions {
    input_schema: InputSchema,
    output_schema: FunctionNameOutput,
    task_description: String,
}
// Struct for the entire JSON prompt
#[derive(Serialize, Deserialize, Debug)]
struct FunctionNamePrompt {
    instructions: Instructions,
    sample_input: InputSchema,
    expected_output: FunctionNameOutput,
    input: Option<InputSchema>,
}

impl FunctionNamePrompt {
    fn set_input(&mut self, input: InputSchema) {
        self.input = Some(input);
    }
}

pub struct FunctionNameIdentifier {
    prompt: FunctionNamePrompt,
    cached_output: HashMap<String, String>
}

impl FunctionNameIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_function_name");
        if system_prompt_opt.is_none() {
            log::error!("[FunctionNameIdentifier/new] Unable to read prompt_function_name");
            return None;
        }
        let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
        let prompt_json_res = serde_json::from_str(&system_prompt_lines);
        if prompt_json_res.is_err() {
            log::error!("[FunctionNameIdentifier/new] Unable to deserialize prompt_json: {:?}",
                prompt_json_res.expect("Empty prompt_json_res"));
            return None;
        }
        let prompt_json: FunctionNamePrompt = prompt_json_res.expect("Empty error in prompt_json_res");
        return Some(Self { prompt: prompt_json, cached_output: HashMap::new()});
    }

    pub async fn function_name_in_line(&mut self, code_line: &str, lang: &str) -> Option<FunctionNameOutput> {
        // concatenate functioncallsoutput for all chunks
        if let Some(cached_func_name) = self.cached_output.get(code_line) {
            return Some(FunctionNameOutput{ function_name: cached_func_name.to_string(), notes: None })
        }
        let input = InputSchema{ code_line: code_line.to_string(), language: lang.to_string() };
        self.prompt.input = Some(input);
        let prompt_str_res = serde_json::to_string(&self.prompt);
        if prompt_str_res.is_err() {
            log::error!(
                "[FunctionNameIdentifier/function_name_in_line] Unable to serialize prompt: {:?}",
                prompt_str_res.expect_err("Empty error in prompt_str_res"));
                return None;
        }
        let prompt_str = prompt_str_res.expect("Uncaught error in prompt_str_res");
        let final_prompt = format!("{}\nOutput - ", &prompt_str);
        let prompt_response_opt =  call_llm_api(final_prompt).await;
        if prompt_response_opt.is_none() {
            log::error!("[FunctionNameIdentifier/function_name_in_line] Unable to call llm for code line: {:?}", code_line);
            return None;
        }
        let mut prompt_response = prompt_response_opt.expect("Empty prompt_response_opt");
        if let Some(stripped_json) = strip_json_prefix(&prompt_response) {
            prompt_response = stripped_json.to_string();
        }
        let deserialized_response = serde_json::from_str(&prompt_response);
        if deserialized_response.is_err() {
            let e = deserialized_response.expect_err("Empty error in deserialized_response");
            log::error!("[FunctionNameIdentifier/function_name_in_line] Error in deserializing response: {:?}", e);
            return None;
        }
        let func_name: FunctionNameOutput = deserialized_response.expect("Empty error in deserialized_response");
        if func_name.get_function_name().is_empty() {
            return None;
        }
        self.cached_output.insert(code_line.to_string(), func_name.get_function_name().to_string());
        return Some(func_name);
    }
}