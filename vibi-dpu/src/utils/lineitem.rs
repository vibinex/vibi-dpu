#[derive(Debug, Clone)]
pub struct LineItem {
    author_id: String,
    timestamp: String,
}

impl LineItem {
    pub fn new(author_id: String, timestamp: String) -> Self {
        Self {
            author_id,
            timestamp
        }
    }

    pub fn author_id(&self) -> &String {
        &self.author_id
    }

    pub fn timestamp(&self) -> &String {
        &self.timestamp
    }
}