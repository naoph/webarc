use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use diesel_async::RunQueryDsl;
use log::*;

use webarc::core;
use webarc::core::models::*;
use webarc::msg::clicor;

#[get("/version")]
async fn version() -> impl Responder {
    format!("{}", env!("CARGO_PKG_VERSION"))
}

#[post("/user/create")]
async fn user_create(
    req: web::Json<clicor::CreateUserRequest>,
    state: web::Data<core::state::State>,
) -> impl Responder {
    let username = req.username();
    let password = req.password();
    if username.len() == 0 {
        return HttpResponse::BadRequest().json(clicor::CreateUserResponse::InvalidUsername);
    }
    if password.len() == 0 {
        return HttpResponse::BadRequest().json(clicor::CreateUserResponse::InvalidPassword);
    }
    let passhash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            error!("bcrypt::hash failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: bcrypt");
        }
    };
    let mut conn = match state.db_pool().await.get().await {
        Ok(c) => c,
        Err(e) => {
            error!("db_pool.get() failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: db pool");
        }
    };
    let new_user = core::models::InsUser::new(username.to_string(), passhash);
    let count = diesel::insert_into(core::schema::users::table)
        .values(new_user)
        .execute(&mut conn)
        .await;
    match count {
        Ok(1) => HttpResponse::Created().json(clicor::CreateUserResponse::Created),
        Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            HttpResponse::Conflict().json(clicor::CreateUserResponse::UnavailableUsername)
        }
        Err(e) => {
            error!("New username insertion failed unexpectedly: {e}");
            HttpResponse::InternalServerError().body("Internal server error: db insert")
        }
        Ok(n) => {
            error!("New username insertion should affect 1 row but affected {n}");
            HttpResponse::InternalServerError()
                .body("Internal server error: db affected too many rows")
        }
    }
}

#[post("/auth")]
async fn auth(
    req: web::Json<clicor::AuthRequest>,
    state: web::Data<core::state::State>,
) -> impl Responder {
    use core::schema::users;
    if req.username().len() == 0 || req.password().len() == 0 {
        return HttpResponse::BadRequest().json(clicor::AuthResponse::UnacceptableCredentials);
    }
    let mut conn = match state.db_pool().await.get().await {
        Ok(c) => c,
        Err(e) => {
            error!("db_pool.get() failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: db pool");
        }
    };

    let user: Result<Vec<DbUser>, _> = users::dsl::users
        .filter(users::dsl::username.eq(req.username()))
        .load(&mut conn)
        .await;
    let user = match user {
        Ok(mut v) => match v.len() {
            0 => {
                return HttpResponse::Unauthorized().json(clicor::AuthResponse::InvalidCredentials);
            }
            1 => v.remove(0),
            _ => {
                error!("/auth multiple users with same username");
                return HttpResponse::InternalServerError().body("Internal server error: get user");
            }
        },
        Err(e) => {
            error!("/auth get user failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: get user");
        }
    };
    match bcrypt::verify(req.password(), &user.passhash) {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Unauthorized().json(clicor::AuthResponse::InvalidCredentials);
        }
        Err(e) => {
            error!("/auth verify hash failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: verify user");
        }
    };
    let new_token = rand::random::<u128>();
    state.register_token(new_token, user.id).await;

    HttpResponse::Ok().json(clicor::AuthResponse::Authenticated {
        token: new_token.to_string(),
    })
}

async fn server(config: core::config::CoreConfig) -> std::io::Result<()> {
    let data = web::Data::new(core::state::State::from_config(config.clone()).await);
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(version)
            .service(user_create)
            .service(auth)
    })
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
