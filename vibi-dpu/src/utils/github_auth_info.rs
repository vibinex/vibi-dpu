use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GithubAuthInfo {
    token: String,
    installation_id: String,
    expires_at: String,
    timestamp: Option<u64>,
}

impl GithubAuthInfo {
    // Constructor
    pub fn new(token: String, installation_id: String, expires_at: String, timestamp: Option<u64>) -> Self {
        Self {
            token,
            installation_id,
            expires_at,
            timestamp,
        }
    }

    // Public getter methods
    pub fn token(&self) -> &String {
        &self.token
    }

    pub fn installation_id(&self) -> &String {
        &self.installation_id
    }

    pub fn expires_at(&self) -> &String {
        &self.expires_at
    }

    pub fn timestamp(&self) -> &Option<u64> {
        &self.timestamp
    }

    // Public setters
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = Some(timestamp);
    }
}