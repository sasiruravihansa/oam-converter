use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::Deserialize;
use reqwest::Client;
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;

mod config;
mod db;
mod llm;
mod storage;
mod logger;

use config::Config;
use storage::{Storage, GcsStorage, S3Storage, AzureStorage};
use serde_json::json;

#[derive(Deserialize)]
struct GenerateRequest {
    oam_url: String,
    external_id: String,
    provider: String,
    tool: String,
}

struct AppState {
    http_client: Client,
    db_pool: deadpool_postgres::Pool,
    storage_client: Arc<dyn Storage>,
}

async fn generate_iac(
    state: web::Data<AppState>,
    req: web::Json<GenerateRequest>,
) -> impl Responder {
    let temp_dir_path = std::env::temp_dir().join(&req.external_id);
    if let Err(e) = fs::create_dir_all(&temp_dir_path) {
        return HttpResponse::InternalServerError().body(format!("Failed to create temp directory: {}", e));
    }
    let log_file_path = temp_dir_path.join("request-log.txt");

    logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Fetching OAM file...").ok();
    let oam_yaml = match state.http_client.get(&req.oam_url).send().await {
        Ok(res) => match res.text().await {
            Ok(text) => {
                logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Successfully fetched OAM file.").ok();
                text
            },
            Err(e) => {
                logger::append_request_log(&log_file_path, "", &req.external_id, "", 1, &format!("Failed to read OAM file content: {}", e)).ok();
                return HttpResponse::InternalServerError().body(format!("Failed to read OAM file content: {}", e));
            }
        },
        Err(e) => {
            logger::append_request_log(&log_file_path, "", &req.external_id, "", 1, &format!("Failed to fetch OAM file URL: {}", e)).ok();
            return HttpResponse::InternalServerError().body(format!("Failed to fetch OAM file URL: {}", e));
        }
    };

    let prompt = llm::build_prompt(&oam_yaml, &req.provider, &req.tool);

    logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Generating files from LLM...").ok();
    let generated_files = match llm::generate_files(&state.http_client, &prompt).await {
        Ok(files) => {
            logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Successfully generated files from LLM.").ok();
            files
        },
        Err(e) => {
            logger::append_request_log(&log_file_path, "", &req.external_id, "", 1, &format!("Failed to generate files from LLM: {}", e)).ok();
            return HttpResponse::InternalServerError().body(format!("Failed to generate files from LLM: {}", e));
        }
    };

    for (file_path, content) in &generated_files.files {
        let path = temp_dir_path.join(file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Zipping directory...").ok();
    let zip_file_path = std::env::temp_dir().join(format!("{}.zip", req.external_id));
    if let Err(e) = storage::zip_directory(&temp_dir_path, &zip_file_path) {
        logger::append_request_log(&log_file_path, "", &req.external_id, "", 1, &format!("Failed to zip directory: {}", e)).ok();
        return HttpResponse::InternalServerError().body(format!("Failed to zip directory: {}", e));
    }
    logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Successfully zipped directory.").ok();

    logger::append_request_log(&log_file_path, "", &req.external_id, "", 0, "Uploading to storage...").ok();
    let object_key = format!("{}/{}.zip", req.external_id, chrono::Utc::now().to_rfc3339());
    let storage_path = match state.storage_client.upload(&zip_file_path, &object_key).await {
        Ok(path) => {
            logger::append_request_log(&log_file_path, "", &req.external_id, &path, 0, &format!("Successfully uploaded to {}", path)).ok();
            path
        },
        Err(e) => {
            logger::append_request_log(&log_file_path, "", &req.external_id, "", 1, &format!("Failed to upload to storage: {}", e)).ok();
            return HttpResponse::InternalServerError().body(format!("Failed to upload to storage: {}", e));
        }
    };

    let response_message = format!("Successfully generated and uploaded IaC to {}", storage_path);
    if let Err(e) = db::save_request(&state.db_pool, &req.external_id, &storage_path, 0, &response_message).await {
        eprintln!("Failed to save request to database: {}", e);
    }
    
    // 7. Clean up temporary files and directory
    fs::remove_dir_all(&temp_dir_path).ok();
    fs::remove_file(&zip_file_path).ok();

    // 8. Respond to the user
    let deploy_script = generated_files.files.get("deploy.sh").cloned().unwrap_or_default();
    let response_body = json!({
        "message": response_message,
        "deploy_script": deploy_script,
    });

    HttpResponse::Ok().json(response_body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let config = Config::from_env().expect("Failed to load configuration");
    let db_pool = db::create_pool(&config.database_url);

    let storage_client: Arc<dyn Storage> = match config.storage_provider.as_str() {
        "gcs" => {
            let bucket = config.gcs_bucket.clone().unwrap();
            let gcs_storage = GcsStorage::new(bucket).await.expect("Failed to create GCS client");
            Arc::new(gcs_storage)
        },
        "s3" => Arc::new(S3Storage),
        "azure" => Arc::new(AzureStorage),
        _ => panic!("Unsupported storage provider"),
    };

    let app_state = web::Data::new(AppState {
        http_client: Client::new(),
        db_pool,
        storage_client,
    });

    println!("Server starting at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/generate", web::post().to(generate_iac))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}