use std::net::SocketAddr;

use axum::{
    extract::Path,
    headers::{authorization::Bearer, Authorization},
    routing::{get, put},
    Json, Router, TypedHeader,
};
use axum_error::*;
use fehler::throws;
use serde_json::json;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::graph::{Email, Folder, GraphClient};

pub mod email;

pub struct Server {
    addr: SocketAddr,
}

impl Server {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Listening on {}", self.addr);
        Ok(axum::Server::bind(&self.addr)
            .serve(self.routes().into_make_service())
            .await?)
    }

    pub fn routes(&self) -> Router {
        Router::new()
            .route("/api/emails", get(get_emails))
            .route("/api/emails/move/:folder", put(put_bulk_move))
            .route("/api/emails/:id", get(get_email))
            .route("/api/emails/:id/move/:folder", put(put_move))
            .route("/api/emails/:id/archive", put(put_archive))
            .route("/api/emails/:id/spam", put(put_mark_spam))
            .route("/api/folders", get(get_folders))
            .route("/api/:folder/emails", get(get_folder_emails))
            .layer(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::any())
                    .allow_methods(AllowMethods::any())
                    .allow_headers(AllowHeaders::any()),
            )
            .layer(TraceLayer::new_for_http())
    }
}

#[throws]
async fn get_emails(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
) -> Json<Vec<Email>> {
    let client = GraphClient::new(access_code.token().to_owned());
    Json(client.get_user_emails().await?)
}

#[throws]
async fn get_folders(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
) -> Json<Vec<Folder>> {
    let client = GraphClient::new(access_code.token().to_owned());
    Json(client.get_user_folders().await?)
}

#[throws]
async fn get_folder_emails(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(folder): Path<String>,
) -> Json<Vec<Email>> {
    let client = GraphClient::new(access_code.token().to_owned());
    Json(client.get_user_emails_from_folder(&folder).await?)
}

#[throws]
async fn get_email(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(id): Path<String>,
) -> Json<Email> {
    let client = GraphClient::new(access_code.token().to_owned());
    Json(client.get_email_by_id(&id).await?)
}

#[throws]
async fn put_bulk_move(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(folder): Path<String>,
    Json(internal_ids): Json<Vec<String>>,
) -> Json<serde_json::Value> {
    info!("Moving {internal_ids:?} to {folder}...");
    let server = email::Server::new(access_code.token().to_owned())?;
    let internal_ids = internal_ids.iter().map(|s| s.as_str()).collect();
    // FIXME assuming INBOX for the folder
    server.move_emails("INBOX", &folder, internal_ids)?;
    Json(json!({ "ok": true }))
}

#[throws]
async fn put_move(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path((email_id, folder_name)): Path<(String, String)>,
) -> Json<Email> {
    info!("Moving {email_id} to {folder_name}...");
    let mut client = GraphClient::new(access_code.token().to_owned());
    Json(
        client
            .move_email_to_folder_by_name(&email_id, &folder_name)
            .await?,
    )
}

#[throws]
async fn put_archive(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(internal_id): Path<String>,
) -> Json<serde_json::Value> {
    let server = email::Server::new(access_code.token().to_owned())?;
    // FIXME assuming INBOX for the folder
    server.move_emails("INBOX", "Archive", vec![&internal_id])?;
    Json(json!({ "ok": true }))
}

#[throws]
async fn put_mark_spam(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(internal_id): Path<String>,
) -> Json<serde_json::Value> {
    let server = email::Server::new(access_code.token().to_owned())?;
    // FIXME assuming INBOX for the folder
    server.move_emails("INBOX", "Junk Email", vec![&internal_id])?;
    Json(json!({ "ok": true }))
}
