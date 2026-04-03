use log::*;
use snafu::prelude::*;

use crate::{core::models::InsExtract, msg::corwrk};

pub async fn extract(
    state: actix_web::web::Data<crate::core::state::State>,
    extractor: String,
    url: url::Url,
    db_capid: i32,
) {
    let result = try_extract(state, extractor, url, db_capid).await;
    println!("final extract result: {:#?}", result);
}

async fn try_extract(
    state: actix_web::web::Data<crate::core::state::State>,
    extractor: String,
    url: url::Url,
    db_capid: i32,
) -> Result<InsExtract, InsExtract> {
    let worker = state
        .worker_dispatch()
        .select_worker(&extractor, &url)
        .await;
    debug!("Extract task for {extractor} / {url} assigned worker {worker}");
    let descriptor = state.worker_dispatch().describe_worker(&worker).await;
    let http = state.http_client();
    let extract_uuid = uuid::Uuid::new_v4();
    let failure = InsExtract::new(extract_uuid.clone(), db_capid, extractor.clone(), false);
    let mut initresp: corwrk::InitiateExtractResponse = corwrk::InitiateExtractResponse::InvalidUrl;

    // Try up to 3 times to initiate
    for i in 0..3 {
        match initiate(&http, &extractor, &url, &descriptor).await {
            Ok(r) => {
                initresp = r;
                break;
            }
            Err(e) => {
                error!("POST /extract/create encountered an error: {:?}", e);
                match i {
                    0 => tokio::time::sleep(tokio::time::Duration::from_secs(5)).await,
                    1 => tokio::time::sleep(tokio::time::Duration::from_secs(30)).await,
                    _ => {
                        error!("Initiating extract on {extractor} / {url} failed 3 times");
                        return Err(failure);
                    }
                }
                continue;
            }
        }
    }
    let ticket = match initresp {
        corwrk::InitiateExtractResponse::InvalidUrl => {
            error!("Extracting {extractor} / {url} returned InvalidUrl");
            return Err(failure);
        }
        corwrk::InitiateExtractResponse::InvalidExtractor => {
            error!("Extracting {extractor} / {url} returned InvalidExtractor");
            return Err(failure);
        }
        corwrk::InitiateExtractResponse::Initiated { ticket } => ticket,
    };

    // Wait for a final result from the worker
    let mut abnormal_responses: u32 = 0;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        match progcheck(&http, &descriptor, &ticket).await {
            Ok(corwrk::QueryExtractProgressResponse::InProgress) => {
                continue;
            }
            Ok(corwrk::QueryExtractProgressResponse::NoSuchExtract) => {
                debug!("w [{worker}] / e [{extractor}] / u [{url}]: NoSuchExtract");
                abnormal_responses += 1;
                if abnormal_responses > 6 {
                    debug!(
                        "w [{worker}] / e [{extractor}] / u [{url}]: Too many abnormal responses, bailing"
                    );
                    return Err(failure);
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        2_u64.pow(abnormal_responses),
                    ))
                    .await;
                    continue;
                }
            }
            Ok(corwrk::QueryExtractProgressResponse::UnsupportedUrl) => {
                error!("w [{worker}] / e [{extractor}] / u [{url}]: UnsupportedUrl");
                return Err(failure);
            }
            Ok(corwrk::QueryExtractProgressResponse::Failed) => {
                error!("w [{worker}] / e [{extractor}] / u [{url}]: Failed");
                return Err(failure);
            }
            Ok(corwrk::QueryExtractProgressResponse::Completed) => {
                debug!("w [{worker}] / e [{extractor}] / u [{url}]: Completed");
                break;
            }
            Err(e) => {
                debug!("w [{worker}] / e [{extractor}] / u [{url}]: Err: {e}");
                abnormal_responses += 1;
                if abnormal_responses > 6 {
                    debug!(
                        "w [{worker}] / e [{extractor}] / u [{url}]: Too many abnormal responses, bailing"
                    );
                    return Err(failure);
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        2_u64.pow(abnormal_responses),
                    ))
                    .await;
                    continue;
                }
            }
        }
    }
    Ok(failure)
}

async fn initiate(
    http: &reqwest::Client,
    extractor: &str,
    url: &url::Url,
    descriptor: &crate::core::state::WorkerDescriptor,
) -> Result<corwrk::InitiateExtractResponse, WebClientError> {
    let endpoint = descriptor.url().join("/extract/create").unwrap();
    let req = corwrk::InitiateExtractRequest::new(url, extractor);
    let req = http
        .post(endpoint)
        .json(&req)
        .header("Authorization", format!("Bearer {}", descriptor.token()));
    let resp = req.send().await;
    let resp = resp.context(ReqwestSnafu)?;
    resp.json().await.context(JsonSnafu)
}

async fn progcheck(
    http: &reqwest::Client,
    descriptor: &crate::core::state::WorkerDescriptor,
    ticket: &uuid::Uuid,
) -> Result<corwrk::QueryExtractProgressResponse, WebClientError> {
    let endpoint = descriptor
        .url()
        .join("/extract/progress/")
        .unwrap()
        .join(&ticket.to_string())
        .unwrap();
    let req = http
        .get(endpoint)
        .header("Authorization", format!("Bearer {}", descriptor.token()));
    let resp = req.send().await;
    let resp = resp.context(ReqwestSnafu)?;
    resp.json().await.context(JsonSnafu)
}

#[derive(Debug, Snafu)]
enum WebClientError {
    #[snafu(display("reqwest returned an error"))]
    ReqwestError { source: reqwest::Error },

    #[snafu(display("response could not be deserialized as json"))]
    JsonError { source: reqwest::Error },
}
