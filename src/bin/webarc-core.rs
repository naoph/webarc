use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, cookie, get, post, web};
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use diesel_async::RunQueryDsl;
use lazy_static::lazy_static;
use log::*;
use tera::{Context, Tera};

use webarc::core;
use webarc::core::extract;
use webarc::core::models::*;
use webarc::core::schema;
use webarc::msg::clicor;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*.html");
        let tera = match Tera::new(path) {
            Ok(t) => t,
            Err(e) => {
                error!("Tera setup error: {e}");
                std::process::exit(1);
            }
        };
        tera
    };
}

#[derive(serde::Deserialize)]
struct AuthForm {
    pub username: String,
    pub password: String,
}

/// Extract `token` from `Authorization: Bearer token` header, if able
fn get_bearer_token(req: &HttpRequest) -> Option<u128> {
    let authorization = req.headers().get("authorization")?.to_str().ok()?;
    if !authorization.starts_with("Bearer ") {
        return None;
    }
    let token = (&authorization[7..]).to_string();
    let token = token.parse::<u128>().ok();
    token
}

/// Extract token from cookie, if able
fn get_cookie_token(req: &HttpRequest) -> Option<u128> {
    let cookie = req.cookie("auth")?;
    let token = cookie.value();
    let token = token.parse::<u128>().ok();
    token
}

/// Extract token by any available methods
fn get_token(req: &HttpRequest) -> Option<u128> {
    get_bearer_token(req).or(get_cookie_token(req))
}

#[get("/version")]
async fn version() -> impl Responder {
    format!("{}", env!("CARGO_PKG_VERSION"))
}

#[get("/login")]
async fn tera_login() -> impl Responder {
    let document = TEMPLATES.render("login.html", &Context::new());
    match document {
        Ok(d) => HttpResponse::Ok().body(d),
        Err(e) => {
            error!("Error rendering login.html: {e}");
            HttpResponse::InternalServerError().body("Error rendering login.html")
        }
    }
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

#[post("/auth/form")]
async fn auth_form(
    form: web::Form<AuthForm>,
    state: web::Data<core::state::State>,
) -> impl Responder {
    use core::schema::users;
    if form.username.len() == 0 || form.password.len() == 0 {
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
        .filter(users::dsl::username.eq(&form.username))
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
    match bcrypt::verify(&form.password, &user.passhash) {
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
    let mut cookie = cookie::Cookie::new("auth", new_token.to_string());
    cookie.set_path("/");
    HttpResponse::Ok().cookie(cookie).body("login successful")
}

#[post("/capture/create")]
async fn capture_create(
    req: web::Json<clicor::CreateCaptureRequest>,
    full_req: HttpRequest,
    state: web::Data<core::state::State>,
) -> impl Responder {
    let bearer = match get_bearer_token(&full_req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(clicor::CreateCaptureResponse::Unauthenticated);
        }
    };
    let user_id = match state.user_from_token(bearer).await {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized()
                .json(clicor::CreateCaptureResponse::Unauthenticated);
        }
    };
    let extractors = state
        .extractor_map()
        .await
        .extractors_for_url(req.url())
        .await;
    debug!("Extractors for {}: {:?}", req.url(), extractors);
    if extractors.len() == 0 {
        return HttpResponse::BadRequest().json(clicor::CreateCaptureResponse::NoExtractors);
    }
    let capture_uuid = uuid::Uuid::new_v4();
    let new_capture = core::models::InsCapture {
        uuid: capture_uuid,
        url: req.url().clone(),
        time_initiated: chrono::Utc::now(),
        owner: user_id,
        public: req.public(),
    };
    let mut conn = match state.db_pool().await.get().await {
        Ok(c) => c,
        Err(e) => {
            error!("db_pool.get() failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: db pool");
        }
    };
    let new_capture: Result<core::models::DbCapture, _> =
        diesel::insert_into(core::schema::captures::table)
            .values(new_capture)
            .get_result(&mut conn)
            .await;
    debug!("new_capture: {:?}", new_capture);
    let new_capture = match new_capture {
        Ok(c) => c,
        Err(e) => {
            error!("new_capture was Err: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: db");
        }
    };
    state
        .capture_map()
        .await
        .new_status(&capture_uuid, extractors.len(), user_id, req.public())
        .await;
    // Create a directory for the new capture
    match state
        .storage_manager()
        .register_capture(&capture_uuid)
        .await
    {
        Ok(()) => {}
        Err(e) => {
            error!("Capture storage directory could not be created: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: fs");
        }
    }
    for extractor in extractors.iter() {
        let state = state.clone();
        let extractor = extractor.clone();
        let url = req.url().clone();
        let db_capid = new_capture.id;
        tokio::spawn(extract::extract(
            state,
            extractor,
            url,
            db_capid,
            capture_uuid.clone(),
        ));
    }
    HttpResponse::Accepted().json(clicor::CreateCaptureResponse::Initiated {
        capture_id: capture_uuid,
    })
}

#[get("/capture/{uuid}/status")]
async fn capture_status(
    uuid: web::Path<uuid::Uuid>,
    full_req: HttpRequest,
    state: web::Data<core::state::State>,
) -> impl Responder {
    let bearer = match get_bearer_token(&full_req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(clicor::CreateCaptureResponse::Unauthenticated);
        }
    };
    let user_id = match state.user_from_token(bearer).await {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized()
                .json(clicor::CreateCaptureResponse::Unauthenticated);
        }
    };
    let status = match state.capture_map().await.get_status(&uuid).await {
        Some(a) => a,
        None => {
            return HttpResponse::NotFound().body("Not found");
        }
    };
    if status.allows_user(user_id) {
        HttpResponse::Ok().json(status.get_progress())
    } else {
        HttpResponse::Unauthorized().body("Unauthorized")
    }
}

#[get("/resource/{uuid}/{tail:.*}")]
async fn resource(
    pair: web::Path<(uuid::Uuid, std::path::PathBuf)>,
    full_req: HttpRequest,
    state: web::Data<core::state::State>,
) -> impl Responder {
    let (uuid, tail) = pair.into_inner();
    let bearer = match get_token(&full_req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(clicor::CreateCaptureResponse::Unauthenticated);
        }
    };
    let user_id = match state.user_from_token(bearer).await {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized()
                .json(clicor::CreateCaptureResponse::Unauthenticated);
        }
    };
    let mut conn = match state.db_pool().await.get().await {
        Ok(c) => c,
        Err(e) => {
            error!("db_pool.get() failed: {e}");
            return HttpResponse::InternalServerError().body("Internal server error: db pool");
        }
    };
    let capture: Result<core::models::DbCapture, _> = schema::captures::table
        .filter(schema::captures::uuid.eq(&uuid))
        .get_result(&mut conn)
        .await;
    let capture = match capture {
        Ok(c) => c,
        Err(diesel::result::Error::NotFound) => {
            return HttpResponse::NotFound().body("No such capture");
        }
        Err(e) => {
            error!("Database error loading /capture/{uuid}: {e}");
            return HttpResponse::InternalServerError().body("Internal server error");
        }
    };
    if (!capture.public) && (capture.owner != user_id) {
        return HttpResponse::Unauthorized().body("Not authorized to view capture");
    }
    let mime = state
        .storage_manager()
        .asset_mime(&uuid, tail.clone())
        .await;
    let mime = match mime {
        Some(m) => m,
        None => return HttpResponse::InternalServerError().body("Internal server error: no mime"),
    };
    let size = state
        .storage_manager()
        .asset_size(&uuid, tail.clone())
        .await;
    let size = match size {
        Some(s) => s,
        None => return HttpResponse::InternalServerError().body("Internal server error: no size"),
    };
    let stream = state.storage_manager().asset_stream(&uuid, tail).await;
    HttpResponse::Ok()
        .insert_header(actix_web::http::header::ContentLength(size))
        .content_type(mime)
        .streaming(stream)
}

async fn server(config: core::config::CoreConfig) -> std::io::Result<()> {
    let data = web::Data::new(core::state::State::from_config(config.clone()).await);
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(version)
            .service(tera_login)
            .service(user_create)
            .service(auth)
            .service(auth_form)
            .service(capture_create)
            .service(capture_status)
            .service(resource)
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
