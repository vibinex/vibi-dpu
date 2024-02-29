use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Relevance {
    provider: String,
    git_alias: String,
    relevance_str: String,
    relevance_num: f32,
    handles: Option<Vec<String>>,
}

impl Relevance {
    // Constructor
    pub fn new(
        provider: String,
        git_alias: String,
        relevance_str: String,
        relevance_num: f32,
        handles: Option<Vec<String>>,
    ) -> Self {
        Self {
            provider,
            git_alias,
            relevance_str,
            relevance_num,
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

    pub fn relevance_str(&self) -> &String {
        &self.relevance_str
    }

    pub fn relevance_num(&self) -> f32 {
        self.relevance_num
    }

    pub fn handles(&self) -> &Option<Vec<String>> {
        &self.handles
    }
}
