use std::{collections::HashMap, path::{Path, PathBuf}, process::Command, str::{self, FromStr}};

use crate::utils::{gitops::StatItem, review::Review};

#[derive(Debug, Default, Clone)]
pub struct HunkDiffLines {
    start_line: usize,
    end_line: usize,
    function_line: Option<String>,
    line_number: Option<usize>,
    function_name: Option<String>
}

impl HunkDiffLines {
    pub fn start_line(&self) -> &usize {
        &self.start_line
    }

    pub fn end_line(&self) -> &usize {
        &self.end_line
    }

    pub fn function_line(&self) -> &Option<String> {
        &self.function_line
    }

    pub fn line_number(&self) -> &Option<usize> {
        &self.line_number
    }

    pub fn set_line_number(&mut self, line_number: Option<usize>) {
        self.line_number = line_number;
    }

    pub fn set_function_name(&mut self, function_name: String) {
        self.function_name = Some(function_name);
    }
}

#[derive(Debug, Default, Clone)]
pub struct FileHunks {
    deleted_hunks: Vec<HunkDiffLines>,
    added_hunks: Vec<HunkDiffLines>
}

impl FileHunks {
    pub fn deleted_hunks(&self) -> &Vec<HunkDiffLines> {
        &self.deleted_hunks
    }

    pub fn added_hunks(&self) -> &Vec<HunkDiffLines> {
        &self.added_hunks
    }

    // Mutable references to allow modification of the hunks
    pub fn deleted_hunks_mut(&mut self) -> &mut Vec<HunkDiffLines> {
        &mut self.deleted_hunks
    }

    pub fn added_hunks_mut(&mut self) -> &mut Vec<HunkDiffLines> {
        &mut self.added_hunks
    }

    pub fn is_func_in_hunks(&self, function_name: &str) -> &Option<usize> {
        for hunk_lines in self.added_hunks() {
            if let Some(func_raw) = hunk_lines.function_line() {
                if func_raw.contains(function_name) {
                    return hunk_lines.line_number();
                }
            }
        }
        for hunk_lines in self.deleted_hunks() {
            if let Some(func_raw) = hunk_lines.function_line() {
                if func_raw.contains(function_name) {
                    return hunk_lines.line_number();
                }
            }
        }
        return &None;
    }
}

#[derive(Debug, Default, Clone)]
pub struct HunkDiffMap {
    file_line_map: HashMap<String, FileHunks>,
}

impl HunkDiffMap {
    pub fn file_line_map(&self) -> &HashMap<String, FileHunks> {
        &self.file_line_map
    }

    pub fn file_line_map_mut(&mut self) -> &mut HashMap<String, FileHunks> {
        &mut self.file_line_map
    }

    pub fn all_files(&self) -> Vec<&String> {
        self.file_line_map.keys().collect::<Vec<&String>>()
    }

    pub fn all_files_pathbuf(&self, clone_dir: &str) -> Vec<PathBuf> {
        let base_path = Path::new(clone_dir);
        self.file_line_map.keys()
        .filter_map(|s| {
            let relative_path = Path::new(s);
            let abs_filepath = base_path.join(relative_path);
            Some(abs_filepath)
        })
        .collect::<Vec<PathBuf>>()
    }

    pub fn file_hunks(&self, filename: &str) -> Option<&FileHunks> {
        self.file_line_map.get(filename)
    }
}

pub fn get_changed_hunk_lines(diff_files: &Vec<StatItem>, review: &Review) -> HunkDiffMap {
    let mut file_hunk_map = HunkDiffMap { file_line_map: HashMap::new() };
    let prev_commit = review.base_head_commit();
    let curr_commit = review.pr_head_commit();
    let clone_dir = review.clone_dir();

    for item in diff_files {
        let filepath = item.filepath.as_str();
        let commit_range = format!("{}...{}", prev_commit, curr_commit);
        log::debug!("[get_changed_hunk_lines] | clone_dir = {:?}, filepath = {:?}", clone_dir, filepath);

        let output_res = Command::new("git")
            .arg("diff")
            .arg("--unified=0")
            .arg("--ignore-space-change")
            .arg(&commit_range)
            .arg(&filepath)
            .current_dir(clone_dir)
            .output();

        if output_res.is_err() {
            let commanderr = output_res.expect_err("No error in output_res");
            log::error!("[get_changed_hunk_lines] git diff command failed to start : {:?}", commanderr);
            continue;
        }

        let result = output_res.expect("Uncaught error in output_res");
        let diff = result.stdout;
        let diffstr_res = std::str::from_utf8(&diff);

        if diffstr_res.is_err() {
            let e = diffstr_res.expect_err("No error in diffstr_res");
            log::error!("[get_changed_hunk_lines] Unable to deserialize diff: {:?}", e);
            continue;
        }

        let diffstr = diffstr_res.expect("Uncaught error in diffstr_res");
        log::debug!("[get_changed_hunk_lines] diffstr = {}", &diffstr);

        let mut current_add_start = 0;
        let mut current_del_start = 0;
        let mut current_add_end = 0;
        let mut current_del_end = 0;
        let mut in_add_hunk = false;
        let mut in_del_hunk = false;
        let mut file_hunks = FileHunks {
            deleted_hunks: Vec::new(),
            added_hunks: Vec::new(),
        };

        // Variable to store the function line
        let mut function_line: Option<String> = None;

        for line in diffstr.lines() {
            if line.starts_with("@@") {
                // Save previous hunks if any
                if in_add_hunk {
                    file_hunks.added_hunks.push(HunkDiffLines {
                        start_line: current_add_start,
                        end_line: current_add_end,
                        function_line: function_line.clone(), // Use the function line stored
                        line_number: None,
                        function_name: None
                    });
                }
                if in_del_hunk {
                    file_hunks.deleted_hunks.push(HunkDiffLines {
                        start_line: current_del_start,
                        end_line: current_del_end,
                        function_line: function_line.clone(), // Use the function line stored
                        line_number: None,
                        function_name: None
                    });
                }

                // Reset states for next hunk
                in_add_hunk = false;
                in_del_hunk = false;

                // Extract the function name or any string after the last @@
                if let Some(pos) = line.rfind("@@ ") {
                    function_line = Some(line[(pos+3)..].to_string());
                } else {
                    function_line = None; // Reset if no valid function line found
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Determine the start and end lines for the hunks
                let del_hunk = parts.get(1);
                let add_hunk = parts.get(2);

                if let Some(del_hunk) = del_hunk {
                    if del_hunk.starts_with('-') {
                        if let Some((start, len)) = parse_hunk_range(del_hunk) {
                            current_del_start = start;
                            current_del_end = start + len - 1;
                            in_del_hunk = true;
                        }
                    }
                }

                if let Some(add_hunk) = add_hunk {
                    if add_hunk.starts_with('+') {
                        if let Some((start, len)) = parse_hunk_range(add_hunk) {
                            current_add_start = start;
                            current_add_end = start + len - 1;
                            in_add_hunk = true;
                        }
                    }
                }
            }
        }

        // Push the last hunks if still in any hunk
        if in_add_hunk {
            file_hunks.added_hunks.push(HunkDiffLines {
                start_line: current_add_start,
                end_line: current_add_end,
                function_line: function_line.clone(), // Use the function line stored
                line_number: None,
                function_name: None
            });
        }
        if in_del_hunk {
            file_hunks.deleted_hunks.push(HunkDiffLines {
                start_line: current_del_start,
                end_line: current_del_end,
                function_line: function_line.clone(), // Use the function line stored
                line_number: None,
                function_name: None
            });
        }

        let abs_filepath = Path::new(review.clone_dir());
        let abs_file_pathbuf = abs_filepath.join(Path::new(filepath));
        file_hunk_map.file_line_map.insert(
            abs_file_pathbuf.to_str().expect("Unable to deserialize pathbuf").to_string(),
            file_hunks,
        );
    }

    return file_hunk_map;
}


fn parse_hunk_range(hunk: &str) -> Option<(usize, usize)> {
    let hunk = hunk.trim_start_matches(&['-', '+'][..]);
    let parts: Vec<&str> = hunk.split(',').collect();
    if parts.len() == 1 {
        if let Ok(start) = parts[0].parse::<usize>() {
            return Some((start, 1));
        }
    } else if parts.len() == 2 {
        if let (Ok(start), Ok(len)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
            if len == 0 {
                return None;
            }
            return Some((start, len));
        }
    }
    None
}
