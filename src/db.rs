use deadpool_postgres::{Config, Pool, Runtime};
use std::str::FromStr;
use tokio_postgres::NoTls;

// Basic representation of a request to be stored in the database
#[allow(dead_code)]
pub struct OamRequest {
    pub id: i32,
    pub external_id: String,
    pub storage_path: String,
}

pub fn create_pool(database_url: &str) -> Pool {
    let pg_config = tokio_postgres::Config::from_str(database_url)
        .expect("Failed to parse database URL");

    let cfg = Config {
        user: pg_config.get_user().map(String::from),
        password: pg_config.get_password().map(|p| String::from_utf8_lossy(p).to_string()),
        host: pg_config.get_hosts().get(0).map(|h| match h {
            tokio_postgres::config::Host::Tcp(s) => s.clone(),
            _ => "localhost".to_string(), // Or handle other host types
        }),
        port: pg_config.get_ports().get(0).cloned(),
        dbname: pg_config.get_dbname().map(String::from),
        ..Default::default()
    };

    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create database pool")
}

// Placeholder function to save a request
#[allow(dead_code)]
pub async fn save_request(pool: &Pool, external_id: &str, storage_path: &str, code: i32, response: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;
    client.execute(
        "INSERT INTO oam_requests (external_id, storage_path, code, response) VALUES ($1, $2, $3, $4)",
        &[&external_id, &storage_path, &code, &response],
    ).await?;
    Ok(())
}

// Placeholder function to get a request by external_id
#[allow(dead_code)]
pub async fn get_request_by_external_id(pool: &Pool, external_id: &str) -> Option<OamRequest> {
    let client = pool.get().await.ok()?;
    let row = client.query_one(
        "SELECT id, external_id, storage_path FROM oam_requests WHERE external_id = $1 ORDER BY created_at DESC LIMIT 1",
        &[&external_id],
    ).await.ok()?;
    Some(OamRequest {
        id: row.get(0),
        external_id: row.get(1),
        storage_path: row.get(2),
    })
}