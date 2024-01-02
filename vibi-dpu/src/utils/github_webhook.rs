use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Webhook {
    id: String,
    active: bool,
    created_at: String,
    events: Vec<String>,
    ping_url: String,
    url: String,
    config: HashMap<String, serde_json::Value>
}

impl Webhook {
    // Constructor
    pub fn new(
        id: String,
        active: bool,
        created_at: String,
        events: Vec<String>,
        ping_url: String,
        url: String,
        config: HashMap<String, serde_json::Value>
    ) -> Self {
        Self {
            id,
            active,
            created_at,
            events,
            ping_url,
            url,
            config
        }
    }

    // Public getter methods
    pub fn url(&self) -> &String {
        &self.url
    }
}