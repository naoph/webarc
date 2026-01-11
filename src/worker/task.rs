use actix_web::web;
use log::*;
use url::Url;
use uuid::Uuid;

use crate::worker::state::State;

pub async fn capture_task(ticket: Uuid, extractor: String, url: Url, state: web::Data<State>) {
    debug!("Begin capture task\nticket:    {ticket}\nextractor: {extractor}\nurl:       {url}");
}
