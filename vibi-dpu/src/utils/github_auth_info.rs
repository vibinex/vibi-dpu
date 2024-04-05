use std::fs::File;
use std::io::Write;
use std::io::Read;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GithubAuthInfo {
	token: String,
	expires_at: String,
	installation_id: Option<String>
}

impl GithubAuthInfo {

	// Public getter methods
	pub fn token(&self) -> &String {
		&self.token
	}

	pub fn expires_at(&self) -> &String {
		&self.expires_at
	}

	pub fn installation_id(&self) -> &Option<String> {
		&self.installation_id
	}

	pub fn set_installation_id(&mut self, installation_id: &str) {
		self.installation_id = Some(installation_id.to_string());
	}

	pub fn load_from_file() -> Option<Self> {
		log::debug!("[github_auth_info/load_from_file] Loading github auth info from: {}", &PATH);
		let file_res = File::open(&PATH);
		if let Err(err) = file_res {
			log::error!("[github_auth_info/load_from_file] Unable to open file: {:?}", &err);
			return None;
		}
		let mut file = file_res.expect("Uncaught error in file_res");
		let mut contents = String::new();
		let read_res = file.read_to_string(&mut contents);
		if let Err(err) = read_res {
			log::error!("[github_auth_info/load_from_file] Unable to read from file: {:?}", &err);
			return None;
		}
		let auth_info_res = serde_json::from_str(&contents);
		if let Err(err) = auth_info_res {
			log::error!("[github_auth_info/load_from_file] Unable to parse file contents: {:?}", &err);
			return None;            
		}
		let auth_info: GithubAuthInfo = auth_info_res.expect("Uncaught error in auth_info_res");
		Some(auth_info)
	}

	pub fn save_to_file(&self) {
		log::debug!("[github_auth_info/save_to_file] Saving auth info to file: {}", &PATH);
		let json_str_res = serde_json::to_string(self);
		if let Err(err) = json_str_res {
			log::error!("[github_auth_info/save_to_file] Unable to convert auth info to string: {:?}", err);
			return;
		}
		let json_str = json_str_res.expect("Uncaught error in json_str_res");
		let file_res = File::create(&PATH);
		if let Err(err) = file_res {
			log::error!("[github_auth_info/save_to_file] Unable to create/open file: {:?}", err);
			return;
		}
		let mut file = file_res.expect("Uncaught error in file_res");
		let write_res = file.write_all(json_str.as_bytes());
		if let Err(err) = write_res {
			log::error!("[github_auth_info/save_to_file] Unable to write to file: {:?}", err);
		}
	}
}

static PATH: &str = "/app/config/dpu_creds.json";
//docker run -e INSTALL_ID=topic-name -v ~/.config/vibinex:/app/config dpu:local
