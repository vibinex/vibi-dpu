use std::path::Path;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;

use crate::utils::reqwest_client::get_client;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool
}

pub async fn call_llm_api(prompt: String) -> Option<String> {
    let client = get_client();
    let url = "http://35.244.9.107/api/generate";
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

    log::debug!("[call_llm_api] chunks = {:?}", &chunks);
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
    log::error!("[read_file] file name = {}", &file);
    let path = Path::new(file);
    let content_res = fs::read_to_string(path);
    if !path.exists() {
        log::error!("[read_file] Path does not exist: {:?}", &path);
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

pub fn parse_llm_response(llm_response: &str) -> Option<String> {
    return None;
}