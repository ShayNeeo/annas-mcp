use std::path::{Path, PathBuf};
use futures_util::StreamExt;
use reqwest::{Client, Response};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::downloader::sanitize::make_safe_filepath;
use crate::errors::{AppError, Result};

#[derive(Debug, Clone)]
pub struct DownloadedFileInfo {
    pub file_path: PathBuf,
    pub bytes_written: u64,
    pub filename: String,
}

pub struct FileDownloader {
    client: Client,
}

impl FileDownloader {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn stream_response_to_file(
        response: Response,
        destination_path: &Path,
    ) -> Result<DownloadedFileInfo> {
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let temp_path = destination_path.with_extension("download.tmp");
        let mut file = File::create(&temp_path).await?;
        let mut bytes_written: u64 = 0;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Err(e) = file.write_all(&chunk).await {
                        let _ = fs::remove_file(&temp_path).await;
                        return Err(AppError::Download(format!("Failed to write chunk: {e}")));
                    }
                    bytes_written += chunk.len() as u64;
                }
                Err(e) => {
                    let _ = fs::remove_file(&temp_path).await;
                    return Err(AppError::Download(format!("Stream read error: {e}")));
                }
            }
        }

        if let Err(e) = file.flush().await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(AppError::Download(format!("Failed to flush file: {e}")));
        }
        drop(file);

        if let Err(e) = fs::rename(&temp_path, destination_path).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(AppError::Download(format!("Failed to finalize file: {e}")));
        }

        let filename = destination_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!(
            "Successfully saved file: {} ({} bytes)",
            destination_path.display(),
            bytes_written
        );

        Ok(DownloadedFileInfo {
            file_path: destination_path.to_path_buf(),
            bytes_written,
            filename,
        })
    }

    pub async fn download_url(
        &self,
        url: &str,
        destination_path: &Path,
    ) -> Result<DownloadedFileInfo> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Download(format!("Failed to fetch download URL: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Download(format!(
                "Download request failed with status {status}: {body}"
            )));
        }

        Self::stream_response_to_file(resp, destination_path).await
    }

    pub async fn download_to_folder(
        &self,
        url: &str,
        folder: &Path,
        title: &str,
        format: &str,
    ) -> Result<DownloadedFileInfo> {
        let target_path = make_safe_filepath(folder, title, format);
        self.download_url(url, &target_path).await
    }
}
