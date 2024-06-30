use std::{collections::HashMap, process::Command, str};

use crate::utils::{gitops::StatItem, review::Review};

pub fn get_changed_files(small_files: &Vec<StatItem>, review: &Review) -> (HashMap<String, Vec<(usize, usize)>>, HashMap<String, Vec<(usize, usize)>>) {
    // Replace this with actual logic to get changed files in the PR
    let mut add_hunks_map = HashMap::<String, Vec<(usize, usize)>>::new();
    let mut del_hunks_map = HashMap::<String, Vec<(usize, usize)>>::new();
    let prev_commit = review.base_head_commit();
    let curr_commit = review.pr_head_commit();
    let clone_dir = review.clone_dir();

    for item in small_files {
        let filepath = item.filepath.as_str();
        let commit_range = format!("{}...{}", prev_commit, curr_commit);
        log::debug!("[extract_hunks] | clone_dir = {:?}, filepath = {:?}", clone_dir, filepath);
        let output_res = Command::new("git")
            .arg("diff")
            .arg("--unified=0")
            .arg(&commit_range)
            .arg(&filepath)
            .current_dir(clone_dir)
            .output();
        if output_res.is_err() {
            let commanderr = output_res.expect_err("No error in output_res");
            log::error!("[extract_hunks] git diff command failed to start : {:?}", commanderr);
            continue;
        }
        let result = output_res.expect("Uncaught error in output_res");
        let diff = result.stdout;
        let diffstr_res = str::from_utf8(&diff);
        if diffstr_res.is_err() {
            let e = diffstr_res.expect_err("No error in diffstr_res");
            log::error!("[extract_hunks] Unable to deserialize diff: {:?}", e);
            continue;
        }
        let diffstr = diffstr_res.expect("Uncaught error in diffstr_res");
        log::debug!("[extract_hunks] diffstr = {}", &diffstr);

        let mut add_hunks = Vec::new();
        let mut del_hunks = Vec::new();

        for line in diffstr.lines() {
            if line.starts_with("@@") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() > 2 {
                    let del_hunk = parts[1];
                    let add_hunk = parts[2];

                    if del_hunk.starts_with('-') {
                        if let Some((start, len)) = parse_hunk_range(del_hunk) {
                            let end = start + len - 1;
                            del_hunks.push((start, end));
                        }
                    }

                    if add_hunk.starts_with('+') {
                        if let Some((start, len)) = parse_hunk_range(add_hunk) {
                            let end = start + len - 1;
                            add_hunks.push((start, end));
                        }
                    }
                }
            }
        }

        if !add_hunks.is_empty() {
            add_hunks_map.insert(filepath.to_string(), add_hunks);
        }
        if !del_hunks.is_empty() {
            del_hunks_map.insert(filepath.to_string(), del_hunks);
        }
    }
    (add_hunks_map, del_hunks_map)
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
            return Some((start, len));
        }
    }
    None
}