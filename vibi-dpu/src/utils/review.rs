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

    // Public setter methods
    pub fn set_base_head_commit(&mut self, base_head_commit: String) {
        self.base_head_commit = base_head_commit;
    }

    pub fn set_pr_head_commit(&mut self, pr_head_commit: String) {
        self.pr_head_commit = pr_head_commit;
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_repo_name(&mut self, repo_name: String) {
        self.repo_name = repo_name;
    }

    pub fn set_repo_owner(&mut self, repo_owner: String) {
        self.repo_owner = repo_owner;
    }

    pub fn set_provider(&mut self, provider: String) {
        self.provider = provider;
    }

    pub fn set_db_key(&mut self, db_key: String) {
        self.db_key = db_key;
    }

    pub fn set_clone_dir(&mut self, clone_dir: String) {
        self.clone_dir = clone_dir;
    }

    pub fn set_clone_url(&mut self, clone_url: String) {
        self.clone_url = clone_url;
    }

    pub fn set_author(&mut self, author: String) {
        self.author = author;
    }
}
