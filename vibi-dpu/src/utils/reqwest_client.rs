use std::sync::Arc;
use once_cell::sync::Lazy;
use reqwest::Client;

static CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    Arc::new(Client::new())
});

pub fn get_client() -> Arc<Client> {
    Arc::clone(&CLIENT)
}