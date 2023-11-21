use serde_json::{json, Value};

use crate::utils::{review::Review, hunk::BlameItem, reqwest_client::get_client};

use super::config::{prepare_headers, github_base_url};

pub async fn get_blame_user(blame: &BlameItem, review: &Review, access_token: &str) -> Option<String> {
    let body = prepare_body(blame, review);
    let headers_opt = prepare_headers(access_token);
    if headers_opt.is_none() {
        eprintln!("Unable to prepare headers for blame: {:?}", blame);
        return None;
    }
    let headers = headers_opt.expect("Empty headers_opt");
    let url = format!("{}/graphql", github_base_url());
    let client = get_client();
    let response_res = client.post(url).headers(headers).body(body).send().await;
    if response_res.is_err() {
        let e = response_res.expect_err("Empty error in response_res");
        eprintln!("[get_blame_user] Unable to get blame user for blame: {:?}, error: {:?}", blame, e);
        return None;
    }
    let response = response_res.expect("Uncaught error in response_res");
    let json_res = response.json::<Value>().await;
    if json_res.is_err() {
        let e = json_res.expect_err("Empty error in json_res");
        eprintln!("[get_blame_user] Unable to deserialize response, error: {:?}, blame: {:?}", e, blame);
        return None;
    }
    let json = json_res.expect("Uncaught error in json_res");
    let range_json_opt = json["data"]["repository"]["object"]["blame"]["ranges"].as_array();
    if range_json_opt.is_none() {
        eprintln!("[get_blame_user] Unable to get ranges for blame author deserialization: {:?}", &json);
        return None;
    }
    let range_json = range_json_opt.expect("Empty range_json_opt");
    for range in range_json {
        let start_res: Result<i32, _> = range["startingLine"].to_string().parse();
        let end_res: Result<i32, _> = range["endingLine"].to_string().parse();
        if start_res.is_err() || end_res.is_err() {
            continue;
        }
        let start_range = start_res.expect("Uncaught error in start_res");
        let end_range = end_res.expect("Uncaught error in end_res");
        let line_start_res: Result<i32, _> = blame.line_start().parse();
        let line_end_res: Result<i32, _> = blame.line_end().parse();
        if line_start_res.is_err() || line_end_res.is_err() {
            continue;
        }
        let blame_start = line_start_res.expect("Uncaught error in line_start_res");
        let blame_end = line_end_res.expect("Uncuaght error in line_end_res");
        if blame_start >= start_range && blame_end <= end_range {
            let user_opt = range["commit"]["author"]["user"]["login"].as_str();
            if user_opt.is_none() {
                continue;
            }
            let blame_login = user_opt.expect("Empty user_opt").to_string();
            return Some(blame_login);
        }
    }
    return None;
}

fn prepare_body(blame: &BlameItem, review: &Review) -> String {
    let query = format!(
        r#"
        {{
            repository(owner: "{}", name: "{}") {{
              object(oid: "{}") {{
                ... on Commit {{
                  blame(path: "{}") {{
                    ranges {{
                      age
                      commit {{
                        author {{
                          user {{
                            login
                          }}
                        }}
                      }}
                      startingLine
                      endingLine
                    }}
                  }}
                }}
              }}
            }}
          }}
        "#,
        review.repo_owner(), review.repo_name(), blame.commit().trim_matches('"'), blame.filepath().trim_matches('"')
    );
    let body = json!({
        "query": query
    });
    return body.to_string();
}