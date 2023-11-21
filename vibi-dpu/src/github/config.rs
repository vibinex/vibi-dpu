use std::{env, collections::HashMap};
use reqwest::{Response, header::{HeaderMap, HeaderValue, AUTHORIZATION, ACCEPT, USER_AGENT}, header};
use serde_json::Value;
use serde::{Deserialize, Serialize};

use crate::utils::reqwest_client::get_client;

pub fn github_base_url() -> String {
    env::var("GITHUB_BASE_URL").expect("BITBUCKET_BASE_URL must be set")
}

pub async fn get_api_paginated(url: &str, access_token: &str, params: Option<HashMap<&str, &str>> ) -> Option<Vec<Value>> {
    let mut is_first_call = true;
    let mut next_url_mut: Option<String> = None;
    let mut get_url = url.to_string();
    let mut values = Vec::<Value>::new();
    while is_first_call || next_url_mut.is_some() {
        if is_first_call {
            is_first_call = false;
        } else {
            get_url = next_url_mut.expect("Empty next_url_mut");
        }
        let response_opt = get_api_response(&get_url, None, &access_token, &params).await;
        if response_opt.is_none() {
            eprintln!("[get_api_paginated] Unable to call get api and get initial response for: 
                {:?}, {:?}, {:?}", url, access_token, params);
            return None
        }
        let response = response_opt.expect("Empty initial_response");
        let next_url = extract_next_url(&response);
        let deserialized_opt = deserialize_response(response).await;
        if deserialized_opt.is_none() {
            eprintln!("[get_api_paginated] deserialization failed for: 
                {:?}, {:?}, {:?}", url, access_token, params);
            return None
        }
        let deserialized_val = deserialized_opt.expect("Empty deserialized_opt");
        values.push(deserialized_val);
        next_url_mut = next_url.clone();
        
    }
    return Some(values);
}

async fn get_api_response(url: &str, headers_opt: Option<reqwest::header::HeaderMap>, access_token: &str,  params: &Option<HashMap<&str, &str>>) -> Option<Response> {
    let get_headers_opt = get_headers(&headers_opt, access_token);
    if get_headers_opt.is_none() {
        eprintln!("[get_api_response] Unable to prepare headers, headers_opt: {:?}", &headers_opt);
        return None;
    }
    let headers = get_headers_opt.expect("Uncaught error in get_headers_opt");
    let client = get_client();
    let get_response = client.get(url)
        .headers(headers.clone())
        .query(params)
        .send()
        .await;

    if get_response.is_err() {
        let e = get_response.expect_err("No error in get_response");
        eprintln!("[get_api_response] Error sending GET request without params
             to {}, error: {}", url, e);
        return None;
    }
    let response = get_response.expect("Uncaught error in get_res");
    if !response.status().is_success() {
        eprintln!("[get_api_response] Failed to call Github API {}, status: {}",
            url, response.status());
        return None;
    }
    return Some(response);
}

async fn deserialize_response(response: Response) -> Option<Value> {
    let res_val = response.json::<Value>().await;
    if res_val.is_err() {
        let e = res_val.expect_err("Empty error in res_val");
        eprintln!("[deserialize_response] Unable to deserialize response, error: {:?}", e);
        return None;
    }
    let deserialized = res_val.expect("Uncaught error in res_val");
    return Some(deserialized);
}

fn get_headers(headers_opt: &Option<reqwest::header::HeaderMap>, access_token: &str) -> Option<reqwest::header::HeaderMap> {
    let headers;
    if headers_opt.is_none() {
        let headers_opt_new = prepare_headers(access_token);
        if headers_opt_new.is_none() {
            eprintln!("[get_headers] Unable to prepare_headers, empty headers_opt");
            return None;
        }
        headers = headers_opt_new.expect("Empty headers_opt");
    } else {
        headers = headers_opt.to_owned().expect("Empty headers_opt");
    }
    return Some(headers);
}

fn extract_next_url(response: &Response) -> Option<String> {
    let headers = response.headers().clone();
    let link_header = headers.get(header::LINK);
    let next_url_opt = link_header.and_then(|value| {
        value.to_str().ok().and_then(|header_value| {
            header_value.split(',')
                .find(|part| part.contains(r#"rel="next""#))
                .and_then(|next_link_part| {
                    next_link_part.split(';')
                        .next()
                        .map(|url| url.trim_matches(&[' ', '<', '>', '"'] as &[_]))
                        .map(str::to_string)
                })
        })
    });
    if next_url_opt.as_ref().is_some_and(|url| url == "null") {
        return None;
    }
    return next_url_opt;
}

// TODO -find all "?" after await specially
pub fn prepare_headers(access_token: &str) -> Option<HeaderMap> {
    let mut headers = HeaderMap::new();

    let auth_header_res = format!("Bearer {}", access_token).parse();
    let accept_value = "application/vnd.github+json";

    if auth_header_res.is_err() {
        let e = auth_header_res.expect_err("Empty error in auth_header_res");
        eprintln!("Invalid auth header: {:?}", e);
        return None;
    }
    let header_authval = auth_header_res.expect("Uncaught error in auth_header_res");
    headers.insert(AUTHORIZATION, header_authval);

    let accept_header_res = HeaderValue::from_str(accept_value);
    if accept_header_res.is_err() {
        let e = accept_header_res.expect_err("Empty error in accept_header_res: {:?}");
        eprintln!("Could not parse Accept header value {}", e);
        return None;
    }
    let accept_header = accept_header_res.expect("Error parsing Accept header value");
    headers.insert(ACCEPT, accept_header);

    // User-Agent header is static, so we can use from_static
    let user_agent_header_res = HeaderValue::from_str("Vibinex code review Test App");
    if user_agent_header_res.is_err() {
        let e = user_agent_header_res.expect_err("Empty error in user_agent_hesder_res: {:?}");
        eprintln!("Could not parse User Agent header value");
        return None;
    }
    let user_agent_header = user_agent_header_res.expect("Error parsing User Agent header value");
    headers.insert(USER_AGENT, user_agent_header);

    return Some(headers)
}