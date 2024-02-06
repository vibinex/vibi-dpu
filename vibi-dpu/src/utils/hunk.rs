#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HunkMap {
    repo_provider: String,
    repo_owner: String,
    repo_name: String,
    prhunkvec: Vec<PrHunkItem>,
    db_key: String,
}


use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrHunkItem {
    pr_number: String,
    author: String,
    blamevec: Vec<BlameItem>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlameItem {
    author: String,
    timestamp: String,
    line_start: String,
    line_end: String,
    filepath: String,
    #[serde(skip_serializing)]
    commit: String, // This variable will be ignored during serialization
    #[serde(skip_serializing)]
    filepath_raw: String, // This variable will be ignored during serialization
}

impl BlameItem {
    // Constructor
    pub fn new(
        author: String,
        timestamp: String,
        line_start: String,
        line_end: String,
        filepath: String,
        commit: String,
        filepath_raw: String,
    ) -> Self {
        Self {
            author,
            timestamp,
            line_start,
            line_end,
            filepath,
            commit,
            filepath_raw,
        }
    }

    // Public getter methods
    pub fn author(&self) -> &String {
        &self.author
    }

    pub fn line_start(&self) -> &String {
        &self.line_start
    }

    pub fn line_end(&self) -> &String {
        &self.line_end
    }

    pub fn commit(&self) -> &String {
        &self.commit
    }

    pub fn filepath_raw(&self) -> &String {
        &self.filepath_raw
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
    pub fn author(&self) -> &String {
        &self.author
    }

    pub fn blamevec(&self) -> &Vec<BlameItem> {
        &self.blamevec
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
}
