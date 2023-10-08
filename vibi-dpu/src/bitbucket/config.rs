use std::{env, collections::HashMap};
use reqwest::{Response, header::{HeaderMap, HeaderValue}};
use serde_json::Value;
use crate::client::config::get_client;

pub fn bitbucket_base_url() -> String {
    env::var("BITBUCKET_BASE_URL").expect("BITBUCKET_BASE_URL must be set")
}

pub async fn get_api_values(url: &str, access_token: &str, params: Option<HashMap<&str, &str>> ) -> Vec<Value> {
    let response_opt = get_api_response(url, None, access_token, &params).await;
    println!("response of get_api = {:?}", &response_opt);
    let (mut response_values, next_url) = deserialize_paginated_response(response_opt).await;
    if next_url.is_some() {
        let mut page_values = get_all_pages(next_url, access_token, &params).await;
        response_values.append(&mut page_values);
    }
    return response_values;
}

pub async fn get_api_response(url: &str, headers_opt: Option<reqwest::header::HeaderMap>, access_token: &str, params: &Option<HashMap<&str, &str>>) -> Option<Response>{
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
    let get_res = client.get(url).headers(headers).send().await;
    if get_res.is_err() {
        let e = get_res.expect_err("No error in get_res");
        eprintln!("Error sending GET request without params to {}, error: {}", url, e);
        return None;
    }
    let response = get_res.expect("Uncaught error in get_res");
    if !response.status().is_success() {
        eprintln!("Failed to call API {}, status: {}", url, response.status());
        return None;
    }
    return Some(response);
}

async fn deserialize_paginated_response(response_opt: Option<Response>) -> (Vec<Value>, Option<String>) {
    let mut values_vec = Vec::new();
    if response_opt.is_none() {
        eprintln!("Response is None, can't deserialize");
        return (values_vec, None);
    }
    let response = response_opt.expect("Uncaught empty response_opt");
    let parse_res = response.json::<serde_json::Value>().await;
    if parse_res.is_err() {
        let e = parse_res.expect_err("No error in parse_res");
        eprintln!("Unable to deserialize response: {}", e);
        return (values_vec, None);
    }
    let response_json = parse_res
        .expect("Uncaught error in parse_res in deserialize_response");
    let res_values_opt = response_json["values"].as_array();
    if res_values_opt.is_none() {
        eprintln!("response_json[values] is empty");
        return (values_vec, None);
    }
    let values = res_values_opt.expect("res_values_opt is empty");
    for value in values {
        values_vec.push(value.to_owned()); 
    }
    return (values_vec, Some(response_json["next"].to_string()));
}

async fn get_all_pages(next_url: Option<String>, access_token: &str, params: &Option<HashMap<&str, &str>>) -> Vec<Value>{
    let mut values_vec = Vec::new();
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
        let response_opt = get_api_response(url, None, access_token, params).await;
        let (mut response_values, url_opt) = deserialize_paginated_response(response_opt).await;
        next_url_mut = url_opt.clone();
        values_vec.append(&mut response_values);
    }
    return values_vec;
}

pub fn prepare_auth_headers(access_token: &str) -> Option<HeaderMap>{
    let mut headers_map = HeaderMap::new();
    let auth_header = format!("Bearer {}", access_token);
    let headervalres = HeaderValue::from_str(&auth_header);
    if headervalres.is_err() {
        let e = headervalres.expect_err("No error found in headervalres");
        eprintln!("Could not parse header value: {}", e);
        return None;
    }
    let headerval = headervalres.expect("Empty headervalres");
    headers_map.insert("Authorization", headerval);
    return Some(headers_map);
}

pub fn prepare_headers(access_token: &str) -> Option<HeaderMap> {
    let mut headers = reqwest::header::HeaderMap::new(); 
    let auth_header_res = format!("Bearer {}", access_token).parse();
    if auth_header_res.is_err() {
        let e = auth_header_res
            .expect_err("Empty error in auth_header_res");
        eprintln!("Invalid auth header: {:?}", e);
        return None;
    }
    let header_authval = auth_header_res
        .expect("Uncaught error in auth_header_res");
    headers.insert( reqwest::header::AUTHORIZATION, header_authval);
    let header_accept_res =  "application/json".parse();
    if header_accept_res.is_err() {
        let e = header_accept_res
            .expect_err("No error in header_accept_res");
        eprintln!("Invalide accept header val, error: {:?}", e);
        return None;
    }
    let header_acceptval = header_accept_res
        .expect("Uncaught error in header_accept_res");
    headers.insert("Accept", header_acceptval);
    return Some(headers);
}