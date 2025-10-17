use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, warn};

#[cfg(feature = "s3")]
use aws_sdk_s3::{Client, primitives::ByteStream};

/// S3 uploader for protobuf files and index
pub struct S3Uploader {
    #[cfg(feature = "s3")]
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Uploader {
    /// Create a new S3 uploader
    #[cfg(feature = "s3")]
    pub async fn new(bucket: String, prefix: String) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        let client = Client::new(&config);

        info!(
            "S3 uploader initialized for bucket: {} with prefix: {}",
            bucket, prefix
        );

        Ok(Self {
            client,
            bucket,
            prefix,
        })
    }

    #[cfg(not(feature = "s3"))]
    pub async fn new(_bucket: String, _prefix: String) -> Result<Self> {
        anyhow::bail!("S3 support not enabled. Rebuild with --features s3")
    }

    /// Upload a single file to S3
    #[cfg(feature = "s3")]
    pub async fn upload_file(&self, local_path: &Path, s3_key: &str) -> Result<()> {
        let full_key = if self.prefix.is_empty() {
            s3_key.to_string()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), s3_key)
        };

        info!(
            "Uploading {} to s3://{}/{}",
            local_path.display(),
            self.bucket,
            full_key
        );

        let body = ByteStream::from_path(local_path)
            .await
            .context(format!("Failed to read file: {}", local_path.display()))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(body)
            .send()
            .await
            .context(format!(
                "Failed to upload to s3://{}/{}",
                self.bucket, full_key
            ))?;

        info!("Successfully uploaded to s3://{}/{}", self.bucket, full_key);
        Ok(())
    }

    #[cfg(not(feature = "s3"))]
    pub async fn upload_file(&self, _local_path: &Path, _s3_key: &str) -> Result<()> {
        anyhow::bail!("S3 support not enabled. Rebuild with --features s3")
    }

    /// Upload all protobuf files from a directory
    pub async fn upload_protobuf_files(&self, pb_dir: &Path) -> Result<Vec<String>> {
        info!("Uploading protobuf files from {}", pb_dir.display());

        let mut uploaded_files = Vec::new();

        for entry in std::fs::read_dir(pb_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only upload .pb.gz files (skip index.db and logs)
            if path.is_file()
                && let Some(extension) = path.extension()
                && extension == "gz"
                && path.to_str().unwrap_or("").ends_with(".pb.gz")
            {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid filename")?;

                self.upload_file(&path, file_name).await?;
                uploaded_files.push(file_name.to_string());
            }
        }

        info!("Uploaded {} protobuf files", uploaded_files.len());
        Ok(uploaded_files)
    }

    /// Upload the index database
    pub async fn upload_index(&self, index_path: &Path) -> Result<()> {
        if !index_path.exists() {
            warn!("Index file not found: {}", index_path.display());
            return Ok(());
        }

        let file_name = index_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid index filename")?;

        self.upload_file(index_path, file_name).await?;
        Ok(())
    }

    /// Upload the log file
    pub async fn upload_log(&self, log_path: &Path) -> Result<()> {
        if !log_path.exists() {
            warn!("Log file not found: {}", log_path.display());
            return Ok(());
        }

        let file_name = log_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid log filename")?;

        self.upload_file(log_path, file_name).await?;
        Ok(())
    }

    /// Upload all files from output directory (protobuf files, index, and log)
    pub async fn upload_all(&self, output_dir: &Path) -> Result<()> {
        info!("Starting full upload from {}", output_dir.display());

        // Upload protobuf files
        let pb_files = self.upload_protobuf_files(output_dir).await?;

        // Upload index
        let index_path = output_dir.join("index.db");
        self.upload_index(&index_path).await?;

        // Upload log
        let log_path = output_dir.join("proton-beam.log");
        self.upload_log(&log_path).await?;

        info!(
            "Upload complete: {} protobuf files + index + log",
            pb_files.len()
        );
        Ok(())
    }
}

/// Helper function to parse S3 URI (s3://bucket/prefix)
pub fn parse_s3_uri(uri: &str) -> Result<(String, String)> {
    let uri = uri.trim();

    if !uri.starts_with("s3://") {
        anyhow::bail!("S3 URI must start with s3://");
    }

    let without_scheme = &uri[5..]; // Remove "s3://"

    let parts: Vec<&str> = without_scheme.splitn(2, '/').collect();

    let bucket = parts[0].to_string();
    let prefix = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        String::new()
    };

    if bucket.is_empty() {
        anyhow::bail!("S3 bucket name cannot be empty");
    }

    Ok((bucket, prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s3_uri() {
        // Test with prefix
        let (bucket, prefix) = parse_s3_uri("s3://my-bucket/path/to/data").unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(prefix, "path/to/data");

        // Test without prefix
        let (bucket, prefix) = parse_s3_uri("s3://my-bucket").unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(prefix, "");

        // Test with trailing slash
        let (bucket, prefix) = parse_s3_uri("s3://my-bucket/prefix/").unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(prefix, "prefix/");

        // Test invalid URI
        assert!(parse_s3_uri("http://my-bucket").is_err());
        assert!(parse_s3_uri("s3://").is_err());
    }
}
