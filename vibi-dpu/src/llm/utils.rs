use std::path::Path;

use futures_util::StreamExt;
use std::fs;

use crate::utils::reqwest_client::get_client;

pub async fn call_llm_api(prompt: String) -> Option<String> {
    let client = get_client();
    let url = "https://your-llm-api-endpoint.com";
    let token = "your_api_token";

    let response_res = client.post(url)
        .bearer_auth(token)
        .json(&serde_json::json!({"prompt": prompt}))
        .send()
        .await;
    if response_res.is_err() {
        let err = response_res.expect_err("No error in response_res");
        log::error!("[call_llm_api] Error in calling api: {:?}", err);
        return None;
    }
    let response = response_res.expect("Uncaught error in response_res");
    let mut final_response = String::new();

    let mut stream = response.bytes_stream();
    while let Some(item_res) = stream.next().await {
        if item_res.is_err() {
            let err = item_res.expect_err("Empty error in item_res");
            log::error!("[call_llm_api] Error in parsing stream {:?}", err);
            return None;
        }
        let item = item_res.expect("Empty item_res");
        let chunk = item;

        final_response.push_str(&String::from_utf8_lossy(&chunk));
    }

    Some(final_response)
}

pub fn get_changed_files() -> Option<Vec<String>> {
    // Replace this with actual logic to get changed files in the PR
    let output_res = std::process::Command::new("git")
        .args(&["diff", "--name-only", "HEAD^", "HEAD"])
        .output();
    if output_res.is_err() {
        let err = output_res.expect_err("Empty error in output_res");
        log::error!("[get_changed_files] Error in getting diff files: {:?}", err);
        return None;
    }
    let output = output_res.expect("Uncaught error in output_res");
    let files = String::from_utf8_lossy(&output.stdout);
    Some(files.lines().map(String::from).collect())
}

pub fn read_files(files: Vec<String>) -> Option<String> {
    let mut content = String::new();

    for file in files {
        let path = Path::new(&file);
        let content_res = fs::read_to_string(path);
        if path.exists() {
            if content_res.is_err() {
                return None;
            }
            content = content_res.expect("Empty content_res");
            content.push('\n');
        }
    }

    Some(content)
}