use serde::{Deserialize, Serialize};

use super::utils::{call_llm_api, get_specific_lines, read_file};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmFunctionLineMapResponse {
    functions: Option<Vec<FunctionLineMap>>
}

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
    let system_prompt_opt = read_file("/app/prompts/prompt_function_lines");
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
                let mut unparsed_res = llm_response;
                // parse response to FunctionLineMap
                if unparsed_res.contains("```json") {
                    unparsed_res = extract_json_from_llm_response(&unparsed_res);
                }
                let flinemap_opt = clean_and_deserialize(&unparsed_res);
                log::debug!("[extract_function_lines] flinemap_res {:?} ", &flinemap_opt);
                if flinemap_opt.is_none() {
                    log::error!(
                        "[extract_function_lines] Unable to clean and deserialize llm response: {:?}",
                        &unparsed_res);
                    continue;
                }
                let flinemapresp: LlmFunctionLineMapResponse = flinemap_opt.expect("Uncaught error in flinemap_res");
                // add to vec
                if flinemapresp.functions.is_some() {
                    flines.extend(flinemapresp.functions.expect("Empty functions"));
                }
            }
        }   
    }
    if flines.is_empty() {
        log::error!("[extract_function_lines] No functions extracted");
        return None;
    }
    let parsed_flines = process_flinemap_response(&flines, lines.len());
    return Some(parsed_flines);
}

fn clean_and_deserialize(json_str: &str) -> Option<LlmFunctionLineMapResponse> {
    let mut cleaned_str = json_str.to_string();
    while !cleaned_str.is_empty() {
        match serde_json::from_str(&cleaned_str) {
            Ok(parsed) => return Some(parsed),
            Err(e) if e.to_string().contains("trailing characters") => {
                cleaned_str.pop(); // Remove the last character and try again
            }
            Err(e) => return None,
        }
    }
    None
}

fn extract_json_from_llm_response(llm_response: &str) -> String {
    let start_delim = "```json"; 
    let end_delim = "```";
    // Find the starting index of the JSON part
    let start_index = llm_response.find(start_delim).expect("find operation failed for ```json");
    // Find the ending index of the JSON part
    let end_index = llm_response[start_index + start_delim.len()..].find(end_delim).expect("find for ``` failed");

    // Extract the JSON part
    llm_response[start_index + start_delim.len()..start_index + start_delim.len() + end_index].trim().to_string()
}

fn process_flinemap_response(flines: &Vec<FunctionLineMap>, total_lines: usize) -> Vec<FunctionLineMap> {
    log::debug!("[process_flinemap_response] flines = {:?}", &flines);
    let mut resolved_flines: Vec<FunctionLineMap> = vec![];
    for flinemap in flines {
        if flinemap.name == "unknown" {
            if !resolved_flines.is_empty() {
                let fline_len = resolved_flines.len();
                resolved_flines[fline_len - 1].line_end = flinemap.line_end;
                continue;
            }
        }
        resolved_flines.push(flinemap.to_owned());
    }
    if let Some(last_flinemap) = resolved_flines.last() {
        if last_flinemap.line_end == -1 {
            let fline_len = resolved_flines.len();
            resolved_flines[fline_len - 1].line_end = total_lines as i32;
        }
    }
    log::debug!("[process_flinemap_response] resolved_flines = {:?}", &resolved_flines);
    return resolved_flines;
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmCalledFunctionResponse {
    functions: Option<Vec<CalledFunction>>
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
    let system_prompt_opt = read_file("/app/prompts/prompt_function_calls");
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
            // optional - paginate
            let mut unparsed_res = llm_response;
            // parse response to FunctionLineMap
            if unparsed_res.contains("```json") {
                unparsed_res = extract_json_from_llm_response(&unparsed_res);
            }
            let called_functions_res = serde_json::from_str(&unparsed_res);
            if called_functions_res.is_err() {
                let e = called_functions_res.expect_err("Empty error in called_functions_res");
                log::error!(
                    "[extract_function_calls] Unable to deserialize called_functions: {:?}", e);
                return None;
            }
            let called_func_response: LlmCalledFunctionResponse = called_functions_res.expect("Uncaught error in called_functions_res"); 
            return called_func_response.functions;
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
struct LlmCalledFunctionPathResponse {
    functions: Option<Vec<CalledFunctionPath>>
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct CalledFunctionPath {
    pub import_path: String,
    pub function_name: String,
    import_line: u32
}

pub async fn extract_function_import_path(called_funcs: &Vec<CalledFunction>, numbered_content: &str, file_name: &str) -> Option<Vec<CalledFunctionPath>> {
    let system_prompt_opt = read_file("/app/prompts/prompt_function_call_path");
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
        let first_occurence_line_opt = find_first_occurence(&numbered_lines, &called_func.name, called_func.line);
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
            let mut unparsed_res = llm_response;
            // parse response to FunctionLineMap
            if unparsed_res.contains("```json") {
                unparsed_res = extract_json_from_llm_response(&unparsed_res);
            }
            let called_functions_res = serde_json::from_str(&unparsed_res);
            if called_functions_res.is_err() {
                let e = called_functions_res.expect_err("Empty error in called_functions_res");
                log::error!(
                    "[extract_function_calls] Unable to deserialize called_functions: {:?}", e);
                return None;
            }
            let called_func_paths_res: LlmCalledFunctionPathResponse = called_functions_res.expect("Uncaught error in called_functions_res");
            return called_func_paths_res.functions;
        }
    }
    // optional - paginate
}

fn find_first_occurence(lines: &Vec<&str>, func_name: &str, hunk_line: usize) -> Option<String> {
    for (idx, line) in lines.iter().enumerate() {
        if idx+1 != hunk_line && line.contains(func_name) {
            return Some(line.to_owned().to_owned());
        }
    }
    return None;
}