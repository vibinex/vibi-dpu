use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PrInfo {
    pub base_head_commit: String,
    pub pr_head_commit: String,
    pub state: String,
    pub pr_branch: String,
    pub author: Option<String>,
}

impl PrInfo {

    // Public getter methods
    pub fn pr_head_commit(&self) -> &String {
        &self.pr_head_commit
    }
}
