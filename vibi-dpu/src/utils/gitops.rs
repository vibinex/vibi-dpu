use std::collections::HashMap;
use std::process::Command;
use std::str;
use serde::Deserialize;
use serde::Serialize;
use sha256::digest;

use crate::bitbucket::auth::refresh_git_auth;
use crate::bitbucket::user::get_commit_bb;

use super::hunk::BlameItem;
use super::review::Review;
use super::lineitem::LineItem;

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
		eprintln!("Failed to start git rev-list: {:?}", e);
		return false;
	}
	let output = output_res.expect("Uncaught error in output_res");
	println!("Execute git rev-list, exit code: {:?}", output.status.code());
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("git rev-list stderr = {:?}", v),
		Err(e) => {/* error handling */ println!("git rev-list stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("git rev-list stdout = {:?}", v),
		Err(e) => {/* error handling */ println!("git rev-list stdout error {}", e)}, 
	};
	return true;
}

pub async fn git_pull(review: &Review) {
	let directory = review.clone_dir();
	println!("directory = {}", &directory);
	let access_token_opt = refresh_git_auth(review.clone_url(), review.clone_dir()).await;
	if access_token_opt.is_none() {
		eprintln!("Unable to get access_token from refresh_git_auth");
		return;
	}
	let access_token = access_token_opt.expect("Empty access_token");
    set_git_url(review.clone_url(), directory, &access_token);
	let output_res = Command::new("git")
		.arg("pull")
		.current_dir(directory)
		.output();
	if output_res.is_err() {
		let e = output_res.expect_err("No error in output_res");
		eprintln!("failed to execute git pull: {:?}", e);
		return;
	}
	let output = output_res.expect("Uncaught error in output_res");
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("git pull stderr = {:?}", v),
		Err(e) => {/* error handling */ println!("git pull stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("git pull stdout = {:?}", v),
		Err(e) => {/* error handling */ println!("git pull stdout error {}", e)}, 
	};
}

fn set_git_url(git_url: &str, directory: &str, access_token: &str) {
    let clone_url = git_url.to_string()
        .replace("git@", format!("https://x-token-auth:{{{access_token}}}@").as_str())
        .replace("bitbucket.org:", "bitbucket.org/");
    let output_res = Command::new("git")
		.arg("remote").arg("set-url").arg("origin")
		.arg(clone_url)
		.current_dir(directory)
		.output();
	if output_res.is_err() {
		let e = output_res.expect_err("No error in output_res");
		eprintln!("failed to execute set_git_url: {:?}", e);
		return;
	}
	let output = output_res.expect("Uncaught error in output_res");
	if !output.status.success() {
		eprintln!("set_git_url failed with exit code: {}", output.status);
		return;
	}
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("set_git_url stderr = {:?}", v),
		Err(e) => {/* error handling */ eprintln!("set_git_url stderr error {}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("set_git_url stdout = {:?}", v),
		Err(e) => {/* error handling */ eprintln!("set_git_url stdout error {}", e)}, 
	};
	println!("set_git_url output = {:?}, {:?}", &output.stdout, &output.stderr);
}

pub fn get_excluded_files(review: &Review) -> Option<(Vec<StatItem>, Vec<StatItem>)> {
	let prev_commit = review.base_head_commit();
	let next_commit = review.pr_head_commit();
	let clone_dir = review.clone_dir();
	println!("prev_commit = {}, next commit = {}, clone_dir = {}", prev_commit, next_commit, clone_dir);
	let git_res = Command::new("git")
		.args(&["diff", prev_commit, next_commit, "--numstat"])
		.current_dir(clone_dir)
		.output();
	if git_res.is_err() {
		let commanderr = git_res.expect_err("No error in git command");
		eprintln!("git diff stat command failed to start : {:?}", commanderr);
		return None;
	}
	let resultstat = git_res.expect("Uncaught error in git_res");
	let stat = resultstat.stdout;
	// parse the output
	let stat_res = str::from_utf8(&stat);
	if stat_res.is_err() {
		let staterr = stat_res.expect_err("No error in git command");
		eprintln!("git diff stat command failed to start : {:?}", staterr);
		return None;
	}
	let statstr = stat_res.expect("Uncaught error in stat_res");
	println!("statstr = {}", statstr);
	return process_statoutput(statstr);
}

fn process_statoutput(statstr: &str) -> Option<(Vec<StatItem>, Vec<StatItem>)>{
    let statvec = process_statitems(statstr);
    let mut bigfiles = Vec::<StatItem>::new();
    let mut smallfiles = Vec::<StatItem>::new();
    let line_threshold = 500;
    for item in statvec {
        // logic for exclusion
        if (item.additions > line_threshold) || 
        (item.deletions > line_threshold) || 
        (item.additions + item.deletions > line_threshold) {
            bigfiles.push(item);
        }
        else {
            smallfiles.push(item);
        }
    }
    return Some((bigfiles, smallfiles));
}

fn generate_statitem(statitems: &Vec<&str>) -> StatItem {
	let mut additions = 0;
	let statitem = StatItem {
		filepath: statitems[2].to_string(),
		additions: match statitems[0].to_string().parse() {
			Ok(adds) => {adds}
			Err(e) => {
				eprintln!("Unable to parse additions: {:?}", e);
				0 // default value
			}
		},
		deletions: match statitems[1].to_string().parse() {
			Ok(dels) => {dels}
			Err(e) => {
				eprintln!("Unable to parse deletions: {:?}", e);
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
		let params = vec![
		"diff".to_string(),
		format!("{prev_commit}:{filepath}"),
		format!("{curr_commit}:{filepath}"),
		"-U0".to_string(),
		];
		let output_res = Command::new("git").args(&params)
		.current_dir(&clone_dir)
		.output();
		if output_res.is_err() {
			let commanderr = output_res.expect_err("No error in output_res");
			eprintln!("git diff command failed to start : {:?}", commanderr);
			continue;
		}
		let result = output_res.expect("Uncaught error in output_res");
		let diff = result.stdout;
		let diffstr_res = str::from_utf8(&diff);
		if diffstr_res.is_err() {
			let e = diffstr_res.expect_err("No error in diffstr_res");
			eprintln!("Unable to deserialize diff: {:?}", e);
			continue;
		}
		let diffstr = diffstr_res.expect("Uncaught error in diffstr_res");
		println!("diffstr = {}", &diffstr);
		diffmap.insert(filepath.to_string(), diffstr.to_string());
	}
	return diffmap;
}

fn process_diff(filepath: &str, diff: &str, linemap: &mut HashMap<String, Vec<String>> ) -> HashMap<String, Vec<String>> {
	let mut limiterpos = Vec::new();
	let delimitter = "@@";
	for (idx, _) in diff.match_indices(delimitter) {
		limiterpos.push(idx);
	}
	let mut idx: usize = 0;
	let len = limiterpos.len();
	while (idx + 1) < len {
		let line_res = diff.get(
			(limiterpos[idx]+delimitter.len())..limiterpos[idx+1]
		);
		if line_res.is_none() {
			eprintln!("Unable to format diff line");
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
				let delidx = delsplit[0].parse::<i32>().unwrap();
				let deldiff = delsplit[1].parse::<i32>().unwrap();
				deletionstr = format!("{delidx},{}", delidx+deldiff);
			}
			else {
				let delidx = deletionstr.parse::<i32>().unwrap();
				deletionstr.push_str(format!(",{}", delidx).as_str());
			}
		}
		else {
			idx += 1;
			continue;
		}
		if linemap.contains_key(filepath) {
			linemap.get_mut(filepath).unwrap().push(deletionstr);
		}
		else {
			linemap.insert(filepath.to_string(), vec!(deletionstr));
		}
		idx += 1;
	}
	return linemap.to_owned();
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
	let commit = review.pr_head_commit();
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
				"-e",
				"--date=unix",
				path.as_str(),
			);
			let blame_res = Command::new("git").args(paramvec)
				.current_dir(clone_dir)
				.output();
			if blame_res.is_err() {
				let e = blame_res.expect_err("No error in blame_res");
				eprintln!("git blame command failed to start : {e}");
				continue;
			}
			let blame_output = blame_res.expect("Uncaught error in blame_res");
			if !blame_output.status.success() {
                eprintln!("git blame command failed with exit code {:?} and error: {:?}",
					blame_output.status.code(), String::from_utf8_lossy(&blame_output.stderr));
                continue;
            }
			let blame = blame_output.stdout;
			let parse_res = str::from_utf8(&blame);
			if parse_res.is_err() {
				let e = parse_res.expect_err("No error in parse_res");
				eprintln!("Unable to deserialize blame: {e}");
			}
			let blamestr = parse_res.expect("Uncaught error in parse_res");
			println!("blamestr = {}", blamestr);
			let blamelines: Vec<&str> = blamestr.lines().collect();
			if blamelines.len() == 0 {
				continue;
			}
			let blamitems_opt = process_blameitem(path, commit, linenum, blamelines, review).await;
			if blamitems_opt.is_some() {
				let blameitems = blamitems_opt.expect("blameitem not found in blameitem_opt");
				blamevec.extend(blameitems);
			}
		}
	}
	return blamevec;
}

async fn process_blameitem(path: &str, commit: &str, linenum: &str, blamelines: Vec<&str>, review: &Review) -> Option<Vec<BlameItem>> {
	let linenumint_res = linenum.parse::<usize>();
	let mut blamevec = Vec::<BlameItem>::new();
	if linenumint_res.is_err() {
		let e = linenumint_res.expect_err("No error found in linenumint_res");
		eprintln!("Unable to parse linenum : {:?}", e);
		return None;
	}
	let linenumint = linenumint_res.expect("Uncaught error in linenumint_res");
	let lineauthormap = process_blamelines(&blamelines, linenumint,
		&review.repo_name(), &review.repo_owner()).await;
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
					digest(path) ));
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
			digest(path)));
	}
	return Some(blamevec);
}

async fn process_blamelines(blamelines: &Vec<&str>, linenum: usize,
    repo_name: &str, repo_owner: &str) -> HashMap<usize, LineItem> {
	let mut linemap = HashMap::<usize, LineItem>::new();
	for lnum  in 0..blamelines.len() {
		let ln = blamelines[lnum];
		let wordvec: Vec<&str> = ln.split(" ").collect();
        let commit = wordvec[0];
        let lineitem_opt = get_commit_bb(commit, repo_name, repo_owner).await;
		if lineitem_opt.is_some() {
			let lineitem = lineitem_opt.expect("Empty linemap_opt");
			linemap.insert(
				linenum + lnum,
				lineitem
			);
		}
	}
	return linemap;
}