use std::net::SocketAddr;

use axum::{routing::get, Router};
use tracing::info;

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

pub async fn get_emails() -> &'static str {
    "Hello, world!"
}
