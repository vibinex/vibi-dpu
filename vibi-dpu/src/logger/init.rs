use env_logger::Env;
use chrono::Utc;
use fern::log_file;
use std::time::Duration;

use crate::logger::cleanup::cleanup_old_logs;

pub fn init_logger() -> bool {
    let log_file_path = "/tmp/logs"; // TODO - decide optimal directory
    // Set up env_logger to log messages to stdout
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    // Create a directory for log files (change the path as needed)
    let create_dir_res = std::fs::create_dir_all(log_file_path);
    if create_dir_res.is_err() {
        let e = create_dir_res.expect_err("Empty error in create_dir_res");
        log::error!("[init_logger] Unable to create logs dir: {:?}", e);
        return false;
    }

    // Set up fern to log messages to files with hourly rotation
    let log_file_path = format!("logs/{}.log", Utc::now().format("%Y-%m-%d_%H-%M-%S"));
    let file_config_res = log_file(&log_file_path);
    if file_config_res.is_err() {
        let e = file_config_res.expect_err("Empty error in file_config_res");
        log::error!("[init_logger] Unable to create log file: {:?}", e);
        // TODO - log error message
        return false;
    }
    let file_config = file_config_res.expect("Uncaught error in file_config_res");
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
                &log_file_path, Duration::from_secs(retention_days * 24 * 60 * 60));
            // Sleep for retention_days before the next cleanup
            tokio::time::sleep(Duration::from_secs(retention_days * 24 * 60 * 60)).await;
        }
    });
    return true;
}