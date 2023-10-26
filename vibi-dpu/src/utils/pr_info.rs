use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PrInfo {
    pub base_head_commit: String,
    pub pr_head_commit: String,
    pub state: String,
    pub pr_branch: String,
}

impl PrInfo {
    // Constructor
    pub fn new(base_head_commit: String, pr_head_commit: String, state: String, pr_branch: String) -> Self {
        Self {
            base_head_commit,
            pr_head_commit,
            state,
            pr_branch,
        }
    }

    // Public getter methods
    pub fn base_head_commit(&self) -> &String {
        &self.base_head_commit
    }

    pub fn pr_head_commit(&self) -> &String {
        &self.pr_head_commit
    }

    pub fn state(&self) -> &String {
        &self.state
    }

    pub fn pr_branch(&self) -> &String {
        &self.pr_branch
    }

    // Public setter methods
    pub fn set_base_head_commit(&mut self, new_base_head_commit: String) {
        self.base_head_commit = new_base_head_commit;
    }

    pub fn set_pr_head_commit(&mut self, new_pr_head_commit: String) {
        self.pr_head_commit = new_pr_head_commit;
    }

    pub fn set_state(&mut self, new_state: String) {
        self.state = new_state;
    }

    pub fn set_pr_branch(&mut self, new_pr_branch: String) {
        self.pr_branch = new_pr_branch;
    }
}
