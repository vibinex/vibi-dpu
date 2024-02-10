use std::collections::HashMap;
use std::process::Command;
use std::str;
use serde::Deserialize;
use serde::Serialize;
use sha256::digest;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::fs;
use std::io::ErrorKind;

use super::hunk::BlameItem;
use super::review::Review;
use super::lineitem::LineItem;
use crate::db::repo::save_repo_to_db;
use crate::utils::repo::Repository;

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct StatItem {
	filepath: String,
	additions: i32,
	deletions: i32,
}

pub fn commit_exists(commit: &str, directory: &str) -> bool {
	let output_res = Command::new("git")
		.arg("rev-list")
		.arg(commit)
		.current_dir(directory)
		.output();
	if output_res.is_err() {
		let e = output_res.expect_err("No error in output_res");
		log::error!("[commit_exists] Failed to start git rev-list: {:?}", e);
		return false;
	}
	let output = output_res.expect("Uncaught error in output_res");
	if !output.status.success() {
		log::error!("[commit_exists] git rev-list, exit code: {:?}",
			output.status.code());
		// for debugging
		match str::from_utf8(&output.stderr) {
			Ok(v) => log::error!("[commit_exists] git rev-list stderr = {:?}", v),
			Err(e) => {/* error handling */ log::error!("[commit_exists] git rev-list stderr error {}", e)}, 
		};
		return false;
	}
	log::debug!("[commit_exists] Execute git rev-list, exit code: {:?}", output.status.code());
	if output.status.code() == Some(128) {
		// for debugging
		match str::from_utf8(&output.stderr) {
			Ok(v) => log::error!("[commit_exists] git rev-list stderr = {:?}", v),
			Err(e) => {/* error handling */ log::error!("[commit_exists] git rev-list stderr error {}", e)}, 
		};
		return false;
	}
	// for debugging
	match str::from_utf8(&output.stderr) {
		Ok(v) => log::debug!("[commit_exists] git rev-list stderr = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[commit_exists] git rev-list stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => log::debug!("[commit_exists] git rev-list stdout = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[commit_exists] git rev-list stdout error {}", e)}, 
	};
	return true;
}

pub async fn git_pull(review: &Review, access_token: &str) {
	let directory = review.clone_dir();
	log::debug!("[git_pull] directory = {}", &directory);
    set_git_url(review.clone_url(), directory, &access_token, review.provider());
	let output_res = Command::new("git")
		.arg("pull")
		.current_dir(directory)
		.output();
	if output_res.is_err() {
		let e = output_res.expect_err("No error in output_res");
		log::error!("[git_pull] failed to execute git pull: {:?}", e);
		return;
	}
	let output = output_res.expect("Uncaught error in output_res");
	match str::from_utf8(&output.stderr) {
		Ok(v) => log::debug!("[git_pull] git pull stderr = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[git_pull] git pull stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => log::debug!("[git_pull] git pull stdout = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[git_pull] git pull stdout error {}", e)}, 
	};
}

fn set_git_url(git_url: &str, directory: &str, access_token: &str, repo_provider: &str) {
    let clone_url_opt = create_clone_url(git_url, access_token, repo_provider);
	if clone_url_opt.is_none(){
		return
	}
	let clone_url = clone_url_opt.expect("empty clone_url_opt");
    let output_res = Command::new("git")
		.arg("remote").arg("set-url").arg("origin")
		.arg(clone_url)
		.current_dir(directory)
		.output();
	if output_res.is_err() {
		let e = output_res.expect_err("No error in output_res");
		log::error!("[set_git_url] failed to execute set_git_url: {:?}", e);
		return;
	}
	let output = output_res.expect("Uncaught error in output_res");
	if !output.status.success() {
		log::error!("[set_git_url] set_git_url failed with exit code: {}", output.status);
		return;
	}
	match str::from_utf8(&output.stderr) {
		Ok(v) => log::debug!("[set_git_url] set_git_url stderr = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[set_git_url] stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => log::error!("[set_git_url] stdout = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[set_git_url] stdout error {}", e)}, 
	};
	log::debug!("[set_git_url] set_git_url output = {:?}, {:?}", &output.stdout, &output.stderr);
}

pub fn get_excluded_files(review: &Review) -> Option<(Vec<StatItem>, Vec<StatItem>)> {
	let prev_commit = review.base_head_commit();
	let next_commit = review.pr_head_commit();
	let clone_dir = review.clone_dir();
	log::debug!("[get_excluded_files] prev_commit = {}, next commit = {}, clone_dir = {}",
		prev_commit, next_commit, clone_dir);
	let commit_range = format!("{}...{}", prev_commit, next_commit);
	let git_res = Command::new("git")
		.args(&["diff", &commit_range, "--numstat"])
		.current_dir(clone_dir)
		.output();
	if git_res.is_err() {
		let commanderr = git_res.expect_err("No error in git command");
		log::error!("[get_excluded_files] git diff stat command failed to start : {:?}", commanderr);
		return None;
	}
	let resultstat = git_res.expect("Uncaught error in git_res");
	let stat = resultstat.stdout;
	// parse the output
	let stat_res = str::from_utf8(&stat);
	if stat_res.is_err() {
		let staterr = stat_res.expect_err("No error in git command");
		log::error!("[get_excluded_files] git diff stat command failed to start : {:?}", staterr);
		return None;
	}
	let statstr = stat_res.expect("Uncaught error in stat_res");
	log::debug!("[get_excluded_files] statstr = {}", statstr);
	return process_statoutput(statstr);
}

fn process_statoutput(statstr: &str) -> Option<(Vec<StatItem>, Vec<StatItem>)>{
    let statvec = process_statitems(statstr);
    let mut excluded_files = Vec::<StatItem>::new();
    let mut filtered_files = Vec::<StatItem>::new();
    let line_threshold = 500;
    for item in statvec {
        // logic for exclusion
        if (item.additions > line_threshold) || 
        (item.deletions > line_threshold) || 
        (item.additions + item.deletions > line_threshold) ||
		(item.deletions < 1) {
            excluded_files.push(item);
        }
        else {
            filtered_files.push(item);
        }
    }
    return Some((excluded_files, filtered_files));
}

fn generate_statitem(statitems: &Vec<&str>) -> StatItem {
	let statitem = StatItem {
		filepath: statitems[2].to_string(),
		additions: match statitems[0].to_string().parse() {
			Ok(adds) => {adds}
			Err(e) => {
				log::error!("[generate_statitem] Unable to parse additions: {:?}", e);
				0 // default value
			}
		},
		deletions: match statitems[1].to_string().parse() {
			Ok(dels) => {dels}
			Err(e) => {
				log::error!("[generate_statitem] Unable to parse deletions: {:?}", e);
				0 // default value
			}
		},
	};
	return statitem;
}

fn process_statitem(line: &str) -> Option<StatItem> {
	let statitems: Vec<&str> = line.split("\t").collect();
	if statitems.len() >= 3 {
		let statitem = generate_statitem(&statitems);
		return Some(statitem);
	}
	return None;
}

fn process_statitems(statstr: &str) -> Vec<StatItem> {
    let statlines = statstr.split("\n");
    let mut statvec = Vec::<StatItem>::new();
    for line in statlines {
		let statitem_opt = process_statitem(line);
		if statitem_opt.is_none() {
			continue;
		}
		let statitem = statitem_opt.expect("statitem is empty");
		statvec.push(statitem);
	}
    return statvec;
}

pub fn generate_diff(review: &Review, smallfiles: &Vec<StatItem>) -> HashMap<String, String> {
	let mut diffmap = HashMap::<String, String>::new();
	let prev_commit = review.base_head_commit();
	let curr_commit = review.pr_head_commit();
	let clone_dir = review.clone_dir();
	for item in smallfiles {
		let filepath = item.filepath.as_str();
		let commit_range = format!("{}...{}", prev_commit, curr_commit);
		log::debug!("[generate_diff] | clone_dir = {:?}, filepath = {:?}", clone_dir, filepath);
		let output_res = Command::new("git")
			.arg("diff")
			.arg("-U0")
			.arg(&commit_range)
			.arg(&filepath)
			.current_dir(clone_dir)
			.output();
		if output_res.is_err() {
			let commanderr = output_res.expect_err("No error in output_res");
			log::error!("[generate_diff] git diff command failed to start : {:?}", commanderr);
			continue;
		}
		let result = output_res.expect("Uncaught error in output_res");
		let diff = result.stdout;
		let diffstr_res = str::from_utf8(&diff);
		if diffstr_res.is_err() {
			let e = diffstr_res.expect_err("No error in diffstr_res");
			log::error!("[generate_diff] Unable to deserialize diff: {:?}", e);
			continue;
		}
		let diffstr = diffstr_res.expect("Uncaught error in diffstr_res");
		log::debug!("[generate_diff] diffstr = {}", &diffstr);
		diffmap.insert(filepath.to_string(), diffstr.to_string());
	}
	return diffmap;
}

fn process_diff(filepath: &str, diff: &str, linemap: &mut HashMap<String, Vec<String>> ) -> HashMap<String, Vec<String>> {
	let mut limiterpos = Vec::new();
	let delimitter = "@@";
	for (idx, _) in diff.match_indices(delimitter) {
		if has_deletions(&diff[idx..]) {
            limiterpos.push(idx);
        }
	}
	let mut idx: usize = 0;
	let len = limiterpos.len();
	while (idx + 1) < len {
		let line_res = diff.get(
			(limiterpos[idx]+delimitter.len())..limiterpos[idx+1]
		);
		if line_res.is_none() {
			log::error!("[process_diff] Unable to format diff line");
			continue;
		}
		let line = line_res.expect("Empty line_res");
		let sublines: Vec<&str> = line.split(" ").collect();
		if line.contains("\n") || sublines.len() != 4 {
			idx += 1;
			continue;
		}
		let mut deletionstr = sublines[1].to_owned();
		// let additionstr = sublines[1];
		if deletionstr.contains("-") {
			deletionstr = deletionstr.replace("-", "");
			if deletionstr.contains(",") {
				let delsplit: Vec<&str> = deletionstr.split(",").collect();
				let delidx_res = delsplit[0].parse::<i32>();
				let deldiff_res = delsplit[1].parse::<i32>();
				if delidx_res.is_err() || deldiff_res.is_err() {
					log::error!("[process_diff] Unable to parse delidx_res or deldiff_res: {:?} {:?}",
						delidx_res, deldiff_res);
					continue;
				}
				let delidx = delidx_res.expect("Uncaught error in delidx_res");
				let deldiff = deldiff_res.expect("Uncaught error in deldiff_res");
				deletionstr = format!("{delidx},{}", delidx+deldiff);
			}
			else {
				let delidx_res = deletionstr.parse::<i32>();
				if delidx_res.is_err() {
					log::error!("[process_diff] Unable to parse delidx_res {:?}",
						delidx_res);
					continue;
				}
				let delidx = delidx_res.expect("Uncaught error in delidx_res");
				deletionstr.push_str(format!(",{}", delidx).as_str());
			}
		}
		else {
			idx += 1;
			continue;
		}
		if linemap.contains_key(filepath) {
			let linemap_mut_res = linemap.get_mut(filepath);//.unwrap().push(deletionstr);
			if linemap_mut_res.is_none() {
				log::error!("[process_diff] Unable to get mutable ref for linemap: {:?}", linemap);
				continue;
			}
			let linemap_mut = linemap_mut_res.expect("Empty linemap_mut_res");
			linemap_mut.push(deletionstr);
		}
		else {
			linemap.insert(filepath.to_string(), vec!(deletionstr));
		}
		idx += 1;
	}
	return linemap.to_owned();
}

fn has_deletions(hunk: &str) -> bool {
    // Split the hunk into lines
    let lines = hunk.split('\n').collect::<Vec<&str>>();
    
    // Iterate through the lines to check for deletions
    for line in lines.iter() {
        if line.starts_with('-') && !line.starts_with("---") {
            return true;
        }
    }
    
    return false;
}

pub fn process_diffmap(diffmap: &HashMap<String, String>) -> HashMap<String, Vec<String>> {
	let mut linemap: HashMap<String, Vec<String>> = HashMap::new();
	for (filepath, diff) in diffmap {
		linemap = process_diff(filepath, diff, &mut linemap);
	}
	return linemap;
}

pub async fn generate_blame(review: &Review, linemap: &HashMap<String, Vec<String>>) ->  Vec<BlameItem>{
	let mut blamevec = Vec::<BlameItem>::new();
	let commit = review.base_head_commit();
	let clone_dir = review.clone_dir();
	for (path, linevec) in linemap {
		for line in linevec {
			let linenumvec: Vec<&str> = line.split(",").collect();
			let linenum = linenumvec[0];
			let paramvec: Vec<&str> = vec!(
				"blame",
				commit,
				"-L",
				line.as_str(),
				"-w",
				"-e",
				"--date=unix",
				"-l",
				path.as_str(),
			);
			let blame_res = Command::new("git").args(paramvec)
				.current_dir(clone_dir)
				.output();
			if blame_res.is_err() {
				let e = blame_res.expect_err("No error in blame_res");
				log::error!("[generate_blame] git blame command failed to start : {e}");
				continue;
			}
			let blame_output = blame_res.expect("Uncaught error in blame_res");
			if !blame_output.status.success() {
                log::error!("[generate_blame] git blame command failed with exit code {:?} and error: {:?}",
					blame_output.status.code(), String::from_utf8_lossy(&blame_output.stderr));
                continue;
            }
			let blame = blame_output.stdout;
			let parse_res = str::from_utf8(&blame);
			if parse_res.is_err() {
				let e = parse_res.expect_err("No error in parse_res");
				log::error!("[generate_blame] Unable to deserialize blame: {e}");
			}
			let blamestr = parse_res.expect("Uncaught error in parse_res");
			log::debug!("[generate_blame] blamestr = {}", blamestr);
			let blamelines: Vec<&str> = blamestr.lines().collect();
			if blamelines.len() == 0 {
				continue;
			}
			let blamitems_opt = process_blameitem(path, linenum, blamelines).await;
			if blamitems_opt.is_some() {
				let blameitems = blamitems_opt.expect("blameitem not found in blameitem_opt");
				blamevec.extend(blameitems);
			}
		}
	}
	return blamevec;
}

async fn process_blameitem(path: &str, linenum: &str, blamelines: Vec<&str>) -> Option<Vec<BlameItem>> {
	let linenumint_res = linenum.parse::<usize>();
	let mut blamevec = Vec::<BlameItem>::new();
	if linenumint_res.is_err() {
		let e = linenumint_res.expect_err("No error found in linenumint_res");
		log::error!("[generate_blame] Unable to parse linenum : {:?}", e);
		return None;
	}
	let linenumint = linenumint_res.expect("Uncaught error in linenumint_res");
	let lineauthormap = process_blamelines(&blamelines, linenumint).await;
	let mut linebreak = linenumint;
	for lidx in linenumint..(linenumint + blamelines.len()-1) {
		if lineauthormap.contains_key(&lidx) && lineauthormap.contains_key(&(lidx+1)) {
			let lineitem = lineauthormap.get(&lidx).expect("lidx checked");
			if lineitem.author_id() == 
			lineauthormap.get(&(lidx+1)).expect("lidx+1 checked").author_id() {
				continue;
			}
			else {
				blamevec.push(BlameItem::new(
					lineitem.author_id().to_string(),
					lineitem.timestamp().to_string(),
					linebreak.to_string(),
					lidx.to_string(),
					digest(path),
					lineitem.commit().trim_matches('"').to_string(),
					path.to_string())
				);
				linebreak = lidx + 1;
			}
		}
	}
	let lastidx = linenumint + blamelines.len()-1;
	if lineauthormap.contains_key(&lastidx) {
		let lineitem = lineauthormap.get(&lastidx).expect("lastidx checked");
		blamevec.push(BlameItem::new(
			lineitem.author_id().to_string(),
			lineitem.timestamp().to_string(),
			linebreak.to_string(),
			lastidx.to_string(),
			digest(path),
			lineitem.commit().to_string(),
			path.to_string(),
		));
	}
	return Some(blamevec);
}

async fn process_blamelines(blamelines: &Vec<&str>, linenum: usize) -> HashMap<usize, LineItem> {
	let mut linemap = HashMap::<usize, LineItem>::new();
	for lnum  in 0..blamelines.len() {
		let blame_line = blamelines[lnum];
		let blame_line_words: Vec<&str> = blame_line.split(" ").collect();
		let commit = blame_line_words[0].to_string();
		let (author, idx) = extract_author(&blame_line_words);
		let timestamp = extract_timestamp(&blame_line_words, idx);
		let lineitem = LineItem::new(author, timestamp, commit);
		linemap.insert(
			linenum + lnum,
			lineitem
		);
	}
	return linemap;
}

fn extract_author(blame_line_words: &Vec<&str>) -> (String, usize) {
	let mut author = blame_line_words[1];
	let mut idx = 1;
	// Check if the second value is an email address (enclosed in angle brackets)
	if !author.starts_with('(') && !author.ends_with('>') {
		// Shift the index to the next non-empty value
		while idx < blame_line_words.len() && (blame_line_words[idx] == "" || !blame_line_words[idx].starts_with('(')){
			idx += 1;
		}
		if idx < blame_line_words.len() {
			author = blame_line_words[idx];
		}
	} else {
		// Remove the angle brackets from the email address
		author = author.trim_start_matches('<').trim_end_matches('>');
	}
	let authorstr = author.replace("(", "")
		.replace("<", "")
		.replace(">", "");
	return (authorstr, idx)
}

fn extract_timestamp(wordvec: &Vec<&str>, mut idx: usize) -> String {
	let mut timestamp = wordvec[2];
	if timestamp == "" || timestamp.starts_with('(') {
		idx = idx + 1;
		while idx < wordvec.len() && (wordvec[idx] == "" || wordvec[idx].starts_with('(')) {
			idx = idx + 1;
		}
		if idx < wordvec.len() {
			timestamp = wordvec[idx];
		}
	}
	return timestamp.to_string();
}

pub fn create_clone_url(git_url: &str, access_token: &str, repo_provider: &str) -> Option<String> {
	let mut clone_url = None;
	if repo_provider == "github" {
		clone_url = Some(git_url.to_string()
			.replace("git@", format!("https://x-access-token:{access_token}@").as_str())
			.replace("github.com:", "github.com/"));
	} else if repo_provider == "bitbucket" {
		clone_url = Some(git_url.to_string()
			.replace("git@", format!("https://x-token-auth:{{{access_token}}}@").as_str())
			.replace("bitbucket.org:", "bitbucket.org/"));
	}
	return clone_url;
}

pub fn set_git_remote_url(git_url: &str, directory: &str, access_token: &str, repo_provider: &str) {
    let clone_url_opt = create_clone_url(git_url, access_token, repo_provider);
    if clone_url_opt.is_none() {
        log::error!("[set_git_remote_url] Unable to create clone url for repo provider {:?}, empty clone_url_opt",
			repo_provider);
        return;
    }
    let clone_url = clone_url_opt.expect("empty clone_url_opt");
    let output = Command::new("git")
		.arg("remote").arg("set-url").arg("origin")
		.arg(clone_url)
		.current_dir(directory)
		.output()
		.expect("failed to execute git pull");
    // Only for debug purposes
	match str::from_utf8(&output.stderr) {
		Ok(v) => log::debug!("[set_git_remote_url] stderr = {:?}", v),
		Err(e) => log::error!("[set_git_remote_url] stderr error: {}", e), 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => log::debug!("[set_git_remote_url] stdout = {:?}", v),
		Err(e) => log::error!("[set_git_remote_url] stdout error: {}", e), 
	};
	log::debug!("[set_git_remote_url] git pull output = {:?}, {:?}", &output.stdout, &output.stderr);
}

pub async fn clone_git_repo(repo: &mut Repository, access_token: &str, repo_provider: &str) {
    let git_url = repo.clone_ssh_url();
    // call function for provider specific git url formatting
    let clone_url_opt = create_clone_url(git_url, access_token, repo_provider);
    if clone_url_opt.is_none() {
        log::error!("[clone_git_repo] Unable to create clone url for repo provider {:?}, empty clone_url_opt",
			repo_provider);
        return;
    }
    let clone_url = clone_url_opt.expect("empty clone_url_opt");
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let mut directory = format!("/tmp/{}/{}/{}", repo.provider(), 
        repo.workspace(), random_string);
    // Check if directory exists
    let exists_res = fs::metadata(&directory).await;
    if exists_res.is_err() {
        let e = exists_res.expect_err("No error in exists_res");
        log::debug!("[clone_git_repo] executing metadata in {:?}, output: {:?}",
                &directory, e);
        if e.kind() != ErrorKind::NotFound {
            return;
        }
    }
    let remove_dir_opt = fs::remove_dir_all(&directory).await;
    if remove_dir_opt.is_err() {
        let e = remove_dir_opt.expect_err("No error in remove_dir_opt");
        log::debug!("[clone_git_repo] Execute in directory: {:?}, remove_dir_all: {:?}",
            &directory, e);
        if e.kind() != ErrorKind::NotFound {
            return;
        }
    }
    let create_dir_opt = fs::create_dir_all(&directory).await;
    if create_dir_opt.is_err() {
        let e = create_dir_opt.expect_err("No error in create_dir_opt");
        log::debug!("[clone_git_repo] Executing in directory: {:?}, create_dir_all: {:?}",
            &directory, e);
        if e.kind() != ErrorKind::NotFound {
            return;
        }
    }
    log::debug!("[clone_git_repo] directory exists? {}", fs::metadata(&directory).await.is_ok());
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg(clone_url).current_dir(&directory);
    let output_res = cmd.output();
    if output_res.is_err() {
        let e = output_res.expect_err("No error in output_res in git clone");
        log::error!("[clone_git_repo] Executing in directory: {:?}, git clone: {:?}",
            &directory, e);
        return;
    }
    let output = output_res.expect("Uncaught error in output_res");
	match str::from_utf8(&output.stderr) {
		Ok(v) => log::debug!("[clone_git_repo] stderr = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[clone_git_repo] git clone stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => log::debug!("[clone_git_repo] stdout = {:?}", v),
		Err(e) => {/* error handling */ log::error!("[clone_git_repo] git clone stdout error {}", e)}, 
	};
    directory = format!("{}/{}", &directory, repo.name());
    repo.set_local_dir(&directory);
    save_repo_to_db(repo);
}

pub fn get_git_aliases(repo: &Repository) -> Option<Vec<String>> {
	let local_dir_opt = repo.local_dir().to_owned();
	if local_dir_opt.is_none() {
		return None;
	}
	let local_dir = local_dir_opt.expect("Empty local_dir");
	let output = Command::new("git")
		.arg("log")
		.arg("--all")
		.arg("--format=%ae")
		.current_dir(&local_dir)
		.output()
		.expect("Failed to execute git log");
	let emails: Vec<String> = match str::from_utf8(&output.stdout) {
        Ok(output) => output
            .lines()
            .map(|line| line.trim().to_string())
            .collect(),
        Err(_) => return None, // Return None if unable to parse output
    };
	// Sort and remove duplicates
    let mut unique_emails: Vec<String> = emails.into_iter().collect();
    unique_emails.sort();
    unique_emails.dedup();

    // Only for debug purposes
	match str::from_utf8(&output.stderr) {
		Ok(v) => log::debug!("[set_git_remote_url] stderr = {:?}", v),
		Err(e) => log::error!("[set_git_remote_url] stderr error: {}", e), 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => log::debug!("[set_git_remote_url] stdout = {:?}", v),
		Err(e) => log::error!("[set_git_remote_url] stdout error: {}", e), 
	};
	log::debug!("[set_git_remote_url] git pull output = {:?}, {:?}", &output.stdout, &output.stderr);

	return Some(unique_emails);
}
