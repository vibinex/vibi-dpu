use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use super::utils::{call_llm_api, read_file, strip_json_prefix};

// Struct to represent the output schema
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionNameOutput {
    name: String,
    entity_type: String,
    status: String,
    notes: Option<String>,
}

impl FunctionNameOutput {
    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_entity_type(&self) -> &String {
        &self.entity_type
    }

    pub fn get_status(&self) -> &String {
        &self.status
    }

    pub fn get_notes(&self) -> Option<&String> {
        self.notes.as_ref()
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

#[derive(Debug, Serialize, Deserialize)]
struct ValidationPromptInput {
    code_line: String, // A line of code that possibly contains a function definition
    language: String,  // Programming language of the code (e.g., "Python", "Java")
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationPromptOutput {
    is_definition: bool, // True if the line is a valid function definition, false otherwise
    status: String,    // "valid", "invalid_input", or "insufficient_context"
    notes: Option<String>, // Optional notes explaining the decision
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationPromptOutputSchema {
    is_definition: String,
    status: String,
    notes: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationPromptInstructions {
    input_schema: ValidationPromptInput,
    output_schema: ValidationPromptOutputSchema,
    task_description: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationPrompt {
    instructions: ValidationPromptInstructions,
    sample_input: ValidationPromptInput,
    expected_output: ValidationPromptOutput,
    input: Option<ValidationPromptInput>
}

impl ValidationPrompt {
    fn set_input(&mut self, input: ValidationPromptInput) {
        self.input = Some(input);
    }
}

pub struct FunctionNameIdentifier {
    prompt: FunctionNamePrompt,
    validation_prompt: ValidationPrompt,
    cached_output: HashMap<String, FunctionNameOutput>
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

        let system_prompt_validation_opt = read_file("/app/prompts/prompt_valid_function_def");
        if system_prompt_validation_opt.is_none() {
            log::error!("[FunctionNameIdentifier/new] Unable to read prompt_valid_function_def");
            return None;
        }
        let system_prompt_validation_lines = system_prompt_validation_opt.expect("Empty system_prompt");
        let prompt_validation_json_res = serde_json::from_str(&system_prompt_validation_lines);
        if prompt_validation_json_res.is_err() {
            log::error!("[FunctionNameIdentifier/new] Unable to deserialize prompt_json: {:?}",
                prompt_validation_json_res.expect("Empty prompt_json_res"));
            return None;
        }
        let prompt_validation_json: ValidationPrompt = prompt_validation_json_res.expect("Empty error in prompt_json_res");
        return Some(Self { prompt: prompt_json, validation_prompt: prompt_validation_json, cached_output: HashMap::new()});
    }

    pub async fn function_name_in_line(&mut self, code_line: &str, lang: &str) -> Option<FunctionNameOutput> {
        if let Some(cached_func_name) = self.cached_output.get(code_line.trim()) {
            return Some(cached_func_name.to_owned());
        }
        let validation_input = ValidationPromptInput { code_line: code_line.to_string(), language: lang.to_string() };
        self.validation_prompt.set_input(validation_input);
        let validation_prompt_str_res = serde_json::to_string(&self.validation_prompt);
        if validation_prompt_str_res.is_err() {
            log::error!(
                "[FunctionNameIdentifier/function_name_in_line] Unable to serialize prompt: {:?}",
                validation_prompt_str_res.expect_err("Empty error in validation_prompt_str_res"));
                return None;
        }
        let validation_prompt_str = validation_prompt_str_res.expect("Uncaught error in validation_prompt_str_res");
        let validation_final_prompt = format!("{}\nOutput - ", &validation_prompt_str);
        log::debug!("[FunctionNameIdentifier/function_name_in_line] validation code_line: {}", code_line);
        let validation_prompt_response_opt =  call_llm_api(validation_final_prompt).await;
        if validation_prompt_response_opt.is_none() {
            log::error!("[FunctionNameIdentifier/function_name_in_line] Unable to call llm for validation code line: {:?}", code_line);
            return None;
        }
        let mut validation_prompt_response = validation_prompt_response_opt.expect("Empty prompt_response_opt");
        if let Some(stripped_json) = strip_json_prefix(&validation_prompt_response) {
            validation_prompt_response = stripped_json.to_string();
        }
        let deserialized_response = serde_json::from_str(&validation_prompt_response);
        if deserialized_response.is_err() {
            let e = deserialized_response.expect_err("Empty error in deserialized_response");
            log::error!("[FunctionNameIdentifier/function_name_in_line] Error in deserializing response: {:?}", e);
            return None;
        }
        let validation_out: ValidationPromptOutput = deserialized_response.expect("Empty error in deserialized_response");
        log::debug!("[FunctionNameIdentifier/function_name_in_line] validation response obj: {:#?}", &validation_out);
        if !validation_out.is_definition || validation_out.status != "valid" {
            log::error!("[FunctionNameIdentifier/function_name_in_line] Given code line is not valid function def");
            return None;
        }
        let input = InputSchema { code_line: code_line.to_string(), language: lang.to_string() };
        self.prompt.set_input(input);
        let prompt_str_res = serde_json::to_string(&self.prompt);
        if prompt_str_res.is_err() {
            log::error!(
                "[FunctionNameIdentifier/function_name_in_line] Unable to serialize prompt: {:?}",
                prompt_str_res.expect_err("Empty error in prompt_str_res"));
                return None;
        }
        let prompt_str = prompt_str_res.expect("Uncaught error in prompt_str_res");
        let final_prompt = format!("{}\nOutput - ", &prompt_str);
        log::debug!("[FunctionNameIdentifier/function_name_in_line] code_line: {}", code_line);
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
        if func_name.get_status() != "valid" || func_name.get_name().is_empty() {
            log::debug!("[FunctionNameIdentifier/function_name_in_line] Invalid name: {:#?}", func_name);
            return None;
        }
        self.cached_output.insert(code_line.trim().to_string(), func_name.clone());
        return Some(func_name);
    }
}