use super::config::{bitbucket_base_url, get_api};
use crate::db::owner::save_workspace_to_db;
use crate::utils::owner::Workspace;
pub async fn get_bitbucket_workspaces(access_token: &str) -> Vec<Workspace> {
    let user_url = format!("{}/workspaces", bitbucket_base_url());
    let response = get_api(&user_url, access_token, None).await;
    let mut workspace_vec = Vec::new();
    for workspace_json in response {
        let val = serde_json::from_value::<Workspace>(workspace_json.clone()).expect("Unable to deserialize workspace");
        save_workspace_to_db(&val);
        workspace_vec.push(val);
    }
    return workspace_vec;
}