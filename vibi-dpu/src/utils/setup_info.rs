use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]

pub struct SetupInfo {
    pub provider: String,
    pub owner: String,
    pub repos: Vec<String>,
}

impl SetupInfo {
    pub fn new(provider: String, owner: String, repos: Vec<String>) -> Self {
		Self {
			provider,
			owner,
			repos,
		}
	}

    // Public getter methods
	pub fn provider(&self) -> &String {
		&self.provider
	}

	pub fn owner(&self) -> &String {
		&self.owner
	}

	pub fn repos(&self) -> &Vec<String> {
		&self.repos
	}

    // Public setter methods
    pub fn set_provider(&mut self, provider: String) {
        self.provider = provider;
    }

}