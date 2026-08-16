use actix_web::web;
use diesel_async::RunQueryDsl;
use log::*;
use snafu::prelude::*;

use crate::core;
use crate::core::extract;

#[derive(Debug, Snafu)]
pub enum CreateCaptureError {
    #[snafu(display("No appropriate extractors for URL"))]
    NoAppropriateExtractorsError,

    #[snafu(display("Mysterious database error"))]
    MysteriousDatabaseError {
        source: mobc::Error<diesel_async::pooled_connection::PoolError>,
    },

    #[snafu(display("Unable to insert"))]
    UnableToInsertError { source: diesel::result::Error },

    #[snafu(display("Unable to register capture"))]
    UnableToRegisterError {
        source: crate::core::state::StorageError,
    },
}

pub async fn create_capture(
    url: url::Url,
    user_id: i32,
    public: bool,
    state: web::Data<core::state::State>,
) -> Result<uuid::Uuid, CreateCaptureError> {
    // Determine appropriate extractors for URL
    let extractors = state.extractor_map().await.extractors_for_url(&url).await;
    debug!("Extractors for {}: {:?}", &url, extractors);
    if extractors.len() == 0 {
        return Err(CreateCaptureError::NoAppropriateExtractorsError);
    }

    // Build and insert the capture
    let capture_uuid = uuid::Uuid::new_v4();
    let new_capture = core::models::InsCapture {
        uuid: capture_uuid,
        url: url.clone(),
        time_initiated: chrono::Utc::now(),
        owner: user_id,
        public,
    };
    let mut conn = state
        .db_pool()
        .await
        .get()
        .await
        .context(MysteriousDatabaseSnafu)?;
    let new_capture: core::models::DbCapture = diesel::insert_into(core::schema::captures::table)
        .values(new_capture)
        .get_result(&mut conn)
        .await
        .context(UnableToInsertSnafu)?;

    // Recordkeeping
    state
        .capture_map()
        .await
        .new_status(&capture_uuid, extractors.len(), user_id, public)
        .await;
    state
        .storage_manager()
        .register_capture(&capture_uuid)
        .await
        .context(UnableToRegisterSnafu)?;

    // Spawn extraction tasks
    for extractor in extractors.iter() {
        let state = state.clone();
        let extractor = extractor.clone();
        let url = url.clone();
        let db_capid = new_capture.id;
        tokio::spawn(extract::extract(
            state,
            extractor,
            url,
            db_capid,
            capture_uuid.clone(),
        ));
    }

    Ok(capture_uuid)
}
