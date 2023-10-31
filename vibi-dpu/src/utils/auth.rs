use serde::Deserialize;
use serde::Serialize;

use super::user::BitbucketUser;

pub trait AuthInfo {
    fn access_token(&self) -> &str;
    fn timestamp(&self) -> Option<u64>;
    fn set_timestamp(&mut self, timestamp: u64);
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BitbucketAuthInfo {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    timestamp: Option<u64>,
}

impl BitbucketAuthInfo{
    pub fn new(access_token: String, refresh_token: String, timestamp: Option<u64>, expires_in: u64) -> Self {
        Self{
            access_token,
            refresh_token,
            expires_in,
            timestamp,
        }
    }

    pub fn refresh_token(&self) -> &String {
        &self.refresh_token
    }

    pub fn expires_in(&self) -> &u64 {
        &self.expires_in
    }
}

impl AuthInfo for BitbucketAuthInfo {
    // Public getter methods
    fn access_token(&self) -> &String {
        &self.access_token
    }

    fn timestamp(&self) -> &Option<u64> {
        &self.timestamp
    }

    // Public setters
    fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = Some(timestamp);
    }
}

pub struct GithubAuthInfo {
    access_token: String,
    install_id: String,
    expires_at: u64,
    timestamp: Option<u64>,
}

impl GithubAuthInfo {
    pub fn new(access_token: String, install_id: String, expires_at: u64, timestamp: Option<u64>) -> Self {
        Self {
            access_token,
            install_id,
            expires_at,
            timestamp
        }
    }

    pub fn install_id(&self) -> &String {
        &self.install_id
    }

    pub fn expires_at(&self) -> &u64 {
        &self.expires_at
    }
}

impl AuthInfo for GithubAuthInfo {

    //getter methods
    fn access_token(&self) -> &String {
        &self.access_token
    }

    fn timestamp(&self) -> &Option<u64> {
        &self.timestamp
    }
}