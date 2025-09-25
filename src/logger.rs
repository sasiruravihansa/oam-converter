use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub fn append_request_log(
    log_path: &Path,
    request_id: &str,
    external_id: &str,
    out_dir: &str,
    code: i32,
    message: &str,
) -> Result<(), std::io::Error> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let record = json!({
        "timestamp": timestamp,
        "request_id": request_id,
        "external_id": external_id,
        "out_dir": out_dir,
        "code": code,
        "message": message,
    });

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(log_path)?;

    writeln!(file, "{}", record.to_string())?;
    file.flush()
}
