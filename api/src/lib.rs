use std::net::SocketAddr;

use axum::{
    extract::Path,
    headers::{authorization::Bearer, Authorization},
    routing::{get, put},
    Json, Router, TypedHeader,
};
use axum_error::*;
use email::Email;
use fehler::throws;
use serde_json::json;
use tracing::info;

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
            .route("/api/emails/:internal_id", get(get_email))
            .route("/api/emails/:internal_id/archive", put(put_archive))
            .route("/api/emails/:internal_id/spam", put(put_mark_spam))
    }
}

#[throws]
async fn get_emails(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
) -> Json<Vec<Email>> {
    let server = email::Server::new(access_code.token().to_owned())?;
    Json(server.fetch("INBOX")?)
}

#[throws]
async fn get_email(
    TypedHeader(access_code): TypedHeader<Authorization<Bearer>>,
    Path(internal_id): Path<String>,
) -> Json<String> {
    let server = email::Server::new(access_code.token().to_owned())?;
    Json(server.fetch_body("INBOX", &internal_id)?)
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
    server.move_emails("INBOX", "Junk Mail", vec![&internal_id])?;
    Json(json!({ "ok": true }))
}
