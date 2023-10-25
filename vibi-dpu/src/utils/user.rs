use std::fmt;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};

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


#[derive(Debug, Serialize, Clone, Eq, Hash, PartialEq)]
pub struct BitbucketUser {
    account_id: String,
    display_name: String,
    nickname: String,
    #[serde(rename = "type")]
    type_str: String,
    uuid: String,
}

impl<'de> Deserialize<'de> for BitbucketUser {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw: RawBitbucketUser = Deserialize::deserialize(deserializer)?;

        Ok(BitbucketUser {
            account_id: strip_quotes(&raw.account_id),
            display_name: strip_quotes(&raw.display_name),
            nickname: strip_quotes(&raw.nickname),
            type_str: strip_quotes(&raw.type_str),
            uuid: strip_quotes(&raw.uuid),
        })
    }
}

impl BitbucketUser {
    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn display_name(&self) -> &String {
        &self.display_name
    }

    pub fn nickname(&self) -> &String {
        &self.nickname
    }
}

#[derive(Deserialize)]
struct RawBitbucketUser {
    account_id: String,
    display_name: String,
    nickname: String,
    #[serde(rename = "type")]
    type_str: String,
    uuid: String,
}

fn strip_quotes(s: &str) -> String {
    s.trim_matches('"').to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceUser {
    display_name: String,
    users: HashSet<BitbucketUser>,
}

impl WorkspaceUser {
    pub fn new(display_name: String, users: HashSet<BitbucketUser>) -> Self {
        Self {
            display_name,
            users
        }
    }

    pub fn display_name(&self) -> &String {
        &self.display_name
    }

    pub fn users(&self) -> &HashSet<BitbucketUser> {
        &self.users
    }

    pub fn users_mut(&mut self) -> &mut HashSet<BitbucketUser> {
        &mut self.users
    }
}