use std::io::Read;

use actix_web::web::Bytes;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, post, web};

use async_stream::stream;
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

#[get("/capture/progress/{ticket}")]
async fn capture_progress(
    path: web::Path<uuid::Uuid>,
    full_req: HttpRequest,
    state: web::Data<worker::state::State>,
) -> impl Responder {
    let bearer = get_bearer_token(&full_req);
    if !state.validate_auth_token(bearer).await {
        return HttpResponse::Unauthorized().finish();
    }
    let ticket = path.into_inner();
    let status = state.capture_status(&ticket).await;
    HttpResponse::Ok().json(status)
}

#[post("/capture/confirm")]
async fn capture_confirm(
    req: web::Json<corwrk::ConfirmCaptureRequest>,
    full_req: HttpRequest,
    state: web::Data<worker::state::State>,
) -> impl Responder {
    let bearer = get_bearer_token(&full_req);
    if !state.validate_auth_token(bearer).await {
        return HttpResponse::Unauthorized().finish();
    }
    let known_hash = match state.get_hash(req.ticket()).await {
        None => {
            return HttpResponse::NotFound().json(corwrk::ConfirmCaptureResponse::NoSuchCapture);
        }
        Some(h) => h,
    };
    if known_hash == req.hash() {
        HttpResponse::Ok().json(corwrk::ConfirmCaptureResponse::CorrectHash)
    } else {
        HttpResponse::Ok().json(corwrk::ConfirmCaptureResponse::IncorrectHash)
    }
}

#[get("/capture/output/{ticket}")]
async fn capture_output(
    path: web::Path<uuid::Uuid>,
    full_req: HttpRequest,
    state: web::Data<worker::state::State>,
) -> impl Responder {
    let bearer = get_bearer_token(&full_req);
    if !state.validate_auth_token(bearer).await {
        return HttpResponse::Unauthorized().finish();
    }
    let ticket: uuid::Uuid = path.into_inner();
    let file_path = state.blob_dir().join(ticket.to_string());
    if let Ok(mut file) = actix_files::NamedFile::open(file_path) {
        let stream = stream! {
            let mut chunk = vec![0u8; 10 * 1024 * 1024];
            loop {
                match file.read(&mut chunk) {
                    Ok(n) => {
                        if n == 0 {
                            break;
                        }
                        yield Result::<Bytes, std::io::Error>::Ok(Bytes::from(chunk[..n].to_vec()));
                    }
                    Err(e) => {
                        yield Result::<Bytes, std::io::Error>::Err(e);
                        break;
                    }
                }
            }
        };
        HttpResponse::Ok()
            .content_type("application/octet-stream")
            .streaming(stream)
    } else {
        HttpResponse::NotFound().finish()
    }
}

async fn server(config: worker::config::WorkerConfig) -> std::io::Result<()> {
    let data = web::Data::new(worker::state::State::from_config(config.clone()).await);
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(version)
            .service(capture_create)
            .service(capture_progress)
            .service(capture_confirm)
            .service(capture_output)
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
