use chrono::Utc;
use fern::log_file;
use log::LevelFilter;
use std::{env, time::Duration};

use crate::logger::cleanup::cleanup_old_logs;

pub fn init_logger() -> bool {
	let log_dir = "/var/log/dpu";
	// Create a directory for log files (change the path as needed)
	let create_dir_res = std::fs::create_dir_all(log_dir);
	if create_dir_res.is_err() {
		let e = create_dir_res.expect_err("Empty error in create_dir_res");
		log::error!("[init_logger] Unable to create logs dir: {:?}", e);
		return false;
	}

	// Set up fern to log messages to files with hourly rotation
	let log_file_path = format!("{}/{}.log", &log_dir, Utc::now().format("%Y-%m-%d_%H-%M-%S"));
	let file_config_res = log_file(&log_file_path);
	if file_config_res.is_err() {
		let e = file_config_res.expect_err("Empty error in file_config_res");
		log::error!("[init_logger] Unable to create log file: {:?}", e);
		return false;
	}
	let file_config = file_config_res.expect("Uncaught error in file_config_res");
	let log_level = env::var("LOG_LEVEL")
		.ok()
		.and_then(|s| s.parse().ok())
		.unwrap_or(LevelFilter::Debug);
	// Chain the file configuration with the stdout configuration
	let dispatcher_res = fern::Dispatch::new()
		.format(|out, message, record| {
			out.finish(format_args!(
				"[{}][{}] {}",
				Utc::now().format("%Y-%m-%d %H:%M:%S"),
				record.level(),
				message
			))
		})
		.level(log_level)
		.chain(std::io::stdout())
		.chain(file_config)
		.apply();
	if dispatcher_res.is_err() {
		let e = dispatcher_res.expect_err("Empty error inside dispatcher_res");
		log::error!("[init_logger] Unable to create logs file dispatcher: {:?}", e);
		return false;
	}
	let retention_days = 60;
	// Spawn a separate thread for log cleanup every 30 days
	tokio::spawn(async move {
		loop {
			// Perform log cleanup
			cleanup_old_logs(
				&log_dir, Duration::from_secs(retention_days * 24 * 60 * 60));
			// Sleep for retention_days before the next cleanup
			tokio::time::sleep(Duration::from_secs(retention_days * 24 * 60 * 60)).await;
		}
	});
	return true;
}