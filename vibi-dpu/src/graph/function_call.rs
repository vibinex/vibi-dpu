use std::{collections::{HashMap, HashSet}, io::BufReader, path::{Path, PathBuf}, process::{Command, Stdio}};

use serde::{Deserialize, Serialize};
use std::io::BufRead;
use crate::utils::review::Review;

use super::{file_imports::ImportDefOutput, function_name::FunctionDefinition, gitops::HunkDiffLines, utils::{call_llm_api, detect_language, numbered_content, read_file, strip_json_prefix}};

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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCall {
    line_number: u32,
    function_name: String,
}

impl FunctionCall {
    pub fn function_name(&self) -> &String {
        &self.function_name
    }

    pub fn line_number(&self) -> &u32 {
        &self.line_number
    }
}

// Output schema structure
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCallsOutput {
    function_calls: Vec<FunctionCall>,
    notes: Option<String>,
}

impl FunctionCallsOutput {
    pub fn function_calls(&self) -> &Vec<FunctionCall> {
        return &self.function_calls
    }

    pub fn trim_empty_function_calls(&mut self) {
        self.function_calls.retain(|func_call| !func_call.function_name().is_empty() && func_call.function_name().len() > 3);
    }
}

pub fn associate_function_calls(
    function_definitions: &Vec<FunctionDefinition>,
    function_calls: &Vec<FunctionCall>,
) -> HashMap<FunctionDefinition, Vec<FunctionCall>> {
     // Sort function definitions by line number
    let mut sorted_definitions = function_definitions.clone();
    sorted_definitions.sort_by_key(|def| def.line_number);

    // Sort function calls by line number
    let mut sorted_calls = function_calls.clone();
    sorted_calls.sort_by_key(|call| call.line_number);

    let mut result: HashMap<FunctionDefinition, Vec<FunctionCall>> = HashMap::new();

    // Use a Peekable iterator for the function definitions
    let mut definition_iter = sorted_definitions.iter().peekable();

    for call in sorted_calls {
        while let Some(current_def) = definition_iter.peek().cloned() {
            // Check the next definition's line number, if it exists
            let next_def_line_opt = definition_iter.clone().peek().and_then(|_| {
                let mut temp_iter = definition_iter.clone();
                temp_iter.next(); // Skip the current one
                temp_iter.peek().map(|next_def| next_def.line_number)
            });

            if call.line_number as usize >= current_def.line_number
                && (next_def_line_opt.is_none()
                || (call.line_number as usize) < next_def_line_opt.expect("Empty next_def_line_opt"))
            {
                // Add the function call to the current definition
                result
                    .entry((*current_def).clone())
                    .or_insert_with(Vec::new)
                    .push(call);
                break;
            }

            // Move to the next definition if the call doesn't belong to the current one
            definition_iter.next();
        }
    }
    result
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
    prompt: JsonStructure,
    chunk_size: usize
}

impl FunctionCallIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_function_calls");
        if system_prompt_opt.is_none() {
            log::error!("[function_calls_in_chunk] Unable to read prompt_function_calls");
            return None;
        }
        let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
        let prompt_json_res = serde_json::from_str(&system_prompt_lines);
        if prompt_json_res.is_err() {
            log::error!("[FunctionCallIdentifier/new] Unable to deserialize prompt_json: {:?}",
                prompt_json_res.expect("Empty prompt_json_res"));
            return None;
        }
        let prompt_json: JsonStructure = prompt_json_res.expect("Empty error in prompt_json_res");
        return Some(Self { prompt: prompt_json, chunk_size: 30});
    }

    pub async fn functions_in_file(&mut self, filepath: &PathBuf, lang: &str) -> Option<FunctionCallsOutput> {
        // concatenate functioncallsoutput for all chunks
        let mut all_func_calls: FunctionCallsOutput = FunctionCallsOutput{ function_calls: vec![], notes: None };
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
            log::debug!("[FunctionCallIdentifier/functions_in_file] chunk = {}", &chunk_str);
            if let Some(mut func_calls) = self.functions_in_chunk(&chunk_str, filepath, lang).await {
                log::debug!("[FunctionCallIdentifier/functions_in_file] func_calls = {:?}", &func_calls);
                all_func_calls.function_calls.append(&mut func_calls.function_calls);
            }
        }
        if all_func_calls.function_calls.is_empty() {
            return None;
        }
        return Some(all_func_calls);
    }

    async fn functions_in_chunk(&mut self, chunk: &str, filepath: &PathBuf, lang: &str) -> Option<FunctionCallsOutput> {
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
        let mut func_calls: FunctionCallsOutput = deserialized_response.expect("Empty error in deserialized_response");
        func_calls.trim_empty_function_calls();
        if func_calls.function_calls().is_empty() {
            return None;
        }
        return Some(func_calls);
    }

    pub async fn function_calls_in_hunks(&mut self, filepath: &PathBuf, lang: &str, diff_hunks: &Vec<HunkDiffLines>) -> Option<Vec<(HunkDiffLines, FunctionCallsOutput)>> {
        let file_contents_res = std::fs::read_to_string(filepath.clone());
        if file_contents_res.is_err() {
            log::error!(
                "[FunctionCallIdentifier/function_calls_in_hunks] Unable to read file: {:?}, error: {:?}",
                &filepath, file_contents_res.expect_err("Empty error in file_contents_res")
            );
            return None;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_contents_res");
        let numbered_content = numbered_content(file_contents);
        let mut hunk_func_pairs: Vec<(HunkDiffLines, FunctionCallsOutput)> = Vec::new();
        for diff_hunk in diff_hunks {
            // TODO - what about full files?
            let batch_size = 30;
            let mut hunk_calls: FunctionCallsOutput = FunctionCallsOutput{ function_calls: vec![], notes: None };
            for start in (diff_hunk.start_line().to_owned()..diff_hunk.end_line().to_owned()).step_by(batch_size) {
                let end = usize::min(start + batch_size, diff_hunk.end_line().to_owned() + 1);
                log::debug!("[function_calls_in_hunks] start = {:?}, end = {:?}", &start, &end);
                let chunk_slice = &numbered_content[start..end];
                let chunk_str = chunk_slice.join("\n");
                if let Some(mut func_calls) = self.functions_in_chunk(&chunk_str, filepath, lang).await {
                    log::debug!("[FunctionCallIdentifier/function_calls_in_hunks] chunk = {:?}", &func_calls);
                    hunk_calls.function_calls.append(&mut func_calls.function_calls);
                }
            }
            if !hunk_calls.function_calls().is_empty() {
                hunk_func_pairs.push((diff_hunk.clone(), hunk_calls));
            }
        }
        log::debug!("[FunctionCallIdentifier/function_calls_in_hunks] hunk_func_pairs = {:?}", &hunk_func_pairs);
        if hunk_func_pairs.is_empty() {
            None
        } else {
            Some(hunk_func_pairs)
        }
    }
    
}

pub fn function_calls_search(review: &Review, function_name: &str, lang: &str) -> Option<HashMap<String, Vec<(usize, String)>>>{
    let pattern = format!(r"{}\([^\)]*\)", function_name); // Regex pattern for the specific function call
    let directory = review.clone_dir();                    // The directory to search in

    // Spawn the ripgrep process, adding `-n` for line numbers and `-H` for filename with matches
    let rg_command_res = Command::new("rg")
        .arg("-n")               // Print line numbers for matching lines
        .arg("-H")               // Print the filename with matches
        .arg("-e")               // Use regular expression
        .arg(pattern)            // The regex pattern for function calls
        .arg(directory)          // Directory to search
        .stdout(Stdio::piped())  // Pipe the output
        .spawn();                // Spawn the ripgrep process

    if rg_command_res.is_err() {
        log::error!(
            "[function_calls_search] error in rg command: {:?}",
            rg_command_res.expect_err("Empty error in rg_command_res")
        );
        return None;
    }

    let rg_command = rg_command_res.expect("Uncaught error in rg_command_res");
    // Capture the stdout of ripgrep
    let stdout = rg_command.stdout.expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    // Use a HashSet to avoid duplicate entries (file, line_number)
    let mut results: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    // Read the output line by line
    for line in reader.lines() {
        if let Ok(output) = line {
            // Each line has the format: "filename:line_number:match"
            let parts: Vec<&str> = output.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let file = parts[0].to_string();
                if let Some(file_lang) = detect_language(&file) {
                    if let Ok(line_number) = parts[1].parse::<usize>() {
                        let matching_line = parts[2].to_string();
                        if let Some(results_vec) = results.get_mut(&file) {
                            results_vec.push((line_number, matching_line));
                        } else {
                            let mut results_vec = Vec::new();
                            results_vec.push((line_number, matching_line));
                            results.insert(file, results_vec);
                        }
                    }
                }
            }
        }
    }

    Some(results)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCallValidatorInput {
    code_line: String,               // A line of code that potentially contains the function call or object usage
    function_or_object_name: String, // The name of the function or object being used
    file_path: String,               // The file path or module from which the function or object is imported
    import_statement: String,        // The actual import statement for the function or object
    language: String,                // The programming language of the code
}

#[derive(Serialize, Deserialize, Debug)]
struct FunctionCallValidatorOutputSchema {
    is_valid_usage: String,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCallValidatorOutput {
    is_valid_usage: bool,            // True if the line is a valid usage of the function or object, false otherwise
    status: String,                  // Status: "valid", "no_match", "invalid_input", or "insufficient_context"
}

impl FunctionCallValidatorOutput {
    pub fn is_valid_function_call(&self) -> bool {
        if self.status == "valid" && self.is_valid_usage {
            return true;
        }
        return false;
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FunctionCallValidatorInstructions {
    input_schema: FunctionCallValidatorInput,       // Schema for the input
    output_schema: FunctionCallValidatorOutputSchema,     // Schema for the output
    task_description: String,        // Description of the task
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCallValidatorPrompt {
    instructions: FunctionCallValidatorInstructions,      // Instructions for the LLM
    sample_input: FunctionCallValidatorInput,       // Example of the input
    expected_output: FunctionCallValidatorOutput,   // Example of the expected output
    input: Option<FunctionCallValidatorInput>
}

impl FunctionCallValidatorPrompt {
    pub fn set_input(&mut self, input: FunctionCallValidatorInput) {
        self.input = Some(input);
    }
}

#[derive(Debug)]
pub struct FunctionCallValidator {
    prompt: FunctionCallValidatorPrompt
}

impl FunctionCallValidator {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_function_call_validator");
        if system_prompt_opt.is_none() {
            log::debug!("[FunctionCallValidator/new] Unable to read prompt_function_call_validator");
            return None;
        }
        let system_prompt_str = system_prompt_opt.expect("Empty system_prompt_opt");
        let sys_prompt_struct_res = serde_json::from_str(&system_prompt_str);
        if sys_prompt_struct_res.is_err() {
            log::debug!("[FunctionCallValidator/new] Unable to deserialize sys prompt: {:?}",
                sys_prompt_struct_res.expect_err("Empty error"));
            return None;
        }
        let sys_prompt_struct: FunctionCallValidatorPrompt = sys_prompt_struct_res.expect("Uncaught error in sys_prompt_struct_res");
        return Some(Self {
            prompt: sys_prompt_struct
        });
    }

    pub async fn valid_func_calls_in_file(&mut self, file_path_buf: &PathBuf, lang: &str, 
        func_name: &str, func_call_line: &str, import_def: &ImportDefOutput) -> bool {
        let file_contents_res = std::fs::read_to_string(file_path_buf);
        if file_contents_res.is_err() {
            let e = file_contents_res.expect_err("Empty error in file_content_res");
            log::error!("[ImportLinesIdentifier/import_lines_range_in_file] Unable to read file: {:?}, error: {:?}", file_path_buf, e);
            return false;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
        let numbered_content = numbered_content(file_contents);
        let import_line_opt = import_def.line_range();
        if import_line_opt.is_none() {
            log::error!(
                "[FunctionCallValidator/valid_func_calls_in_file] No line range in import def: {:#?}",
                &import_def);
            return false;
        }
        let import_line_range = import_line_opt.as_ref().expect("EMpty import_line_opt");
        let import_content = numbered_content[
            (import_line_range.start_line().to_owned() - 1)..(import_line_range.end_line().to_owned() - 1)
            ].join("\n");
        let call_validity = self.valid_func_calls(
            &import_content, &func_call_line, func_name,
            &file_path_buf.to_string_lossy(), lang).await;
        return call_validity;
    }

    async fn valid_func_calls(&mut self, import_content: &str, code_chunk: &str, 
        func_name: &str, file_path: &str, lang: &str) -> bool
    {
        let func_call_validator_input = FunctionCallValidatorInput {
            code_line: code_chunk.to_owned(),
            function_or_object_name: func_name.to_owned(),
            file_path: file_path.to_owned(),
            import_statement: import_content.to_owned(),
            language: lang.to_owned(),
        };
        self.prompt.set_input(func_call_validator_input);
        let func_call_validator_prompt_res = serde_json::to_string(&self.prompt);
        if func_call_validator_prompt_res.is_err() {
            log::debug!(
                "[FunctionCallValidator/valid_func_calls] Unable to deserialize prompt struct: {:?}",
                func_call_validator_prompt_res.expect_err("Empty error in func_call_validator_prompt_res"));
            return false;
        }
        let func_call_validator_prompt_str = func_call_validator_prompt_res.expect("Uncaught error in import_def_prompt_str_res");
        let prompt_str = format!("{}\nOutput -", &func_call_validator_prompt_str);
        log::debug!("[FunctionCallValidator/valid_func_calls] prompt_str: {}", &prompt_str);
        let validator_str_opt = call_llm_api(prompt_str).await;
        // deserialize output
        if validator_str_opt.is_none() {
            log::debug!("[FunctionCallValidator/valid_func_calls] Unable to call llm api");
            return false;
        }
        let mut validator_str = validator_str_opt.expect("Empty validator_str_opt");
        if let Some(stripped_json) = strip_json_prefix(&validator_str) {
            validator_str = stripped_json.to_string();
        }
        let validator_res = serde_json::from_str(&validator_str);
        if validator_res.is_err() {
            log::debug!(
                "[FunctionCallValidator/valid_func_calls] Unable to deserialize func call validator output : {:?}",
                validator_res.expect_err("Empty error in validator_res"));
            return false;
        }
        let call_validator_output: FunctionCallValidatorOutput = validator_res.expect("Uncaught error in import_def_res");
        log::debug!("[FunctionCallValidator/valid_func_calls] import_def: {:?}", &call_validator_output);
        if !call_validator_output.is_valid_function_call() {
            log::debug!(
                "[FunctionCallValidator/valid_func_calls] Invalid function call: {:#?}, for chunk: {} ",
                &call_validator_output, code_chunk);
            return false;
        }
        return true;
    }
}