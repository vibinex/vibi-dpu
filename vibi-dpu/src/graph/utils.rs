use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    log::debug!("[call_llm_api] final response = {}", &final_response);
    let final_response_trimmed = final_response.trim();
    if final_response_trimmed.starts_with("{") && !final_response_trimmed.ends_with("}") {
        final_response.push_str("}");
    }
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

pub fn all_code_files(dir: &str, diff_files: &Vec<StatItem>) -> Option<Vec<PathBuf>> {
    let mut code_files = Vec::<PathBuf>::new();
    let all_diff_langs = detect_langs_diff(diff_files);
    if all_diff_langs.is_empty() {
        log::error!("[all_code_files] No known language files detected in diff");
        return None;
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_owned();
        log::debug!("[all_code_files] path = {:?}", path);
        let ext = path.extension().and_then(|ext| ext.to_str());
        log::debug!("[all_code_files] extension = {:?}", &ext);
        if let Some(file_lang) = detect_language(&path.to_string_lossy()) {
            if all_diff_langs.contains(&file_lang) {
                match path.canonicalize() {
                    Ok(abs_path) => code_files.push(abs_path),
                    Err(e) => log::error!("Failed to get absolute path for {:?}: {:?}", path, e),
                }
            }
        }
    }
    if code_files.is_empty() {
        return None;
    }
    return Some(code_files);
}

fn detect_langs_diff(diff_files: &Vec<StatItem>) -> HashSet<String> {
    let mut all_diff_langs: HashSet<String> = HashSet::new();
    for diff_file in diff_files {
        if let Some(diff_lang) = detect_language(&diff_file.filepath) {
            all_diff_langs.insert(diff_lang);
        }
    }
    return all_diff_langs;
}

pub fn match_imported_filename_to_path(paths: &Vec<PathBuf>, filename: &str) -> Option<PathBuf> {
    let relative_path = Path::new(filename);
    // Find the first path that matches the filename or relative path
    for path in paths {
        if path.ends_with(relative_path) {
            return Some(path.clone());  // Return the first matching path
        }
    }
    // Return an empty PathBuf or handle the case where no match is found
    None
}

pub fn source_diff_files(diff_files: &Vec<StatItem>) -> Option<Vec<StatItem>> {
    let mut code_files = Vec::<StatItem>::new();
    for stat_item in diff_files {
        let filepath_str = &stat_item.filepath;
        let filepath = Path::new(filepath_str);  
        if let Some(lang) = detect_language(&filepath_str) {
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

pub fn absolute_to_relative_path(abs_path: &str, review: &Review) -> Option<String> {
    let base_path = review.clone_dir();
    let full_path = PathBuf::from(abs_path);
    let rel_path_res = full_path.strip_prefix(base_path);
    log::debug!("[absolute_to_relative_path] rel_path = {:#?}", &rel_path_res);
    log::debug!("[absolute_to_relative_path] full_path = {:?}, base_path = {:?}", &full_path, base_path);
    if let Err(e) = rel_path_res {
        log::error!("[absolute_to_relative_path] Error in removing prefix: {:?}", e);
        return None;
    }
    let rel_path = rel_path_res.expect("Uncaught error in rel_path_res");
    return Some(rel_path.to_str().expect("Unable to deserialze rel_path").to_string());
}

pub fn strip_json_prefix(json_str: &str) -> Option<String> {
    let mut extracted_json = json_str.to_string();
    if let Some(start) = json_str.find("```json") {
        // Find the end of "```" after the "```json"
        if let Some(end) = json_str[start + 7..].find("```") {
            // Return the substring between "```json" and "```"
            extracted_json = json_str[start + 7..start + 7 + end].to_string();
        }
    } else if let Some(start) = json_str.find("```") {
        if let Some(end) = json_str[start + 7..].find("```") {
            // Return the substring between "```" and "```"
            extracted_json = json_str[start + 3..start + 3 + end].to_string();
        }
    }
    if extracted_json.starts_with('[') && extracted_json.ends_with(']') {
        // Slice the string to remove the first and last characters
        extracted_json = extracted_json[1..extracted_json.len() - 1].to_string();
        
    }
    extracted_json = fix_unbalanced_json(&extracted_json);
    return Some(extracted_json);
}


fn fix_unbalanced_json(json_str: &str) -> String {
    let mut fixed_json = json_str.to_string();
    
    // Count the number of opening and closing curly braces
    let open_brace_count = fixed_json.matches('{').count();
    let close_brace_count = fixed_json.matches('}').count();
    
    // Add missing closing braces if needed
    if open_brace_count > close_brace_count {
        fixed_json.push('}');
    }
    
    return fixed_json;
}

// Generate a map of file extensions to languages or frameworks
fn get_extension_map() -> HashMap<&'static str, &'static str> {
    let mut extension_map = HashMap::new();

    // Common programming languages
    extension_map.insert("rs", "Rust");
    extension_map.insert("py", "Python");
    extension_map.insert("js", "JavaScript");
    extension_map.insert("ts", "TypeScript");
    extension_map.insert("java", "Java");
    extension_map.insert("rb", "Ruby");
    extension_map.insert("go", "Go");
    extension_map.insert("cpp", "C++");
    extension_map.insert("cs", "C#");
    extension_map.insert("c", "C");
    extension_map.insert("php", "PHP");
    extension_map.insert("swift", "Swift");
    extension_map.insert("kt", "Kotlin");
    extension_map.insert("m", "Objective-C");
    extension_map.insert("pl", "Perl");
    extension_map.insert("r", "R");
    extension_map.insert("scala", "Scala");
    extension_map.insert("dart", "Dart");
    extension_map.insert("lua", "Lua");
    extension_map.insert("hs", "Haskell");
    extension_map.insert("erl", "Erlang");
    extension_map.insert("ml", "OCaml");
    extension_map.insert("groovy", "Groovy");
    extension_map.insert("sql", "SQL");
    extension_map.insert("v", "V");
    extension_map.insert("nim", "Nim");
    extension_map.insert("elm", "Elm");
    extension_map.insert("jl", "Julia");
    extension_map.insert("cr", "Crystal");
    extension_map.insert("ex", "Elixir");
    extension_map.insert("fs", "F#");
    extension_map.insert("clj", "Clojure");
    extension_map.insert("coffee", "CoffeeScript");
    extension_map.insert("hx", "Haxe");
    extension_map.insert("lisp", "Lisp");
    extension_map.insert("scss", "Sass");
    extension_map.insert("ps1", "PowerShell");
    extension_map.insert("vb", "Visual Basic");
    extension_map.insert("bat", "Batch Script");
    extension_map.insert("matlab", "MATLAB");
    extension_map.insert("vbs", "VBScript");
    extension_map.insert("as", "ActionScript");
    extension_map.insert("rkt", "Racket");
    extension_map.insert("cls", "Apex");
    extension_map.insert("sass", "Sass");
    extension_map.insert("less", "Less");

    // Web and markup languages
    extension_map.insert("html", "HTML");
    extension_map.insert("css", "CSS");
    extension_map.insert("xml", "XML");
    extension_map.insert("md", "Markdown");
    extension_map.insert("adoc", "AsciiDoc");
    extension_map.insert("rst", "reStructuredText");

    // Frameworks and template languages
    extension_map.insert("jsx", "React JSX");
    extension_map.insert("tsx", "React TypeScript TSX");
    extension_map.insert("vue", "Vue.js");
    extension_map.insert("erb", "Ruby on Rails Embedded Ruby");
    extension_map.insert("ejs", "Express.js Embedded JavaScript");

    // Config and data formats
    // extension_map.insert("json", "JSON");
    // extension_map.insert("yaml", "YAML");
    // extension_map.insert("toml", "TOML");
    // extension_map.insert("ini", "INI Config");
    // extension_map.insert("config", "Configuration File");

    extension_map
}

// Detect the programming language or framework based on the file extension
pub fn detect_language(file_path: &str) -> Option<String> {
    let extension_map = get_extension_map();
    let path = Path::new(file_path);

    // Extract the file extension and match it with the map
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase()) // Normalize to lowercase
        .and_then(|ext| extension_map.get(ext.as_str()).map(|&lang| lang.to_string()))
}