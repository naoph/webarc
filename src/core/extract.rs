use log::*;

pub async fn extract(
    state: actix_web::web::Data<crate::core::state::State>,
    extractor: String,
    url: url::Url,
    db_capid: i32,
) {
    let worker = state
        .worker_dispatch()
        .select_worker(&extractor, &url)
        .await;
    debug!("Extract task for {extractor} / {url} assigned worker {worker}");
}
