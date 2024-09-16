use std::{collections::HashMap, path::{Path, PathBuf}, slice::Chunks};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    let url = "http://34.100.208.132/api/generate";
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

    // Process each chunk
    for chunk in chunks {
        // Attempt to fix incomplete chunks
        let fixed_chunk = if !chunk.starts_with("{") {
            format!("{{{}", chunk)
        } else if !chunk.ends_with("}") {
            format!("{}{}", chunk, "}")
        } else {
            chunk.to_string()
        };
        let parsed_chunk_res = serde_json::from_str::<Value>(&fixed_chunk);
        if parsed_chunk_res.is_err() {
            let e = parsed_chunk_res.expect_err("Empty error in parsed_chunk_res");
            log::error!("[call_llm_api] Unable to deserialize {}: {:?}", chunk, e);
            continue;
        }
        let parsed_chunk = parsed_chunk_res.expect("Uncaught error in parsed_chunk_res");
        if let Some(parsed_response) =  parsed_chunk.get("response").and_then(|v| v.as_str()){
            final_response.push_str(parsed_response);    
        }
        if let Some(done_field) = parsed_chunk.get("done").and_then(|v| v.as_bool()) {
            if done_field {
                break;
            }
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
            match path.canonicalize() {
                Ok(abs_path) => code_files.push(abs_path),
                Err(e) => log::error!("Failed to get absolute path for {:?}: {:?}", path, e),
            }
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
        .map(|(index, line)| format!("{} {}", index, line))
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