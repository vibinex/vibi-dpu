use reqwest::Response;
use serde_json::Value;

/// Deserializes a successful JSON HTTP response for provider API clients.
pub async fn deserialize_response(response: Response) -> Option<Value> {
    match response.json::<Value>().await {
        Ok(value) => Some(value),
        Err(error) => {
            log::error!("[deserialize_response] Unable to deserialize response: {error}");
            None
        }
    }
}
