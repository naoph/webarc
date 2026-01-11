use std::path::PathBuf;

use actix_web::web;
use async_process::Command;
use log::*;
use sha2::{Digest, Sha256};
use snafu::prelude::*;
use tokio::io::AsyncWriteExt;
use url::Url;
use uuid::Uuid;

use crate::worker::state::State;

pub async fn capture_task(ticket: Uuid, extractor: String, url: Url, state: web::Data<State>) {
    debug!("Begin capture task\nticket:    {ticket}\nextractor: {extractor}\nurl:       {url}");
    let result = Command::new(extractor)
        .arg(url.to_string())
        .arg("asdf")
        .output()
        .await;
    let output = match result {
        Ok(o) => o,
        Err(e) => {
            error!("Error in extractor process: {e}");
            state.abort_capture(ticket).await;
            return;
        }
    };
    let blob = if output.status.success() {
        output.stdout
    } else {
        let err_string = str::from_utf8(&output.stderr)
            .unwrap_or("[bytes]")
            .to_string();
        error!("Extractor exited nonzero\n{}", err_string);
        state.abort_capture(ticket).await;
        return;
    };
    debug!("Extraction successful; blob size: {}", blob.len());
    match write_blob(state.blob_dir(), &ticket, blob).await {
        Ok(h) => {
            state.finalize_capture(ticket, h).await;
        }
        Err(e) => {
            error!("Error writing blob to disc: {e}");
            state.abort_capture(ticket).await;
        }
    }
}

/// Write a blob to disc, returning its sha256 sum
pub async fn write_blob(
    blob_dir: &PathBuf,
    ticket: &Uuid,
    bytevec: Vec<u8>,
) -> Result<String, WriteBlobError> {
    let write_path = blob_dir.join(ticket.to_string());
    let mut f = tokio::fs::File::create(write_path)
        .await
        .context(CreateBlobFileSnafu)?;
    f.write(&bytevec).await.context(WriteBlobFileSnafu)?;
    f.flush().await.context(WriteBlobFileSnafu)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytevec);
    let hash = hex::encode(hasher.finalize());

    Ok(hash)
}

#[derive(Debug, Snafu)]
pub enum WriteBlobError {
    #[snafu(display("Unable to create blob file"))]
    CreateBlobFile { source: std::io::Error },

    #[snafu(display("Unable to write blob file"))]
    WriteBlobFile { source: std::io::Error },
}
