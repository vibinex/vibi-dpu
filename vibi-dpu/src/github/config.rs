use std::{env, collections::HashMap};
use reqwest::{Response, header::{HeaderMap, HeaderValue, AUTHORIZATION, ACCEPT, USER_AGENT}, header};
use serde_json::Value;
use serde::{Deserialize, Serialize};

use crate::utils::reqwest_client::get_client;

// Helper struct for deserialized paginated response
#[derive(Debug, Serialize, Deserialize)]
struct PaginatedResponse {
    values: Vec<Value>,
    next_url: Option<String>,
}

pub fn github_base_url() -> String {
    env::var("GITHUB_BASE_URL").expect("BITBUCKET_BASE_URL must be set")
}

pub async fn get_api_values(url: &str, access_token: &str, params: Option<HashMap<&str, &str>> ) -> Vec<Value> {
    let headers = prepare_headers(access_token);
    let initial_response = get_api_response(url, None, &access_token, &params).await;

    let PaginatedResponse { mut values, next_url } = deserialize_paginated_response(initial_response).await;

    if next_url.is_some() {
        let mut additional_values = get_all_pages(next_url, &access_token, &params).await;
        values.append(&mut additional_values);
    }

    return values;
}

async fn get_api_response(url: &str, headers_opt: Option<reqwest::header::HeaderMap>, access_token: &str,  params: &Option<HashMap<&str, &str>>) -> Option<Response> {
    let mut headers;
    if headers_opt.is_none() {
        let headers_opt_new = prepare_headers(&access_token);
        if headers_opt_new.is_none() {
            eprintln!("Unable to prepare_headers, empty headers_opt");
            return None;
        }
        headers = headers_opt_new.expect("Empty headers_opt");
    } else {
        headers = headers_opt.expect("Empty headers_opt");
    }
    let client = get_client();
    let get_response = client.get(url)
        .headers(headers.clone())
        .query(params)
        .send()
        .await;

    if get_response.is_err() {
        let e = get_response.expect_err("No error in get_response");
        eprintln!("Error sending GET request without params to {}, error: {}", url, e);
        return None;
    }
    let response = get_response.expect("Uncaught error in get_res");
    if !response.status().is_success() {
        eprintln!("Failed to call Github API {}, status: {}", url, response.status());
        return None;
    }
    return Some(response);
}

async fn get_all_pages(next_url: Option<String>, access_token: &str, params: &Option<HashMap<&str, &str>>) -> Vec<Value>{
    let mut all_values = Vec::new();
    let mut next_url_mut = next_url;

    while next_url_mut.is_some() {
        let url_opt = next_url_mut.as_ref();
        if url_opt.is_none() {
            eprintln!("next_url is none");
            break;
        }
        let url = url_opt.expect("Empty next url_opt").trim_matches('"');
        if url == "null" {
            break;   
        }
        let response = get_api_response(&url, None, access_token, params).await;
        let PaginatedResponse { mut values, next_url } = deserialize_paginated_response(response).await;
        all_values.append(&mut values);
        next_url_mut = next_url.clone();
    }

    return all_values;
}

async fn deserialize_paginated_response(response_opt: Option<Response>) -> PaginatedResponse {
    let mut values_vec = Vec::new();
    if response_opt.is_none() {
        eprintln!("Response is None, can't deserialize");
        return PaginatedResponse {
            values: values_vec,
            next_url: None
        };
    }
    let response = response_opt.expect("Uncaught empty response_opt");
    let headers = response.headers().clone();
    let parse_res = response.json::<serde_json::Value>().await;
    if parse_res.is_err() {
        let e = parse_res.expect_err("No error in parse_res");
        eprintln!("Unable to deserialize response: {}", e);
        return PaginatedResponse {
            values: values_vec,
            next_url: None
        };
    }
    let response_json = parse_res.expect("Uncaught error in parse_res in deserialize_response");
    let res_values_opt = response_json["values"].as_array(); // TODO - find out if as_array is needed
    if res_values_opt.is_none() {
        eprintln!("response_json[values] is empty");
        return PaginatedResponse {
            values: values_vec,
            next_url: None
        };
    }
    let values = res_values_opt.expect("res_values_opt is empty");
    for value in values {
        values_vec.push(value.to_owned()); 
    }
    let next_url = extract_next_url(headers.get(header::LINK));

    return PaginatedResponse {
        values: values_vec.to_vec(),
        next_url: next_url
    };
}

fn extract_next_url(link_header: Option<&HeaderValue>) -> Option<String> {
    link_header.and_then(|value| {
        value.to_str().ok().and_then(|header_value| {
            header_value.split(',')
                .find(|part| part.contains(r#"rel="next""#))
                .and_then(|next_link_part| {
                    next_link_part.split(';')
                        .next()
                        .map(|url| url.trim_matches(&[' ', '<', '>'] as &[_]))
                        .map(str::to_string)
                })
        })
    })
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