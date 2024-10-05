use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoConfig {
    comment: bool,
    auto_assign: bool,
    diff_graph: bool
}

impl RepoConfig {
    // Getters
    pub fn comment(&self) -> bool {
        self.comment
    }

    pub fn auto_assign(&self) -> bool {
        self.auto_assign
    }

    pub fn diff_graph(&self) -> bool {
        self.diff_graph
    }

    // Function to create a default RepoConfig
    pub fn default() -> Self {
        RepoConfig {
            comment: true,
            auto_assign: true,
            diff_graph: false
        }
    }
}