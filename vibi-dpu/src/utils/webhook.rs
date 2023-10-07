use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Webhook {
    uuid: String,
    active: bool,
    created_at: String,
    events: Vec<String>,
    ping_url: String,
    url: String,
}

impl Webhook {
    // Constructor
    pub fn new(
        uuid: String,
        active: bool,
        created_at: String,
        events: Vec<String>,
        ping_url: String,
        url: String,
    ) -> Self {
        Self {
            uuid,
            active,
            created_at,
            events,
            ping_url,
            url,
        }
    }

    // Public getter methods
    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn created_at(&self) -> &String {
        &self.created_at
    }

    pub fn events(&self) -> &Vec<String> {
        &self.events
    }

    pub fn ping_url(&self) -> &String {
        &self.ping_url
    }

    pub fn url(&self) -> &String {
        &self.url
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookResponse {
    uuid: String,
    active: bool,
    url: String,
    created_at: String,
    events: Vec<String>,
    links: HashMap<String, HashMap<String, String>>,
}

impl WebhookResponse {
    // Constructor
    pub fn new(
        uuid: String,
        active: bool,
        url: String,
        created_at: String,
        events: Vec<String>,
        links: HashMap<String, HashMap<String, String>>,
    ) -> Self {
        Self {
            uuid,
            active,
            url,
            created_at,
            events,
            links,
        }
    }

    // Public getter methods
    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn url(&self) -> &String {
        &self.url
    }

    pub fn created_at(&self) -> &String {
        &self.created_at
    }

    pub fn events(&self) -> &Vec<String> {
        &self.events
    }

    pub fn links(&self) -> &HashMap<String, HashMap<String, String>> {
        &self.links
    }
}
