use std::fs;
use std::time::{Duration, SystemTime};

pub fn cleanup_old_logs(logs_dir: &str, retention_period: Duration) {
    let entries_res = fs::read_dir(logs_dir);
    if entries_res.is_err() {
        let e = entries_res.expect_err("Empty error in entries_res");
        log::error!("[cleanup_old_logs] Unable to read logs dir: {:?}", e);
        return;
    }
    let entries = entries_res.expect("Uncaught error in entries_res");
    let current_time = SystemTime::now();

    for entry_res in entries {
        if entry_res.is_err() {
            let e = entry_res.expect_err("Empty error inside entry_res");
            log::error!("[cleanup_old_logs] Error in getting dir entry: {:?}", e);
            continue;
        }
        let entry = entry_res.expect("Uncaught error in entry_res");
        let metadata_res = entry.metadata();
        if metadata_res.is_err() {
            let e = metadata_res.expect_err("Empty error in metadata_res");
            log::error!("[cleanup_old_logs] Unable to get metadata for log file: {:?}", e);
            continue;
        }
        let metadata = metadata_res.expect("Uncaught error in metadata_res");
        let modified_time_res = metadata.modified();
        if modified_time_res.is_err() {
            let e = modified_time_res.expect_err("Empty error in modified_time_res");
            log::error!("[cleanup_old_logs] Unable to last modified time of log file: {:?}", e);
            continue;
        }
        let modified_time = modified_time_res.expect("Uncaught error in modified_time_res");
        let elapsed_res  = current_time.duration_since(modified_time);
        if elapsed_res.is_err() {
            let e = elapsed_res.expect_err("Empty error in elapsed_res");
            log::error!("[cleanup_old_logs] Unable to get current time: {:?}", e);
            continue;
        }
        let elapsed = elapsed_res.expect("Uncaught error in elapsed_res");
        // Check if the file is older than the retention period
        if elapsed > retention_period {
            let remove_file_res = fs::remove_file(entry.path());
            if remove_file_res.is_err() {
                let e = remove_file_res.expect_err("Empty error in remove_file_res");
                log::error!("[cleanup_old_logs] Unable to remove old log file: {:?}", e);
            }
        }
    }
}
