use serde::{Deserialize, Serialize};

use super::utils::{call_llm_api, get_specific_lines, read_file};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FunctionLineMap {
    pub name: String,
    pub line_start: i32,
    pub line_end: i32,
    pub inside: String
}

impl FunctionLineMap {
    pub fn new(name: &str, line_start: i32, line_end: i32, inside: &str) -> Self {
        FunctionLineMap {
            name: name.to_string(),
            line_start,
            line_end,
            inside: inside.to_string(),
        }
    }
}

pub async fn extract_function_lines(numbered_content: &str, file_name: &str) -> Option<Vec<FunctionLineMap>> {
    let system_prompt_opt = read_file("/app/prompt_function_lines");
    if system_prompt_opt.is_none() {
        log::error!("[mermaid_comment] Unable to read system prompt");
        return None;
    }
    let system_prompt = system_prompt_opt.expect("Empty system_prompt_opt");
    let mut flines = Vec::<FunctionLineMap>::new();
    // split numbered content and start for loop
    // Split the numbered_content into lines
    let lines: Vec<&str> = numbered_content.lines().collect();
    
    // Determine the batch size
    let batch_size = 30;
    
    // Iterate over the lines in chunks of batch_size
    for chunk in lines.chunks(batch_size) {
        // create prompt
        // call llm api
        let prompt = format!(
            "{}\n\n### User Message\nInput -\n{}\n{}\n\nOutput -",
            system_prompt,
            file_name,
            chunk.join("\n")
        );
        match call_llm_api(prompt).await {
            None => {
                log::error!("[mermaid_comment] Failed to call LLM API");
                return None;
            }
            Some(llm_response) => {
                // parse response to FunctionLineMap
                let flinemap_res = serde_json::from_str(&llm_response);
                if flinemap_res.is_err() {
                    let e = flinemap_res.expect_err("Empty error in flinemap_res");
                    log::error!(
                        "[extract_function_lines] Unable to deserialize llm response: {:?}, error - {:?}",
                        &llm_response, e);
                    continue;
                }
                let flinemap = flinemap_res.expect("Uncaught error in flinemap_res");
                // add to vec
                flines.push(flinemap);
            }
        }   
    }
    if flines.is_empty() {
        log::error!("[extract_function_lines] No functions extracted");
        return None;
    }
    let parsed_flines = process_flinemap_response(&flines);
    return Some(parsed_flines);
}

fn process_flinemap_response(flines: &Vec<FunctionLineMap>) -> Vec<FunctionLineMap> {
    let mut resolved_flines = vec![];
    let mut unfinished_function = FunctionLineMap::new("", 0, 0, "");
    for flinemap in flines {
        if flinemap.line_end == -1 {
            unfinished_function = flinemap.clone();
            continue;
        }
        if flinemap.name == "unknown" {
            if unfinished_function.line_end == -1 {
                unfinished_function.line_end = flinemap.line_start;
                resolved_flines.push(unfinished_function.clone());
                continue;
            }
        }
        resolved_flines.push(flinemap.to_owned());
    }

    return resolved_flines;
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct CalledFunction {
    pub name: String,
    pub line: usize
}

pub async fn extract_function_calls(hunk_lines: &Vec<(usize, usize)>, numbered_content: &str, file_name: &str) -> Option<Vec<CalledFunction>> {
    // extract hunk lines from numbered content
    let user_prompt = get_specific_lines(
        hunk_lines.to_owned(), numbered_content);
    // prepare prompt and call llm api
    let system_prompt_opt = read_file("/app/prompt_function_calls");
    if system_prompt_opt.is_none() {
        log::error!("[extract_function_calls] Unable to read system prompt /app/prompt_function_calls");
        return None;
    }
    let system_prompt = system_prompt_opt.expect("Empty system_prompt_opt");
    let prompt = format!(
        "{}\n\n### User Message\nInput -\n{}\n{}\n\nOutput -",
        &system_prompt,
        file_name,
        &user_prompt
    );
    match call_llm_api(prompt).await {
        None => {
            log::error!("[extract_function_calls] Failed to call LLM API");
            return None;
        }
        Some(llm_response) => {
            // parse response and return CalledFunction Vec
            // optional - paginate
            let called_functions_res = serde_json::from_str(&llm_response);
            if called_functions_res.is_err() {
                let e = called_functions_res.expect_err("Empty error in called_functions_res");
                log::error!(
                    "[extract_function_calls] Unable to deserialize called_functions: {:?}", e);
                return None;
            }
            let called_functions: Vec<CalledFunction> = called_functions_res.expect("Uncaught error in called_functions_res");
            return Some(called_functions);
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct CalledFunctionPath {
    pub path: String,
    pub function_name: String,
    line: u32
}
pub async fn extract_function_import_path(called_funcs: &Vec<CalledFunction>, numbered_content: &str, file_name: &str) -> Option<Vec<CalledFunctionPath>> {
    let system_prompt_opt = read_file("/app/prompt_function_call_path");
    if system_prompt_opt.is_none() {
        log::error!("[extract_function_calls] Unable to read system prompt /app/prompt_function_calls");
        return None;
    }
    let system_prompt = system_prompt_opt.expect("Empty system_prompt_opt");
    let mut user_prompt = String::new();
    // search in numbered content for called functions
    let numbered_lines: Vec<&str> = numbered_content.lines().collect();
    for called_func in called_funcs {
        // extract hunk lines from numbered content or get it as input
        let first_occurence_line_opt = find_first_occurence(&numbered_lines, &called_func.name);
        if first_occurence_line_opt.is_none() {
            log::debug!("[extract_function_import_path] No first occurence found for: {}", &called_func.name);
            continue;
        }
        let first_occurence_line = first_occurence_line_opt.expect("Empty first_occurence_line_opt");
        user_prompt.push_str(first_occurence_line.as_str());
        user_prompt.push_str("\n");
        user_prompt.push_str(numbered_lines[called_func.line]);
        user_prompt.push_str("\n");
    }
    // prepare prompt with hunk lines and occurences and call llm api
    let prompt = format!(
        "{}\n\n### User Message\nInput -\n{}\n{}\n\nOutput -",
        &system_prompt,
        file_name,
        &user_prompt
    );
    // extract CalledFunctionPath vec from responses and return
    match call_llm_api(prompt).await {
        None => {
            log::error!("[extract_function_import_path] Failed to call LLM API");
            return None;
        }
        Some(llm_response) => {
            let called_functions_res = serde_json::from_str(&llm_response);
            if called_functions_res.is_err() {
                let e = called_functions_res.expect_err("Empty error in called_functions_res");
                log::error!(
                    "[extract_function_calls] Unable to deserialize called_functions: {:?}", e);
                return None;
            }
            let called_func_paths: Vec<CalledFunctionPath> = called_functions_res.expect("Uncaught error in called_functions_res");
            return Some(called_func_paths);
        }
    }
    // optional - paginate
}

fn find_first_occurence(lines: &Vec<&str>, func_name: &str) -> Option<String> {
    for line in lines {
        if line.contains(func_name) {
            return Some(line.to_owned().to_owned());
        }
    }
    return None;
}