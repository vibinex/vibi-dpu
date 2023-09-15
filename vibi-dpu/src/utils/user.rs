use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProviderEnum {
    Bitbucket,
    Github,
}

impl fmt::Display for ProviderEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ProviderEnum::Bitbucket => write!(f, "bitbucket"),
            ProviderEnum::Github => write!(f, "github"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    id: String,
    provider_type: ProviderEnum,
}

impl Provider {
    // Constructor
    pub fn new(id: String, provider_type: ProviderEnum) -> Self {
        Self { id, provider_type }
    }

    // Public getter methods
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn provider_type(&self) -> &ProviderEnum {
        &self.provider_type
    }

    // Public setter methods
    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_provider_type(&mut self, provider_type: ProviderEnum) {
        self.provider_type = provider_type;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    provider: Provider,
    name: String,
    workspace: String,
    aliases: Option<Vec<String>>,
}

impl User {
    // Constructor
    pub fn new(provider: Provider, name: String, workspace: String, aliases: Option<Vec<String>>) -> Self {
        Self {
            provider,
            name,
            workspace,
            aliases,
        }
    }

    // Public getter methods
    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn workspace(&self) -> &String {
        &self.workspace
    }

    pub fn aliases(&self) -> &Option<Vec<String>> {
        &self.aliases
    }

    pub fn set_aliases(&mut self, aliases: Option<Vec<String>>) {
        self.aliases = aliases;
    }
}
