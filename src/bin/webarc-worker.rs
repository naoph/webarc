#[macro_use]
extern crate log;

use actix_web::{App, HttpServer, Responder, get};

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

    let sock = ("127.0.0.1", 8081);
    let _ = server(sock).await;
}
