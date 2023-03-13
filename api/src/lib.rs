use std::net::SocketAddr;

use axum::{http::HeaderMap, routing::get, Json, Router};
use axum_error::*;
use email::Email;
use fehler::throws;
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
        Router::new().route("/api/emails", get(get_emails))
    }
}

#[throws]
async fn get_emails(headers: HeaderMap) -> Json<Vec<Email>> {
    let auth_header = headers
        .get("authorization")
        .ok_or(eyre::eyre!("No authorization header"))?;

    let access_code = auth_header.to_str()?.to_string();
    let server = email::Server::new(access_code);
    Json(server.fetch("INBOX")?)
}
