use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use super::{function_line_range::FunctionDefIdentifier, utils::{call_llm_api, numbered_content, read_file, strip_json_prefix}};

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


// Input Schema
#[derive(Serialize, Deserialize, Debug)]
pub struct DefinitionInputSchema {
    pub code_chunk: String, // A chunk of code with line numbers
    pub language: String,   // The programming language (e.g., "Python", "Rust")
}

// Output Schema
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct FunctionDefinition {
    pub line_number: usize,       // The line number where the structure is defined
    pub structure_name: String, // The name of the function, object, or structure
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DefinitionOutputSchema {
    pub function_definition: Option<FunctionDefinition>, // Optional, in case no definition is found
    pub notes: Option<String>,                           // Optional notes about the result
}

// Full Instruction Set
#[derive(Serialize, Deserialize, Debug)]
pub struct DefinitionInstructions {
    pub input_schema: DefinitionInputSchema,
    pub output_schema: DefinitionOutputSchema,
    pub task_description: String, // Task description as described in the prompt
}

// Example Input
#[derive(Serialize, Deserialize, Debug)]
pub struct DefintionExampleInput {
    pub code_chunk: String, // A code chunk with numbered lines
    pub language: String,   // Programming language of the code
}

// Example Output
#[derive(Serialize, Deserialize, Debug)]
pub struct DefintionExampleOutput {
    pub definitions: Option<Vec<FunctionDefinition>>,
    pub notes: Option<String>,
}

// Top-Level Prompt Struct
#[derive(Serialize, Deserialize, Debug)]
struct DefintionPrompt {
    instructions: DefinitionInstructions,
    sample_input: DefintionExampleInput,
    expected_output: DefintionExampleOutput,
    input: Option<DefintionExampleInput>,
}

impl DefintionPrompt {
    fn set_input(&mut self, input_val: DefintionExampleInput) {
        self.input = Some(input_val);
    }
}


pub struct DefinitionIdentifier {
    prompt: DefintionPrompt,
    validation_prompt: ValidationPrompt
}

impl DefinitionIdentifier {
    pub fn new() -> Option<Self> {
        let def_system_prompt_opt = read_file("/app/prompts/prompt_definition");
        if def_system_prompt_opt.is_none() {
            log::error!("[DefintionIdentifier/new] Unable to read prompt_definition");
            return None;
        }
        let def_system_prompt_lines = def_system_prompt_opt.expect("Empty def_system_prompt");
        let def_prompt_json_res = serde_json::from_str(&def_system_prompt_lines);
        if def_prompt_json_res.is_err() {
            log::error!("[DefintionIdentifier/new] Unable to deserialize def_prompt_json: {:?}",
                def_prompt_json_res.expect("Empty def_prompt_json_res"));
            return None;
        }
        let def_prompt_json: DefintionPrompt = def_prompt_json_res.expect("Empty error in def_prompt_json_res");

        let system_prompt_validation_opt = read_file("/app/prompts/prompt_valid_function_def");
        if system_prompt_validation_opt.is_none() {
            log::error!("[DefintionIdentifier/new] Unable to read prompt_valid_function_def");
            return None;
        }
        let system_prompt_validation_lines = system_prompt_validation_opt.expect("Empty system_prompt");
        let prompt_validation_json_res = serde_json::from_str(&system_prompt_validation_lines);
        if prompt_validation_json_res.is_err() {
            log::error!("[DefintionIdentifier/new] Unable to deserialize prompt_json: {:?}",
                prompt_validation_json_res.expect("Empty prompt_json_res"));
            return None;
        }
        let prompt_validation_json: ValidationPrompt = prompt_validation_json_res.expect("Empty error in prompt_json_res");
        return Some(Self { prompt: def_prompt_json, validation_prompt: prompt_validation_json});
    }

    pub async fn identify_defs_in_file(&mut self, filepath: &str, lang: &str) -> Vec<FunctionDefinition>  {
        let mut func_def_vals = Vec::<FunctionDefinition>::new();
        // batch up file
        let file_contents_res = std::fs::read_to_string(filepath.clone());
        if file_contents_res.is_err() {
            log::error!(
                "[DefintionIdentifier/identify_defs_in_file] Unable to read file: {:?}, error: {:?}",
                &filepath, file_contents_res.expect_err("Empty error in file_contents_res")
            );
            return func_def_vals;
            // return None;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_contents_res");
        let numbered_content = numbered_content(file_contents);
        let chunk_size = 20;
        let chunks = numbered_content.chunks(chunk_size);
        for chunk in chunks {
            let chunk_str = chunk.join("\n");
            // ask to identify def lines in each batch
            let def_input = DefintionExampleInput{
                code_chunk: chunk_str,
                language: lang.to_string(),
            };
            self.prompt.set_input(def_input);
            let def_prompt_str_res = serde_json::to_string(&self.prompt);
            if def_prompt_str_res.is_err() {
                log::error!(
                    "[DefintionIdentifier/identify_defs_in_file] Unable to serialize prompt: {:?}",
                    def_prompt_str_res.expect_err("Empty error in def_prompt_str_res"));
                    continue;
            }
            let def_prompt_str = def_prompt_str_res.expect("Uncaught error in def_prompt_str_res");
            let def_final_prompt = format!("{}\nOutput - ", &def_prompt_str);
            log::debug!("[DefintionIdentifier/identify_defs_in_file] def prompt: {}", &def_final_prompt);
            let def_prompt_response_opt =  call_llm_api(def_final_prompt).await;
            if def_prompt_response_opt.is_none() {
                log::error!("[DefintionIdentifier/identify_defs_in_file] Unable to call llm for def prompt: {:?}", chunk);
                continue;
            }
            let mut def_prompt_response = def_prompt_response_opt.expect("Empty prompt_response_opt");
            if let Some(stripped_json) = strip_json_prefix(&def_prompt_response) {
                def_prompt_response = stripped_json.to_string();
            }
            let deserialized_def_response = serde_json::from_str(&def_prompt_response);
            if deserialized_def_response.is_err() {
                let e = deserialized_def_response.expect_err("Empty error in deserialized_def_response");
                log::error!("[DefintionIdentifier/identify_defs_in_file] Error in deserializing def response: {:?}", e);
                continue;
            }
            let def_out: DefintionExampleOutput = deserialized_def_response.expect("Empty error in deserialized_def_response");
            if let Some(func_defs) = def_out.definitions {
                for func_def in func_defs {
                    // // ask validator to validate each line
                    // // TODO - incorrect line
                    // let code_line = numbered_content[func_def.line_number-1].to_string();
                    // let validation_input = ValidationPromptInput { code_line: code_line.to_string(), language: lang.to_string() };
                    // self.validation_prompt.set_input(validation_input);
                    // let validation_prompt_str_res = serde_json::to_string(&self.validation_prompt);
                    // if validation_prompt_str_res.is_err() {
                    //     log::error!(
                    //         "[DefintionIdentifier/identify_defs_in_file] Unable to serialize prompt: {:?}",
                    //         validation_prompt_str_res.expect_err("Empty error in validation_prompt_str_res"));
                    //         continue;
                    // }
                    // let validation_prompt_str = validation_prompt_str_res.expect("Uncaught error in validation_prompt_str_res");
                    // let validation_final_prompt = format!("{}\nOutput - ", &validation_prompt_str);
                    // log::debug!("[DefintionIdentifier/identify_defs_in_file] validation code_line: {}", &validation_final_prompt);
                    // let validation_prompt_response_opt =  call_llm_api(validation_final_prompt).await;
                    // if validation_prompt_response_opt.is_none() {
                    //     log::error!("[DefintionIdentifier/identify_defs_in_file] Unable to call llm for validation code line: {:?}", code_line);
                    //     continue;
                    // }
                    // let mut validation_prompt_response = validation_prompt_response_opt.expect("Empty prompt_response_opt");
                    // if let Some(stripped_json) = strip_json_prefix(&validation_prompt_response) {
                    //     validation_prompt_response = stripped_json.to_string();
                    // }
                    // let deserialized_response = serde_json::from_str(&validation_prompt_response);
                    // if deserialized_response.is_err() {
                    //     let e = deserialized_response.expect_err("Empty error in deserialized_response");
                    //     log::error!("[DefintionIdentifier/identify_defs_in_file] Error in deserializing response: {:?}", e);
                    //     continue;
                    // }
                    // let validation_out: ValidationPromptOutput = deserialized_response.expect("Empty error in deserialized_response");
                    // log::debug!("[DefintionIdentifier/identify_defs_in_file] validation response obj: {:#?}", &validation_out);
                    // if !validation_out.is_definition || validation_out.status != "valid" {
                    //     log::debug!("[DefintionIdentifier/identify_defs_in_file] Given code line is not valid function def");
                    //     continue;
                    // }
                    // // add filtered lines and defs to vec
                    func_def_vals.push(func_def);
                }
            }
        }
        // return this vec of defs
        return func_def_vals;
    }
}