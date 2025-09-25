use async_trait::async_trait;
use std::path::Path;
use std::fs::{File};
use std::io::{Write, Read};
use zip::write::{FileOptions, ZipWriter};
use walkdir::WalkDir;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};

#[async_trait]
pub trait Storage: Send + Sync {
    async fn upload(&self, file_path: &Path, object_key: &str) -> Result<String, String>;
}

// --- Google Cloud Storage Implementation ---
pub struct GcsStorage {
    client: Client,
    bucket: String,
}

impl GcsStorage {
    pub async fn new(bucket: String) -> Result<Self, String> {
        let config = ClientConfig::default().with_auth().await.map_err(|e| e.to_string())?;
        let client = Client::new(config);
        Ok(Self { client, bucket })
    }
}

#[async_trait]
impl Storage for GcsStorage {
    async fn upload(&self, file_path: &Path, object_key: &str) -> Result<String, String> {
        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let mut vec = Vec::new();
        file.read_to_end(&mut vec).map_err(|e| e.to_string())?;

        let upload_type = UploadType::Simple(Media {
            name: object_key.to_owned().into(),
            content_type: "application/zip".into(),
            content_length: Some(vec.len() as u64),
        });

        self.client.upload_object(&UploadObjectRequest {
            bucket: self.bucket.clone(),
            ..Default::default()
        }, vec, &upload_type).await.map_err(|e| e.to_string())?;

        Ok(format!("gs://{}/{}", self.bucket, object_key))
    }
}

// --- AWS S3 Placeholder ---
pub struct S3Storage;

#[async_trait]
impl Storage for S3Storage {
    async fn upload(&self, _file_path: &Path, _object_key: &str) -> Result<String, String> {
        unimplemented!("AWS S3 storage is not yet implemented");
    }
}

// --- Azure Blob Storage Placeholder ---
pub struct AzureStorage;

#[async_trait]
impl Storage for AzureStorage {
    async fn upload(&self, _file_path: &Path, _object_key: &str) -> Result<String, String> {
        unimplemented!("Azure Blob storage is not yet implemented");
    }
}


/// Zips the contents of a directory and returns the path to the zip file.
pub fn zip_directory(dir_path: &Path, zip_file_path: &Path) -> Result<(), String> {
    if !dir_path.is_dir() {
        return Err(format!("Source path is not a directory: {:?}", dir_path));
    }

    let file = File::create(&zip_file_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o755);

    let walkdir = WalkDir::new(dir_path);
    let it = walkdir.into_iter().filter_map(|e| e.ok());

    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(dir_path).map_err(|e| e.to_string())?;

        if path.is_file() {
            zip.start_file(name.to_str().ok_or_else(|| "Invalid file name".to_string())?, options).map_err(|e| e.to_string())?;
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
            zip.write_all(&buffer).map_err(|e| e.to_string())?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name.to_str().ok_or_else(|| "Invalid directory name".to_string())?, options).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
