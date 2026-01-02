#[macro_use]
extern crate log;

use actix_web::{App, HttpServer, Responder, get};

use webarc::worker;

#[get("/version")]
async fn version() -> impl Responder {
    format!("{}", env!("CARGO_PKG_VERSION"))
}

async fn server(sock: impl std::net::ToSocketAddrs) -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(version))
        .bind(sock)?
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

    let _ = server(config.listen()).await;
}
