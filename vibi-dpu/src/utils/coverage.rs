use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Coverage {
    provider: String,
    git_alias: String,
    coverage_str: String,
    coverage_num: f32,
    handles: Option<Vec<String>>,
}

impl Coverage {
    // Constructor
    pub fn new(
        provider: String,
        git_alias: String,
        coverage_str: String,
        coverage_num: f32,
        handles: Option<Vec<String>>,
    ) -> Self {
        Self {
            provider,
            git_alias,
            coverage_str,
            coverage_num,
            handles,
        }
    }

    // Public getter methods
    pub fn provider(&self) -> &String {
        &self.provider
    }

    pub fn git_alias(&self) -> &String {
        &self.git_alias
    }

    pub fn coverage_str(&self) -> &String {
        &self.coverage_str
    }

    pub fn coverage_num(&self) -> f32 {
        self.coverage_num
    }

    pub fn handles(&self) -> &Option<Vec<String>> {
        &self.handles
    }
}
