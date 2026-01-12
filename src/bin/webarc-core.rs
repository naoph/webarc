use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};

use webarc::core;

#[get("/version")]
async fn version() -> impl Responder {
    format!("{}", env!("CARGO_PKG_VERSION"))
}

async fn server(config: core::config::CoreConfig) -> std::io::Result<()> {
    let data = web::Data::new(core::state::State::from_config(config.clone()).await);
    HttpServer::new(move || App::new().app_data(data.clone()).service(version))
        .bind(config.listen())?
        .run()
        .await
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config_path = match std::env::var("WEBARC_CORE_CONFIG") {
        Ok(v) => std::path::PathBuf::from(v),
        Err(e) => {
            eprintln!("Unable to determine config file location: {e}");
            eprintln!("Try setting the WEBARC_CORE_CONFIG environment variable.");
            return;
        }
    };

    let config = match core::config::CoreConfig::from_path(config_path).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let _ = server(config).await;
}
