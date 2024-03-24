use crate::db::prs::update_pr_info_in_db;
use crate::utils::{pr_info::PrInfo, reqwest_client::get_client};
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::HashMap;
use std::str;

use super::config::{bitbucket_base_url, prepare_auth_headers};

pub async fn list_prs_bitbucket(repo_owner: &str,repo_name: &str,access_token: &str,state: &str,) -> Option<Vec<String>> {
    let headers_opt = prepare_auth_headers(access_token);
    if headers_opt.is_none() {
        log::error!("[list_prs_bitbucket] Unable to prepare auth headers: {}", repo_name);
        return None;
    }
    let headers = headers_opt.expect("Empty headers_opt");
    let mut params = HashMap::new();
    params.insert("state".to_string(), state.to_string());
    let pr_list_opt = get_list_prs(&headers, &params, repo_owner, repo_name).await;
    return pr_list_opt;
}

async fn get_list_prs(headers: &HeaderMap, params: &HashMap<String, String>, repo_owner: &str, repo_name: &str) -> Option<Vec<String>> {
    let client = get_client();
    let base_url = bitbucket_base_url();
    let response_result = client
        .get(&format!(
            "{}/repositories/{}/{}/pullrequests",
            &base_url, repo_owner, repo_name
        ))
        .headers(headers.to_owned())
        .query(params)
        .send()
        .await;
    if response_result.is_err() {
        let e = response_result.expect_err("No error in sending request");
        log::error!("[get_list_prs] Failed to send the request {:?}", e);
        return None;
    }

    let response = response_result.expect("Uncaught error in parsing response");
    if !response.status().is_success() {
        log::error!(
            "[get_list_prs] Request failed with status: {:?}",
            response.status()
        );
        return None;
    }

    let parse_result = response.json::<Value>().await;
    if parse_result.is_err() {
        let parse_result_err = parse_result.expect_err("No error in parsing");
        log::error!(
            "[get_list_prs] Failed to parse JSON: {:?}",
            parse_result_err
        );
        return None;
    }
    let prs_data = parse_result.expect("Uncaught error in parsing Prs data");
    let pr_list_parse_res = serde_json::from_value(prs_data["values"].clone());
    if pr_list_parse_res.is_err() {
        let e = pr_list_parse_res.expect_err("Empty error in pr_list_parse_res");
        log::error!("[get_list_prs] Unable to parse get_list_prs: {:?}", e);
        return None;
    }
    let pr_list_parsed: Vec<Value> =
        pr_list_parse_res.expect("Uncaught error in pr_list_parse_res");
    let mut pr_list: Vec<String> = Vec::new();
    for pr in pr_list_parsed.iter() {
        pr_list.push(pr["id"].to_string().trim_matches('"').to_string());
    }
    if pr_list.is_empty() {
        log::error!(
            "[get_list_prs] pr_list is empty for parsed value: {:?}",
            &pr_list_parsed
        );
        return None;
    }
    return Some(pr_list);
}

pub async fn get_pr_info(workspace_slug: &str,repo_slug: &str,access_token: &str,pr_number: &str) -> Option<PrInfo> {
    let base_url = bitbucket_base_url();
    let url = format!(
        "{}/repositories/{}/{}/pullrequests/{}",
        &base_url, workspace_slug, repo_slug, pr_number
    );
    log::debug!("[get_pr_info] url: {:?}", &url);
    log::debug!("[get_pr_info] access token: {:?}", access_token);
    let client = get_client();
    let response_result = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .send()
        .await;

    if response_result.is_err() {
        let res_err = response_result.expect_err("No error in getting Pr response");
        log::error!("[get_pr_info] Error getting PR info: {:?}", res_err);
        return None;
    }
    let response = response_result.expect("Uncaught error in response");
    if !response.status().is_success() {
        log::error!("[get_pr_info] Failed to get PR info, response: {:?}", response);
        return None;
    }
    let pr_data: Value = response.json().await.expect("Error parsing PR data");
    let pr_info = PrInfo {
        base_head_commit: pr_data["destination"]["commit"]["hash"].to_string().trim_matches('"').to_string(),
        pr_head_commit: pr_data["source"]["commit"]["hash"].to_string().trim_matches('"').to_string(),
        state: pr_data["state"].to_string().trim_matches('"').to_string(),
        pr_branch: pr_data["source"]["branch"]["name"].to_string().trim_matches('"').to_string(),
        author: None,
    };
    log::debug!("[get_pr_info] pr_info: {:?}", &pr_info);
    Some(pr_info)
}

pub async fn get_and_store_pr_info(workspace_slug: &str, repo_slug: &str, access_token: &str, pr_number: &str) {
    let repo_provider = "bitbucket";
    if let Some(pr_info) = get_pr_info(workspace_slug, repo_slug, access_token, pr_number).await {
        // If PR information is available, store it in the database
        update_pr_info_in_db(workspace_slug, repo_slug, &pr_info, pr_number, repo_provider).await;
    } else {
        log::error!(
            "[get_and_store_pr_info] No PR info available for PR number: {:?} repository: {:?} repo_owner{:?}",
            pr_number, repo_slug, workspace_slug
        );
    }
}
