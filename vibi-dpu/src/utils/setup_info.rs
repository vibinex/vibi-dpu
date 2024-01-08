use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]

pub struct SetupInfo {
    pub provider: String,
    pub owner: String,
    pub repos: Vec<String>,
}