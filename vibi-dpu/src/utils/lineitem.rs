#[derive(Debug, Clone)]
pub struct LineItem {
    author_id: String,
    timestamp: String,
    commit: String,
}

impl LineItem {
    pub fn new(author_id: String, timestamp: String, commit: String) -> Self {
        Self {
            author_id,
            timestamp,
            commit
        }
    }

    pub fn author_id(&self) -> &String {
        &self.author_id
    }

    pub fn timestamp(&self) -> &String {
        &self.timestamp
    }

    pub fn commit(&self) -> &String {
        &self.commit
    }
}