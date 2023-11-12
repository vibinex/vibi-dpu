use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GithubAuthInfo {
    token: String,
    expires_at: String,
    installation_id: Option<String>
}

impl GithubAuthInfo {
    // Constructor
    pub fn new(token: String, expires_at: String, installation_id: Option<String>) -> Self {
        Self {
            token,
            expires_at,
            installation_id,
        }
    }

    // Public getter methods
    pub fn token(&self) -> &String {
        &self.token
    }

    pub fn expires_at(&self) -> &String {
        &self.expires_at
    }

    pub fn installation_id(&self) -> &Option<String> {
        &self.installation_id
    }

    pub fn set_installation_id(&mut self, installation_id: &str) {
        self.installation_id = Some(installation_id.to_string());
    }
}