#[macro_use]
extern crate log;

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, post, web};

use webarc::msg::corwrk;
use webarc::worker;

/// Extract `token` from `Authorization: Bearer token` header, if able
fn get_bearer_token(req: &HttpRequest) -> Option<String> {
    let authorization = req.headers().get("authorization")?.to_str().ok()?;
    if !authorization.starts_with("Bearer ") {
        return None;
    }
    let token = (&authorization[7..]).to_string();
    Some(token)
}

#[get("/version")]
async fn version() -> impl Responder {
    format!("{}", env!("CARGO_PKG_VERSION"))
}

#[post("/capture/create")]
async fn capture_create(
    req: web::Json<corwrk::InitiateCaptureRequest>,
    full_req: HttpRequest,
    state: web::Data<worker::state::State>,
) -> impl Responder {
    let bearer = get_bearer_token(&full_req);
    if !state.validate_auth_token(bearer).await {
        return HttpResponse::Unauthorized().finish();
    }
    let exe = match state.locate_extractor(req.extractor()).await {
        None => {
            let response = corwrk::InitiateCaptureResponse::InvalidExtractor;
            return HttpResponse::BadRequest().json(response);
        }
        Some(e) => e,
    };
    let url = req.url().clone();
    let new_ticket = uuid::Uuid::new_v4();
    state.register_capture(new_ticket).await;
    tokio::spawn(worker::task::capture_task(new_ticket, exe, url, state));
    HttpResponse::Ok().json(corwrk::InitiateCaptureResponse::Initiated { ticket: new_ticket })
}

async fn server(config: worker::config::WorkerConfig) -> std::io::Result<()> {
    let data = web::Data::new(worker::state::State::from_config(config.clone()).await);
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(version)
            .service(capture_create)
    })
    .bind(config.listen())?
    .run()
    .await
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config_path = match std::env::var("WEBARC_WORKER_CONFIG") {
        Ok(v) => std::path::PathBuf::from(v),
        Err(e) => {
            eprintln!("Unable to determine config file location: {e}");
            eprintln!("Try setting the WEBARC_WORKER_CONFIG environment variable.");
            return;
        }
    };

    let config = match worker::config::WorkerConfig::from_path(config_path).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let _ = server(config).await;
}
