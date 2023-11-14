use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoConfig {
    comment: bool,
    auto_assign: bool
}

impl RepoConfig {
    // Getters
    pub fn comment(&self) -> bool {
        self.comment
    }

    pub fn auto_assign(&self) -> bool {
        self.auto_assign
    }

    pub fn set_auto_assign(&mut self, auto_assign: bool) {
        self.auto_assign = auto_assign;
    }

    // Function to create a default RepoConfig
    pub fn default() -> Self {
        RepoConfig {
            comment: true,
            auto_assign: true
        }
    }
}