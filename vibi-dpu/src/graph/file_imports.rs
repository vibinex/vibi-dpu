use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{graph::utils::numbered_content, utils::review::Review};

use super::utils::{all_code_files, call_llm_api, read_file, strip_json_prefix};


#[derive(Serialize, Deserialize, Debug)]
struct InputSchema {
    function_name: String,
    code_chunk: String,
    language: String,
    file_path: String,
}

// Output schema structure for matching import
#[derive(Serialize, Deserialize, Debug)]
pub struct MatchingImport {
    line_number: u32,
    import_statement: String,
    possible_file_path: String,
}

impl MatchingImport {
    pub fn possible_file_path(&self) -> &String {
        &self.possible_file_path
    }
}

// Full output schema structure
#[derive(Serialize, Deserialize, Debug)]
pub struct ImportPathOutput {
    matching_import: MatchingImport,
    notes: Option<String>,
}

impl ImportPathOutput {
    pub fn get_matching_import(&self) -> &MatchingImport {
        &self.matching_import
    }
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
    function_name: String,
    code_chunk: String,
    language: String,
    file_path: String,
}

// Description of output schema
#[derive(Serialize, Deserialize, Debug)]
struct OutputSchemaDescription {
    matching_import: MatchingImportDescription,
    notes: String,
}

// Description for matching import schema
#[derive(Serialize, Deserialize, Debug)]
struct MatchingImportDescription {
    line_number: String,
    import_statement: String,
    possible_file_path: String,
}

// Complete structure for JSON input and output
#[derive(Serialize, Deserialize, Debug)]
struct ImportPathJsonStructure {
    instructions: Instructions,
    sample_input: InputSchema,
    expected_output: ImportPathOutput,
    input: Option<InputSchema>,
}

impl ImportPathJsonStructure {
    fn set_input(&mut self, input_schema: InputSchema) {
        self.input = Some(input_schema);
    }
}

pub struct ImportIdentifier {
    prompt_struct: ImportPathJsonStructure
}

impl ImportIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_import_file_path");
        if system_prompt_opt.is_none() {
            log::debug!("[ImportIdentifier/new] Unable to read prompt_import_file");
            return None;
        }
        let system_prompt_str = system_prompt_opt.expect("Empty system_prompt_opt");
        let sys_prompt_struct_res = serde_json::from_str(&system_prompt_str);
        if sys_prompt_struct_res.is_err() {
            log::debug!("[ImportIdentifier/new] Unable to deserialize sys prompt: {:?}",
                sys_prompt_struct_res.expect_err("Empty error"));
            return None;
        }
        let sys_prompt_struct: ImportPathJsonStructure = sys_prompt_struct_res.expect("Uncaught error in sys_prompt_struct_res");
        return Some(Self {
            prompt_struct: sys_prompt_struct
        });
    }
    async fn get_import_path(&mut self, func_name: &str, lang: &str, file_path: &str, chunk: &str) -> Option<ImportPathOutput>{
        // create prompt
        let input_schema = InputSchema {
            function_name: func_name.to_string(),
            code_chunk: chunk.to_string(),
            language: lang.to_string(),
            file_path: file_path.to_string(),
        };
        self.prompt_struct.set_input(input_schema);
        // call api
        let import_struct_str_res = serde_json::to_string(&self.prompt_struct);
        if import_struct_str_res.is_err() {
            log::debug!(
                "[ImportIdentifier/get_import_path] Unable to deserialize prompt struct: {:?}",
                import_struct_str_res.expect_err("Empty error in import_struct_str_res"));
            return None;
        }
        let import_struct_str = import_struct_str_res.expect("Uncaught error in import_struct_str_res");
        let prompt_str = format!("{}\nOutput -", &import_struct_str);
        log::debug!("[ImportIdentifier/get_import_path] code_chunk: {}", chunk);
        let import_path_opt = call_llm_api(prompt_str).await;
        // deserialize output
        if import_path_opt.is_none() {
            log::debug!("[ImportIdentifier/get_import_path] Unable to call llm api");
            return None;
        }
        let mut import_path_str = import_path_opt.expect("Empty import_path_opt");
        if let Some(stripped_json) = strip_json_prefix(&import_path_str) {
            import_path_str = stripped_json.to_string();
        }
        let import_path_res = serde_json::from_str(&import_path_str);
        if import_path_res.is_err() {
            log::debug!(
                "[ImportIdentifier/get_import_path] Unable to deserialize import path output : {:?}",
                import_path_res.expect_err("Empty error in import_path_res"));
            return None;
        }
        let import_path: ImportPathOutput = import_path_res.expect("Unacaught error in import_path_res");
        log::debug!("[ImportIdentifier/get_import_path] import_path: {:?}", &import_path);
        if import_path.get_matching_import().possible_file_path().is_empty() {
            log::debug!("[ImportIdentifier/get_import_path] import path not valid: {:#?}", &import_path);
            return None;
        }
        return Some(import_path);
    }

    pub async fn get_import_path_file(&mut self, file_path: &str, lang: &str, func_name: &str) -> Option<ImportPathOutput> {
        let file_contents_res = std::fs::read_to_string(file_path);
        if file_contents_res.is_err() {
            let e = file_contents_res.expect_err("Empty error in file_content_res");
            log::error!("[get_import_lines] Unable to read file: {:?}, error: {:?}", file_path, e);
            return None;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
        let numbered_content = numbered_content(file_contents);
        let chunks = numbered_content.chunks(20);
        for chunk in chunks {
            let chunk_str = chunk.join("\n");
            let import_path_opt = self.get_import_path(func_name, lang, file_path, &chunk_str).await;
            if import_path_opt.is_some() {
                return import_path_opt;
            }
        }
        return None;
    }
}


#[derive(Serialize, Deserialize, Debug)]
struct ImportLinesInput {
    code_chunk: String, // A chunk of code from a source file
    language: String,   // The programming language of the code
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportRangeOutputSchema {
    start_line: String,
    end_line: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportLinesRangeOutputSchema {
    import_ranges: Vec<ImportRangeOutputSchema>,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportRange {
    start_line: usize, // The starting line number of the import range (inclusive)
    end_line: usize,   // The ending line number of the import range (inclusive)
}

impl ImportRange {
    pub fn start_line(&self) -> &usize {
        &self.start_line
    }

    pub fn end_line(&self) -> &usize {
        &self.end_line
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportLinesRange {
    import_ranges: Vec<ImportRange>, // List of ranges containing import statements
    status: String,                  // Status: "valid", "no_imports", or "insufficient_context"
}

impl ImportLinesRange {
    pub fn import_ranges(&self) -> &Vec<ImportRange> {
        &self.import_ranges
    }

    pub fn valid_status(&self) -> bool {
        if self.status != "valid" {
            return false
        }
        return true;
    }

    pub fn remove_outside_range(&mut self, start_idx: usize, end_idx: usize) {
        self.import_ranges.retain(|range| {
            range.start_line >= start_idx && range.end_line <= end_idx
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportLinesInstructions {
    input_schema: ImportLinesInput,   // Schema for the input
    output_schema: ImportLinesRangeOutputSchema, // Schema for the output
    task_description: String,    // Description of the task
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportLinesPrompt {
    instructions: ImportLinesInstructions,  // Instructions for the LLM
    sample_input: ImportLinesInput,   // Sample input example
    expected_output: ImportLinesRange,
    input: Option<ImportLinesInput>
}

impl ImportLinesPrompt {
    pub fn set_input(&mut self, input: ImportLinesInput) {
        self.input = Some(input);
    }
}

#[derive(Debug)]
pub struct ImportLinesIdentifier {
    prompt: ImportLinesPrompt
}

impl ImportLinesIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_import_lines");
        if system_prompt_opt.is_none() {
            log::debug!("[ImportLinesIdentifier/new] Unable to read prompt_import_lines");
            return None;
        }
        let system_prompt_str = system_prompt_opt.expect("Empty system_prompt_opt");
        let sys_prompt_struct_res = serde_json::from_str(&system_prompt_str);
        if sys_prompt_struct_res.is_err() {
            log::debug!("[ImportLinesIdentifier/new] Unable to deserialize sys prompt: {:?}",
                sys_prompt_struct_res.expect_err("Empty error"));
            return None;
        }
        let sys_prompt_struct: ImportLinesPrompt = sys_prompt_struct_res.expect("Uncaught error in sys_prompt_struct_res");
        return Some(Self {
            prompt: sys_prompt_struct
        });
    }

    pub async fn import_lines_range_in_file(&mut self, file_path: &PathBuf, lang: &str) -> Option<Vec<ImportLinesRange>> {
        let file_contents_res = std::fs::read_to_string(file_path);
        if file_contents_res.is_err() {
            let e = file_contents_res.expect_err("Empty error in file_content_res");
            log::error!("[ImportLinesIdentifier/import_lines_range_in_file] Unable to read file: {:?}, error: {:?}", file_path, e);
            return None;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
        let numbered_content = numbered_content(file_contents);
        let chunk_size: usize = 20;
        let chunks = numbered_content.chunks(chunk_size);
        let mut results = Vec::<ImportLinesRange>::new();
        for (idx, chunk) in chunks.enumerate() {
            let chunk_str = chunk.join("\n");
            let start_idx = chunk_size*idx;
            let end_idx = start_idx + chunk_size - 1;
            let import_lines_opt = self.import_lines_in_chunk(&chunk_str, lang, start_idx, end_idx).await;
            if import_lines_opt.is_some() {
                let import_lines = import_lines_opt.expect("Empty import_lines_opt");
                if import_lines.status == "valid" {
                    results.push(import_lines);
                }
            }
        }
        if results.is_empty() {
            return None;
        }
        return Some(results);
    }

    async fn import_lines_in_chunk(&mut self, chunk_code: &str, lang: &str, start_idx: usize, end_idx: usize) -> Option<ImportLinesRange> {
        let prompt_input = ImportLinesInput{
            code_chunk: chunk_code.to_string(),
            language: lang.to_string(),
        };
        self.prompt.set_input(prompt_input);
        let import_lines_prompt_str_res = serde_json::to_string(&self.prompt);
        if import_lines_prompt_str_res.is_err() {
            log::debug!(
                "[ImportLinesIdentifier/import_lines_in_chunk] Unable to deserialize prompt struct: {:?}",
                import_lines_prompt_str_res.expect_err("Empty error in import_lines_prompt_str_res"));
            return None;
        }
        let import_lines_prompt_str = import_lines_prompt_str_res.expect("Uncaught error in import_lines_prompt_str_res");
        let prompt_str = format!("{}\nOutput -", &import_lines_prompt_str);
        log::debug!("[ImportLinesIdentifier/import_lines_in_chunk] code_chunk: {}", chunk_code);
        let import_lines_str_opt = call_llm_api(prompt_str).await;
        // deserialize output
        if import_lines_str_opt.is_none() {
            log::debug!("[ImportLinesIdentifier/import_lines_in_chunk] Unable to call llm api");
            return None;
        }
        let mut import_lines_str = import_lines_str_opt.expect("Empty import_lines_str_opt");
        if let Some(stripped_json) = strip_json_prefix(&import_lines_str) {
            import_lines_str = stripped_json.to_string();
        }
        let import_lines_res = serde_json::from_str(&import_lines_str);
        if import_lines_res.is_err() {
            log::debug!(
                "[ImportLinesIdentifier/import_lines_in_chunk] Unable to deserialize import lines output : {:?}",
                import_lines_res.expect_err("Empty error in import_lines_res"));
            return None;
        }
        let mut import_lines: ImportLinesRange = import_lines_res.expect("Uncaught error in import_lines_res");
        log::debug!("[ImportLinesIdentifier/import_lines_in_chunk] import_lines: {:?}", &import_lines);
        if !import_lines.valid_status() || import_lines.import_ranges().is_empty() {
            log::debug!(
                "[ImportLinesIdentifier/import_lines_in_chunk] Invalid or empty lines: {:#?}, for chunk: {} ",
                &import_lines, chunk_code);
            return None;
        }
        import_lines.remove_outside_range(start_idx, end_idx);
        return Some(import_lines);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportDefInput {
    code_chunk: String,                 // A chunk of code that may contain import statements
    function_or_object_name: String,    // The name of the function or object being imported
    file_path: String,                  // The file path or module from which the import occurs
    language: String,                   // The programming language of the code
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineRange {
    start_line: usize, // The starting line number where the import occurs (starting from 1)
    end_line: usize,   // The ending line number where the import occurs (starting from 1)
}

impl LineRange {
    pub fn start_line(&self) -> &usize {
        &self.start_line
    }
    pub fn end_line(&self) -> &usize {
        &self.end_line
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportDefOutputSchema {
    line_range: String,         // The line number where the import statement occurs (if found)
    status: String,                     // Status: "valid", "no_match", "invalid_input", or "insufficient_context"
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportDefOutput {
    line_range: Option<LineRange>,         // The line number where the import statement occurs (if found)
    status: String,                     // Status: "valid", "no_match", "invalid_input", or "insufficient_context"
}

impl ImportDefOutput {
    pub fn valid_status(&self) -> bool {
        if self.status != "valid" {
            return false;
        }
        return true;
    }

    pub fn line_range(&self) -> &Option<LineRange> {
        &self.line_range
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportDefInstructions {
    input_schema: ImportDefInput,
    output_schema: ImportDefOutputSchema,
    task_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImportDefPrompt {
    instructions: ImportDefInstructions,
    sample_input: ImportDefInput,
    expected_output: ImportDefOutput,
    input: Option<ImportDefInput>
}

impl ImportDefPrompt {
    pub fn set_input(&mut self, input: ImportDefInput) {
        self.input = Some(input);
    }
}

pub struct ImportDefIdentifier {
    prompt: ImportDefPrompt
}

impl ImportDefIdentifier {
    pub fn new() -> Option<Self> {
        let system_prompt_opt = read_file("/app/prompts/prompt_import_def");
        if system_prompt_opt.is_none() {
            log::debug!("[ImportDefIdentifier/new] Unable to read prompt_import_def");
            return None;
        }
        let system_prompt_str = system_prompt_opt.expect("Empty system_prompt_opt");
        let sys_prompt_struct_res = serde_json::from_str(&system_prompt_str);
        if sys_prompt_struct_res.is_err() {
            log::debug!("[ImportDefIdentifier/new] Unable to deserialize sys prompt: {:?}",
                sys_prompt_struct_res.expect_err("Empty error"));
            return None;
        }
        let sys_prompt_struct: ImportDefPrompt = sys_prompt_struct_res.expect("Uncaught error in sys_prompt_struct_res");
        return Some(Self {
            prompt: sys_prompt_struct
        });
    }

    pub async fn identify_import_def(&mut self, file_path: &PathBuf, func_name: &str, lang: &str, import_hunks: &Vec<ImportLinesRange>) -> Option<ImportDefOutput> {
        let file_contents_res = std::fs::read_to_string(file_path);
        if file_contents_res.is_err() {
            let e = file_contents_res.expect_err("Empty error in file_content_res");
            log::error!("[ImportLinesIdentifier/import_lines_range_in_file] Unable to read file: {:?}, error: {:?}", file_path, e);
            return None;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
        let numbered_content = numbered_content(file_contents);
        let numbered_content_len = numbered_content.len();
        for import_range in import_hunks {
            let chunk_ranges = import_range.import_ranges();
            for chunk_range in chunk_ranges {
                let start_idx = chunk_range.start_line().to_owned();
                let end_idx = chunk_range.end_line().to_owned() + 1;  
                if end_idx <= numbered_content_len
                    && start_idx < numbered_content_len 
                {
                    let import_content = &numbered_content[start_idx..end_idx].join("\n");
                    if let Some(import_def) = self.identify_import_in_range(
                        import_content,
                        &file_path.to_string_lossy().to_string(),
                        func_name, lang).await
                        {
                            return Some(import_def);
                        }
                }
            }
        }
        return None;
    }

    async fn identify_import_in_range(&mut self, import_content: &str, file_path: &str, func_name: &str, lang: &str) -> Option<ImportDefOutput> {
        let import_def_input = ImportDefInput{
            code_chunk: import_content.to_string(),
            function_or_object_name: func_name.to_string(),
            file_path: file_path.to_string(),
            language: lang.to_string(),
        };
        self.prompt.set_input(import_def_input);
        let import_def_prompt_str_res = serde_json::to_string(&self.prompt);
        if import_def_prompt_str_res.is_err() {
            log::debug!(
                "[ImportDefIdentifier/identify_import_in_range] Unable to deserialize prompt struct: {:?}",
                import_def_prompt_str_res.expect_err("Empty error in import_lines_prompt_str_res"));
            return None;
        }
        let import_def_prompt_str = import_def_prompt_str_res.expect("Uncaught error in import_def_prompt_str_res");
        let prompt_str = format!("{}\nOutput -", &import_def_prompt_str);
        log::debug!("[ImportDefIdentifier/identify_import_in_range] prompt_str: {}", &prompt_str);
        let import_def_str_opt = call_llm_api(prompt_str).await;
        // deserialize output
        if import_def_str_opt.is_none() {
            log::debug!("[ImportDefIdentifier/identify_import_in_range] Unable to call llm api");
            return None;
        }
        let mut import_def_str = import_def_str_opt.expect("Empty import_def_str_opt");
        if let Some(stripped_json) = strip_json_prefix(&import_def_str) {
            import_def_str = stripped_json.to_string();
        }
        let import_def_res = serde_json::from_str(&import_def_str);
        if import_def_res.is_err() {
            log::debug!(
                "[ImportDefIdentifier/identify_import_in_range] Unable to deserialize import def output : {:?}",
                import_def_res.expect_err("Empty error in import_def_res"));
            return None;
        }
        let import_def: ImportDefOutput = import_def_res.expect("Uncaught error in import_def_res");
        log::debug!("[ImportDefIdentifier/identify_import_in_range] import_def: {:?}", &import_def);
        if !import_def.valid_status() || import_def.line_range().is_none() {
            log::debug!(
                "[ImportDefIdentifier/identify_import_in_range] Invalid or empty def: {:#?}, for chunk: {} ",
                &import_def, import_content);
            return None;
        }
        return Some(import_def);
    }
}
