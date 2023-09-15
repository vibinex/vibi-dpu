use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    name: String,
    uuid: String,
    slug: String,
}

impl Workspace {
    // Constructor
    pub fn new(name: String, uuid: String, slug: String) -> Self {
        Self {
            name,
            uuid,
            slug,
        }
    }

    // Public getter methods
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn slug(&self) -> &String {
        &self.slug
    }
}
