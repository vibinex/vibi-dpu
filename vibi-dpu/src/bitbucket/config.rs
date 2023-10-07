use std::{env, collections::HashMap};

use reqwest::{Response, header::{HeaderMap, HeaderValue}};
use serde_json::Value;

pub fn bitbucket_base_url() -> String {
    env::var("BITBUCKET_BASE_URL").expect("BITBUCKET_BASE_URL must be set")
}

pub async fn get_api(url: &str, access_token: &str, params: Option<HashMap<&str, &str>> ) -> Vec<Value> {
    let response_opt = call_get_api(url, access_token, &params).await;
    println!("response of get_api = {:?}", &response_opt);
    let (mut response_values, next_url) = deserialize_response(response_opt).await;
    if next_url.is_some() {
        let mut page_values = get_all_pages(next_url, access_token, &params).await;
        response_values.append(&mut page_values);
    }
    return response_values;
}

pub async fn call_get_api(url: &str, access_token: &str, params: &Option<HashMap<&str, &str>> ) -> Option<Response>{
    println!("GET api url = {}", url);
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new(); 
    headers.insert( reqwest::header::AUTHORIZATION, 
    format!("Bearer {}", access_token).parse().expect("Invalid auth header"), );
    headers.insert("Accept",
     "application/json".parse().expect("Invalid Accept header"));
    match params {
        Some(params) => {
            match client.get(url).headers(headers).query(params).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Some(response);
                    }
                    else { eprintln!("Failed to call API {}, status: {}", url, response.status()); }
                },
                Err(e) => { eprintln!("Error sending GET request to {}, error: {}", url, e); },
            };
        },
        None => {
            match client.get(url).headers(headers).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Some(response);
                    }
                    else { eprintln!("Failed to call API {}, status: {}", url, response.status()); }
                },
                Err(e) => { eprintln!("Error sending GET request to {}, error: {}", url, e); },
            };
        }
    };
    return None;
}

async fn deserialize_response(response_opt: Option<Response>) -> (Vec<Value>, Option<String>) {
    let values_vec = Vec::new();
    match response_opt {
        Some(response) => {
            match response.json::<serde_json::Value>().await {
                Ok(response_json) => {
                    let mut values_vec = Vec::new();
                    if let Some(values) = response_json["values"].as_array() {
                        for value in values {
                            values_vec.push(value.to_owned()); 
                        }
                        return (values_vec, Some(response_json["next"].to_string()));
                    }
                }
                Err(e) => { eprintln!("Unable to deserialize response: {}", e); }
            };
        },
        None => { eprintln!("Response is None");}
    };
    return (values_vec, None);
}

async fn get_all_pages(next_url: Option<String>, access_token: &str, params: &Option<HashMap<&str, &str>>) -> Vec<Value>{
    let mut values_vec = Vec::new();
    let mut next_url = next_url;
    while next_url.is_some() {
        let url = next_url.as_ref().expect("next_url is none").trim_matches('"');
        if url != "null" {
            let response_opt = call_get_api(url, access_token, params).await;
            let (mut response_values, url_opt) = deserialize_response(response_opt).await;
            next_url = url_opt.clone();
            values_vec.append(&mut response_values);    
        } else {
            break;
        }
    }
    return values_vec;
}

pub fn prepare_auth_headers(access_token: &str) -> HeaderMap{
    let mut headers_map = HeaderMap::new();
    let auth_header = format!("Bearer {}", access_token);
    let headervalres = HeaderValue::from_str(&auth_header);
    match headervalres {
        Ok(headerval) => {
            headers_map.insert("Authorization", headerval);
        },
        Err(e) => panic!("Could not parse header value: {}", e),
    };
    return headers_map;
}