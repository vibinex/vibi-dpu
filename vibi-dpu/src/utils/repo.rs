use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repository {
    name: String,
    uuid: String,
    owner: String,
    is_private: bool,
    clone_ssh_url: String,
    project: String,
    workspace: String,
    local_dir: Option<String>,
    provider: String,
}

impl Repository {
    // Constructor
    pub fn new(
        name: String,
        uuid: String,
        owner: String,
        is_private: bool,
        clone_ssh_url: String,
        project: String,
        workspace: String,
        local_dir: Option<String>,
        provider: String,
    ) -> Self {
        Self {
            name,
            uuid,
            owner,
            is_private,
            clone_ssh_url,
            project,
            workspace,
            local_dir,
            provider,
        }
    }

    // Public getter methods
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn owner(&self) -> &String {
        &self.owner
    }

    pub fn is_private(&self) -> bool {
        self.is_private
    }

    pub fn clone_ssh_url(&self) -> &String {
        &self.clone_ssh_url
    }

    pub fn project(&self) -> &String {
        &self.project
    }

    pub fn workspace(&self) -> &String {
        &self.workspace
    }

    pub fn local_dir(&self) -> &Option<String> {
        &self.local_dir
    }

    pub fn provider(&self) -> &String {
        &self.provider
    }

    //Public Setters
    pub fn set_local_dir(&mut self, local_dir: String) {
        self.local_dir = Some(local_dir);
    }
}
