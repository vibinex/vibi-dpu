use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GithubAuthInfo {
    access_token: String,
    installation_id: String,
    expires_at: u64,
    timestamp: Option<u64>,
}

impl GithubAuthInfo {
    // Constructor
    pub fn new(access_token: String, installation_id: String, expires_at: u64, timestamp: Option<u64>) -> Self {
        Self {
            access_token,
            installation_id,
            expires_at,
            timestamp,
        }
    }

    // Public getter methods
    pub fn access_token(&self) -> &String {
        &self.access_token
    }

    pub fn installation_id(&self) -> &String {
        &self.installation_id
    }

    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    pub fn timestamp(&self) -> &Option<u64> {
        &self.timestamp
    }

    // Public setters
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = Some(timestamp);
    }
}