use std::{collections::HashMap, path::{Path, PathBuf}, slice::Chunks};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strsim::jaro_winkler;
use walkdir::WalkDir;
use std::fs;
use rand::Rng;


use crate::utils::{gitops::StatItem, reqwest_client::get_client, review::Review};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool
}

pub async fn call_llm_api(prompt: String) -> Option<String> {
    let client = get_client();
    let url = "http://host.docker.internal:11434/api/generate";
    log::debug!("[call_llm_api] Prompt = {:?}", &prompt);
    let response_res = client.post(url)
        .json(&json!({"model": "phind-codellama", "prompt": prompt}))
        .send()
        .await;

    if let Err(err) = response_res {
        log::error!("[call_llm_api] Error in calling api: {:?}", err);
        return None;
    }

    let response = response_res.unwrap();
    let mut final_response = String::new();
    let resp_text_res = response.text().await;
    if resp_text_res.is_err() {
        let e = resp_text_res.expect_err("Empty error in resp_text_res");
        log::error!("[call_llm_api] Error while deserializing response to text: {:?}", e);
        return None;
    }
    let resp_text = resp_text_res.expect("Uncaught error in resp_text");
    // Split the string by the sequence "}\n{"
    let split_seq = "}\n{";
    let mut chunks = Vec::new();
    let mut start = 0;
    while let Some(pos) = &resp_text[start..].find(split_seq) {
        let end = start + pos + 1;
        chunks.push(&resp_text[start..end]);
        start = end + 1;
    }

    for chunk in chunks {
        let parsed_chunk_res = serde_json::from_str(&chunk);
        if parsed_chunk_res.is_err() {
            let e = parsed_chunk_res.expect_err("Empty error in parsed_chunk_res");
            log::error!("[call_llm_api] Unable to deserialize {}: {:?}", chunk, e);
            continue;
        }
        let parsed_chunk: LlmResponse = parsed_chunk_res.expect("Uncaught error in parsed_chunk_res");
        final_response.push_str(&parsed_chunk.response);
        if parsed_chunk.done {
            break;
        }
    }
    log::debug!("[call_llm_api] final_response = {:?}", &final_response);
    Some(final_response)
}

pub fn read_file(file: &str) -> Option<String> {
    log::debug!("[read_file] file name = {}", &file);
    let path = Path::new(file);
    let content_res = fs::read_to_string(path);
    if !path.exists() {
        log::error!("[read_file] File does not exist: {:?}", &path);
        return None;
    }
    if content_res.is_err() {
        let err = content_res.expect_err("Empty error in content_res");
        log::error!("[read_file] Error in reading content: {:?}", err);
        return None;
    }
    let content = content_res.expect("Empty content_res");
    Some(content)
}

pub fn get_specific_lines(line_numbers: Vec<(usize, usize)>, numbered_content: &str) -> String {
    // Split the input content into lines and collect into a vector
    let lines: Vec<&str> = numbered_content.lines().collect();
    let mut result = String::new();
    // Iterate over each line number we are interested in
    for (mut start, mut end) in line_numbers {
        if start > end {
            let xchng = start;
            start = end;
            end = xchng;
        }
        for line_number in start..=end {
            // Check if the line_number is within the bounds of the vector
            if line_number < lines.len() {
                result.push_str(lines[line_number]);
                result.push('\n');
            }
        }
    }
    return result;
}

pub fn generate_random_string(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let random_string: String = (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    random_string
}

pub fn all_code_files(dir: &str) -> Option<Vec<PathBuf>> {
    let mut code_files = Vec::<PathBuf>::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_owned();
        log::debug!("[generate_function_map] path = {:?}", path);
        let ext = path.extension().and_then(|ext| ext.to_str());
        log::debug!("[generate_function_map] extension = {:?}", &ext);
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            code_files.push(path);
        }
    }
    if code_files.is_empty() {
        return None;
    }
    return Some(code_files);
}

pub fn source_diff_files(diff_files: &Vec<StatItem>) -> Option<Vec<StatItem>> {
    let mut code_files = Vec::<StatItem>::new();
    for stat_item in diff_files {
        let filepath_str = &stat_item.filepath;
        let filepath = Path::new(filepath_str);   
        if filepath.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            code_files.push(stat_item.clone());
        }
    }
    if code_files.is_empty() {
        return None;
    }
    return Some(code_files);
}

pub fn numbered_content(file_contents: String) -> Vec<String> {
    let lines = file_contents
        .lines()
        .enumerate()
        .map(|(index, line)| format!("{} {}", index+1, line))
        .collect::<Vec<String>>();
    return lines;
}

pub fn match_overlap(str1: &str, str2: &str, similarity_threshold: f64) -> bool {
    let similarity = jaro_winkler(str1, str2);
    if similarity >= similarity_threshold {
        return true;
    }
    return false;
}