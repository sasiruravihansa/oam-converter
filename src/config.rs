#[derive(Clone)]
#[allow(dead_code)]
pub struct Config {
    pub database_url: String,
    pub storage_provider: String,
    pub gcs_bucket: Option<String>,
    pub aws_s3_bucket: Option<String>,
    pub azure_blob_container: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set".to_string())?;
        
        let storage_provider = std::env::var("STORAGE_PROVIDER")
            .unwrap_or_else(|_| "gcs".to_string());

        let gcs_bucket = std::env::var("GCS_BUCKET").ok();
        let aws_s3_bucket = std::env::var("AWS_S3_BUCKET").ok();
        let azure_blob_container = std::env::var("AZURE_BLOB_CONTAINER").ok();

        match storage_provider.as_str() {
            "gcs" if gcs_bucket.is_none() => return Err("STORAGE_PROVIDER is 'gcs' but GCS_BUCKET is not set".to_string()),
            "s3" if aws_s3_bucket.is_none() => return Err("STORAGE_PROVIDER is 's3' but AWS_S3_BUCKET is not set".to_string()),
            "azure" if azure_blob_container.is_none() => return Err("STORAGE_PROVIDER is 'azure' but AZURE_BLOB_CONTAINER is not set".to_string()),
            _ => ()
        };

        Ok(Config {
            database_url,
            storage_provider,
            gcs_bucket,
            aws_s3_bucket,
            azure_blob_container,
        })
    }
}