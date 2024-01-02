use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    name: String,
    uuid: String,
    slug: String,
}

impl Workspace {

    // Public getter methods
    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn slug(&self) -> &String {
        &self.slug
    }
}
