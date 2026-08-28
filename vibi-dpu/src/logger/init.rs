use chrono::Utc;
use fern::log_file;
use log::LevelFilter;
use std::{env, ffi::OsString, path::PathBuf, time::Duration};

use crate::logger::cleanup::cleanup_old_logs;

const DEFAULT_LOG_DIR: &str = "/var/log/dpu";
const LOG_DIR_ENV: &str = "DPU_LOG_DIR";

fn configured_log_dir(value: Option<OsString>) -> PathBuf {
    value
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LOG_DIR))
}

pub fn init_logger() -> bool {
    let configured_log_dir = configured_log_dir(env::var_os(LOG_DIR_ENV));
    let log_dir = match std::fs::create_dir_all(&configured_log_dir) {
        Ok(()) => configured_log_dir,
        Err(error) => {
            let fallback_log_dir = env::temp_dir().join("vibinex-dpu-logs");
            eprintln!(
				"[init_logger] Unable to create configured log directory {}: {error:?}; falling back to {}",
				configured_log_dir.display(),
				fallback_log_dir.display(),
			);
            if let Err(fallback_error) = std::fs::create_dir_all(&fallback_log_dir) {
                eprintln!(
                    "[init_logger] Unable to create fallback log directory {}: {fallback_error:?}",
                    fallback_log_dir.display()
                );
                return false;
            }
            fallback_log_dir
        }
    };

    // Set up fern to log messages to files with hourly rotation
    let log_file_path = log_dir.join(format!("{}.log", Utc::now().format("%Y-%m-%d_%H-%M-%S")));
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
        .unwrap_or(LevelFilter::Info);
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
        log::error!(
            "[init_logger] Unable to create logs file dispatcher: {:?}",
            e
        );
        return false;
    }
    let retention_days = 60;
    // Spawn a separate thread for log cleanup every 30 days
    tokio::spawn(async move {
        loop {
            // Perform log cleanup
            cleanup_old_logs(
                log_dir.to_string_lossy().as_ref(),
                Duration::from_secs(retention_days * 24 * 60 * 60),
            );
            // Sleep for retention_days before the next cleanup
            tokio::time::sleep(Duration::from_secs(retention_days * 24 * 60 * 60)).await;
        }
    });
    return true;
}

#[cfg(test)]
mod tests {
    use super::{configured_log_dir, DEFAULT_LOG_DIR};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn uses_the_legacy_directory_when_no_override_is_set() {
        assert_eq!(configured_log_dir(None), PathBuf::from(DEFAULT_LOG_DIR));
    }

    #[test]
    fn uses_a_non_empty_log_directory_override() {
        assert_eq!(
            configured_log_dir(Some(OsString::from("/tmp/dpu-logs"))),
            PathBuf::from("/tmp/dpu-logs")
        );
    }
}
