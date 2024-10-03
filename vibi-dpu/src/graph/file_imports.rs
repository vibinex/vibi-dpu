use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{graph::utils::numbered_content, utils::review::Review};

use super::utils::{all_code_files, call_llm_api, read_file};

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmImportLineInput {
    language: String,
    file_path: String,
    chunk: String
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmImportLineRequest {
    input: LlmImportLineInput
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FileImportLines {
    lines: Vec<i32>
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmImportPathInput {
    language: String,
    file_path: String,
    import_lines: String
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
struct LlmImportPathRequest {
    input: LlmImportPathInput
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct  ImportPath {
    import_line: usize,
    import_path: String,
    imported: String
}

impl PartialEq for ImportPath {
    fn eq(&self, other: &Self) -> bool {
        self.import_line == other.import_line && self.import_path == other.import_path && self.imported == other.imported
    } 
}

impl ImportPath {
    
    pub fn new(import_line: usize, import_path: String, imported: String) -> Self {
        Self { import_line, import_path, imported }
    }
    pub fn import_path(&self) -> &String {
        &self.import_path
    }

    pub fn imported(&self) -> &String {
        &self.imported
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct ImportPaths {
    imports: Vec<ImportPath>,
}

impl ImportPaths {
    pub fn imports(&self) -> &Vec<ImportPath> {
        &self.imports
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct ChunkImportInfo {
    import_lines: FileImportLines,
    import_paths: Vec<ImportPath>
}

impl ChunkImportInfo {
    pub fn import_paths(&self) -> &Vec<ImportPath> {
        &self.import_paths
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FileImportInfo {
    import_chunk_info: Vec<ChunkImportInfo>,
    filepath: String
}

impl FileImportInfo {
    pub fn all_import_paths(&self) -> Vec<ImportPath> {
        let all_paths: Vec<ImportPath> = self.import_chunk_info
            .iter()
            .flat_map(|chunk| chunk.import_paths())
            .cloned()
            .collect();
        return all_paths;
    }

    pub fn filepath(&self) -> &String {
        &self.filepath
    }
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct FilesImportInfo {
    file_import_map: HashMap<String, FileImportInfo>
}

impl FilesImportInfo {
    pub fn files(&self) -> Vec<&String> {
        self.file_import_map.keys().collect()
    }
    
    pub fn is_import_in_file(&self, filename: &str, import_path: &ImportPath) -> bool {
        self.file_import_map[filename].all_import_paths().contains(import_path)
    }

    pub fn file_import_info(&self, filename: &str) -> Option<&FileImportInfo> {
        self.file_import_map.get(filename)
    }

    pub fn file_import_map(&self) -> &HashMap<String, FileImportInfo> {
        &self.file_import_map
    }
}

pub async fn get_import_lines(file_paths: &Vec<PathBuf>) -> Option<FilesImportInfo> {
    let mut all_import_info = HashMap::<String, FileImportInfo>::new();
    let system_prompt_opt = read_file("/app/prompts/prompt_import_lines");
    if system_prompt_opt.is_none() {
        log::error!("[get_import_lines] Unable to read prompt_import_lines");
        return None;
    }
    let system_prompt_lines = system_prompt_opt.expect("Empty system_prompt");
    let system_prompt_path_opt = read_file("/app/prompts/prompt_import_path");
    if system_prompt_path_opt.is_none() {
        log::error!("[get_import_lines] Unable to read prompt_import_path");
        return None;
    }
    let system_prompt_path = system_prompt_path_opt.expect("Empty system_prompt");
    for path in file_paths {
        log::debug!("[get_import_lines] path = {:?}", path);
        let file_contents_res = std::fs::read_to_string(path.clone());
        if file_contents_res.is_err() {
            let e = file_contents_res.expect_err("Empty error in file_content_res");
            log::error!("[get_import_lines] Unable to read file: {:?}, error: {:?}", path, e);
            continue;
        }
        let file_contents = file_contents_res.expect("Uncaught error in file_content_res");
        let numbered_content = numbered_content(file_contents);
        let chunks = numbered_content.chunks(20);
        let path_str = path.to_str().expect("Empty path");
        let mut chunks_import_vec = Vec::<ChunkImportInfo>::new();
        for chunk in chunks {
            let chunk_str = chunk.join("\n");
            let import_lines_opt = get_import_lines_chunk(
                &system_prompt_lines, &chunk_str,
                path_str).await;
            if import_lines_opt.is_none() {
                log::error!("[get_import_lines] Skipping chunk, unable to get import lines");
                continue;
            }
            let import_lines_chunk = import_lines_opt.expect("Empty func_boundary_opt");
            if let Some(import_paths) = get_import_path_file(&numbered_content,
                import_lines_chunk.clone(), &system_prompt_path, path_str).await {
                    let chunk_import_info = ChunkImportInfo { import_lines: import_lines_chunk, import_paths };
                    chunks_import_vec.push(chunk_import_info);
            }
        }
        let import_info = FileImportInfo {
            import_chunk_info: chunks_import_vec, filepath: path_str.to_string() };
        all_import_info.insert(path_str.to_string(), import_info);
    }
    if all_import_info.is_empty() {
        return None;
    }
    return Some(FilesImportInfo { file_import_map: all_import_info });
}

async fn get_import_lines_chunk(system_prompt_lines: &str, chunk_str: &str, file_path: &str) -> Option<FileImportLines> {
    let llm_req = LlmImportLineRequest { input: 
        LlmImportLineInput {
            language: "typescript".to_string(),
            file_path: file_path.to_string(),
            chunk: chunk_str.to_string() } };
    let llm_req_res = serde_json::to_string(&llm_req);
    if llm_req_res.is_err() {
        log::error!("[get_import_lines_chunk] Error in serializing llm req: {}", llm_req_res.expect_err("Empty error in llm_req_res"));
        return None;
    }
    let llm_req_prompt = llm_req_res.expect("Uncaught error in llm_req_res"); 
    let prompt = format!("{}\n\n### User Message\nInput -\n{}\nOutput -",
        system_prompt_lines, llm_req_prompt);
    match call_llm_api(prompt).await {
        None => {
            log::error!("[get_import_lines_chunk] Failed to call LLM API");
            return None;
        }
        Some(llm_response) => {
            let import_res = serde_json::from_str(&llm_response);
            if import_res.is_err() {
                log::error!(
                    "[get_import_lines_chunk] funcdefs error: {}",
                    import_res.expect_err("Empty error in funcdefs_res"));
                    return None;
            }
            let import_lines_file: FileImportLines = import_res.expect("Uncaught error in funcdefs_res");
            return Some(import_lines_file);
        }
    }
}

async fn get_import_path_file(numbered_content: &Vec<String>, import_line: FileImportLines, system_prompt: &str, file_path: &str) -> Option<Vec<ImportPath>> {
    let mut import_paths = Vec::<ImportPaths>::new();
    // get import lines from numbered lines
    let import_lines_str_opt = numbered_import_lines(numbered_content, import_line);
    if import_lines_str_opt.is_none() {
        log::error!("[get_import_path_file] Unable to get numbered import line");
        return None;
    }
    let import_lines_str_chunks = import_lines_str_opt.expect("Empty import_lines_str_opt");
    for import_lines_chunk in import_lines_str_chunks {
        let llm_req = LlmImportPathRequest{
            input: LlmImportPathInput {
                language: "typescript".to_string(),
                file_path: file_path.to_string(),
                import_lines: import_lines_chunk 
            }
        };
        let llm_req_res = serde_json::to_string(&llm_req);
        if llm_req_res.is_err() {
            log::error!("[get_import_path_file] Error in serializing llm req: {}", llm_req_res.expect_err("Empty error in llm_req_res"));
            return None;
        }
        let llm_req_prompt = llm_req_res.expect("Uncaught error in llm_req_res");
        let prompt = format!("{}\n\n### User Message\nInput -\n{}\nOutput -",
            system_prompt, llm_req_prompt);
        match call_llm_api(prompt).await {
            None => {
                log::error!("[get_import_path_file] Failed to call LLM API");
                return None;
            }
            Some(llm_response) => {
                let import_res = serde_json::from_str(&llm_response);
                if import_res.is_err() {
                    log::error!(
                        "[get_import_path_file] funcdefs error: {}",
                        import_res.expect_err("Empty error in funcdefs_res"));
                        continue;
                }
                let import_path: ImportPaths = import_res.expect("Uncaught error in funcdefs_res");
                import_paths.push(import_path);
            }
        }
    }
    if import_paths.is_empty() {
        return None;
    }
    let import_path_vec: Vec<ImportPath> = import_paths
        .iter()
        .flat_map(|ip| ip.imports.iter().cloned())
        .collect();
    return Some(import_path_vec);
}

fn numbered_import_lines(numbered_content: &Vec<String>, import_line: FileImportLines) -> Option<Vec<String>>{
    let mut chunks = Vec::new();
    let mut chunk = String::new();
    let mut line_count = 0;

    for line in import_line.lines {
        if line_count == 30 {
            chunks.push(chunk.clone());
            chunk = String::new();
            line_count = 0;
        }
        if line < (numbered_content.len() as i32) {
            chunk += &numbered_content[line as usize];
            line_count += 1;
        }
    }

    // Push the last chunk if it's not empty
    if !chunk.is_empty() {
        chunks.push(chunk);
    }

    if chunks.is_empty() {
        return None;
    }
    Some(chunks)
}