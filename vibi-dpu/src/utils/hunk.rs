#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct HunkMap {
    repo_provider: String,
    repo_owner: String,
    repo_name: String,
    prhunkvec: Vec<PrHunkItem>,
    db_key: String,
}


use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct PrHunkItem {
    pr_number: String,
    author: String,
    blamevec: Vec<BlameItem>,
}


#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct BlameItem {
    author: String,
    timestamp: String,
    line_start: String,
    line_end: String,
    filepath: String,
}

impl BlameItem {
    // Constructor
    pub fn new(
        author: String,
        timestamp: String,
        line_start: String,
        line_end: String,
        filepath: String,
    ) -> Self {
        Self {
            author,
            timestamp,
            line_start,
            line_end,
            filepath,
        }
    }

    // Public getter methods
    pub fn author(&self) -> &String {
        &self.author
    }

    pub fn timestamp(&self) -> &String {
        &self.timestamp
    }

    pub fn line_start(&self) -> &String {
        &self.line_start
    }

    pub fn line_end(&self) -> &String {
        &self.line_end
    }

    pub fn filepath(&self) -> &String {
        &self.filepath
    }

    // Public setter methods
    pub fn set_author(&mut self, author: String) {
        self.author = author;
    }

    pub fn set_timestamp(&mut self, timestamp: String) {
        self.timestamp = timestamp;
    }

    pub fn set_line_start(&mut self, line_start: String) {
        self.line_start = line_start;
    }

    pub fn set_line_end(&mut self, line_end: String) {
        self.line_end = line_end;
    }

    pub fn set_filepath(&mut self, filepath: String) {
        self.filepath = filepath;
    }
}

impl PrHunkItem {
    // Constructor
    pub fn new(pr_number: String, author: String, blamevec: Vec<BlameItem>) -> Self {
        Self {
            pr_number,
            author,
            blamevec,
        }
    }

    // Public getter methods
    pub fn pr_number(&self) -> &String {
        &self.pr_number
    }

    pub fn author(&self) -> &String {
        &self.author
    }

    pub fn blamevec(&self) -> &Vec<BlameItem> {
        &self.blamevec
    }

    // Public setter methods
    pub fn set_pr_number(&mut self, pr_number: String) {
        self.pr_number = pr_number;
    }

    pub fn set_author(&mut self, author: String) {
        self.author = author;
    }

    pub fn set_blamevec(&mut self, blamevec: Vec<BlameItem>) {
        self.blamevec = blamevec;
    }
}


impl HunkMap {
    // Constructor
    pub fn new(
        repo_provider: String,
        repo_owner: String,
        repo_name: String,
        prhunkvec: Vec<PrHunkItem>,
        db_key: String,
    ) -> Self {
        Self {
            repo_provider,
            repo_owner,
            repo_name,
            prhunkvec,
            db_key,
        }
    }

    // Public getter methods
    pub fn repo_provider(&self) -> &String {
        &self.repo_provider
    }

    pub fn repo_owner(&self) -> &String {
        &self.repo_owner
    }

    pub fn repo_name(&self) -> &String {
        &self.repo_name
    }

    pub fn prhunkvec(&self) -> &Vec<PrHunkItem> {
        &self.prhunkvec
    }

    pub fn db_key(&self) -> &String {
        &self.db_key
    }

    // Public setter methods
    pub fn set_repo_provider(&mut self, repo_provider: String) {
        self.repo_provider = repo_provider;
    }

    pub fn set_repo_owner(&mut self, repo_owner: String) {
        self.repo_owner = repo_owner;
    }

    pub fn set_repo_name(&mut self, repo_name: String) {
        self.repo_name = repo_name;
    }

    pub fn set_prhunkvec(&mut self, prhunkvec: Vec<PrHunkItem>) {
        self.prhunkvec = prhunkvec;
    }

    pub fn set_db_key(&mut self, db_key: String) {
        self.db_key = db_key;
    }
}
