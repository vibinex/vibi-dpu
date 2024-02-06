use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Review {
    base_head_commit: String,
    pr_head_commit: String,
    id: String,
    repo_name: String,
    repo_owner: String,
    provider: String,
    db_key: String,
    clone_dir: String,
    clone_url: String,
    author: String,
}

impl Review {
    // Constructor
    pub fn new(
        base_head_commit: String,
        pr_head_commit: String,
        id: String,
        repo_name: String,
        repo_owner: String,
        provider: String,
        db_key: String,
        clone_dir: String,
        clone_url: String,
        author: String,
    ) -> Self {
        Self {
            base_head_commit,
            pr_head_commit,
            id,
            repo_name,
            repo_owner,
            provider,
            db_key,
            clone_dir,
            clone_url,
            author,
        }
    }

    // Public getter methods
    pub fn base_head_commit(&self) -> &String {
        &self.base_head_commit
    }

    pub fn pr_head_commit(&self) -> &String {
        &self.pr_head_commit
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn repo_name(&self) -> &String {
        &self.repo_name
    }

    pub fn repo_owner(&self) -> &String {
        &self.repo_owner
    }

    pub fn provider(&self) -> &String {
        &self.provider
    }

    pub fn db_key(&self) -> &String {
        &self.db_key
    }

    pub fn clone_dir(&self) -> &String {
        &self.clone_dir
    }

    pub fn clone_url(&self) -> &String {
        &self.clone_url
    }

    pub fn author(&self) -> &String {
        &self.author
    }
}
