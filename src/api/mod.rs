use std::net::SocketAddr;

use axum::{
    debug_handler,
    extract::Path,
    headers::{authorization::Bearer, Authorization},
    routing::{get, post, put},
    Extension, Json, Router, TypedHeader,
};
use axum_error::*;
use axum_extra::routing::SpaRouter;
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{
    database::{Database, User},
    graph::{Email, Folder, GraphClient, Profile},
    token::get_payload_field,
};

use self::error::AppError;

mod error;

#[derive(Debug, Serialize, Deserialize)]
struct TokenRequest {
    refresh_token: String,
}

pub struct Server {
    addr: SocketAddr,
    database_url: String,
}

impl Server {
    pub fn new(addr: SocketAddr, database_url: String) -> Self {
        Self { addr, database_url }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Connecting to database...");
        let db = Database::new(self.database_url.clone()).await?;

        info!("Running migrations...");
        db.migrate().await?;

        info!("Listening on {}", self.addr);
        Ok(axum::Server::bind(&self.addr)
            .serve(self.routes(db).into_make_service())
            .await?)
    }

    pub fn routes(&self, db: Database) -> Router {
        Router::new()
            .route("/api/me", get(get_profile))
            .route("/api/token", post(post_token))
            .route("/api/emails", get(get_emails))
            .route("/api/emails/move/:folder", put(put_bulk_move))
            .route("/api/emails/:id", get(get_email))
            .route("/api/emails/:id/move/:folder", put(put_move))
            .route("/api/emails/:id/archive", put(put_archive))
            .route("/api/emails/:id/spam", put(put_mark_spam))
            .route("/api/folders", get(get_folders))
            .route("/api/:folder/emails", get(get_folder_emails))
            .merge(SpaRouter::new("/", "public").index_file("index.html"))
            .layer(Extension(db))
            .layer(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::any())
                    .allow_methods(AllowMethods::any())
                    .allow_headers(AllowHeaders::any()),
            )
            .layer(TraceLayer::new_for_http())
    }
}

async fn get_profile(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Profile>, AppError> {
    let client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(client.get_user_profile().await?))
}

#[debug_handler]
async fn post_token(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Extension(db): Extension<Database>,
    Json(data): Json<TokenRequest>,
) -> Result<Json<User>, AppError> {
    let access_token = access_code.token().to_owned();
    let email = get_payload_field(&access_token, "unique_name")?;
    let client = db.get().await?;

    // TODO: do we need expiration time?
    let user =
        User::upsert_with_tokens(&client, &email, &access_token, &data.refresh_token).await?;

    Ok(Json(user))
}

async fn get_emails(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<Email>>, AppError> {
    let client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(client.get_user_emails().await?))
}

async fn get_folders(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<Folder>>, AppError> {
    let client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(client.get_user_folders().await?))
}

async fn get_folder_emails(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(folder): Path<String>,
) -> Result<Json<Vec<Email>>, AppError> {
    let mut client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(
        client.get_user_emails_from_folder_by_name(&folder).await?,
    ))
}

async fn get_email(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(id): Path<String>,
) -> Result<Json<Email>, AppError> {
    let client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(client.get_email_by_id(&id).await?))
}

async fn put_bulk_move(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(folder): Path<String>,
    Json(email_ids): Json<Vec<String>>,
) -> Result<Json<Vec<Email>>, AppError> {
    info!("Moving {email_ids:?} to {folder}...");
    let mut client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(
        client
            .move_emails_to_folder_by_name(email_ids, &folder)
            .await?,
    ))
}

async fn put_move(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path((email_id, folder_name)): Path<(String, String)>,
) -> Result<Json<Email>, AppError> {
    info!("Moving {email_id} to {folder_name}...");
    let mut client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(
        client
            .move_email_to_folder_by_name(&email_id, &folder_name)
            .await?,
    ))
}

async fn put_archive(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(email_id): Path<String>,
) -> Result<Json<Email>, AppError> {
    let mut client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(
        client
            .move_email_to_folder_by_name(&email_id, "Archive")
            .await?,
    ))
}

async fn put_mark_spam(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(email_id): Path<String>,
) -> Result<Json<Email>, AppError> {
    let mut client = GraphClient::new(access_code.token().to_owned());
    Ok(Json(
        client
            .move_email_to_folder_by_name(&email_id, "Junk Email")
            .await?,
    ))
}
