use deadpool_postgres::{Config, CreatePoolError, Pool, PoolError, Runtime};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_postgres::NoTls;
use url::Url;

pub type Result<T> = std::result::Result<T, DatabaseError>;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("postgres pool error: {0}")]
    Pool(#[from] PoolError),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("error creating pool: {0}")]
    CreatePool(#[from] CreatePoolError),

    #[error("postgres error: {0}")]
    Pg(#[from] tokio_postgres::Error),

    #[error("migration error: {0}")]
    Migration(#[from] refinery::Error),
}

#[derive(Clone)]
pub struct Database {
    database_url: String,
    pool: Pool,
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

impl Database {
    pub async fn new(database_url: String) -> Result<Self> {
        let config = create_deadpool_config_from_url(&database_url)?;
        let pool = config.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)?;
        Ok(Self { database_url, pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        let (mut client, connection) = tokio_postgres::connect(&self.database_url, NoTls).await?;

        // Spawn a new tokio task to run the connection in the background.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        embedded::migrations::runner()
            .run_async(&mut client)
            .await?;
        Ok(())
    }

    pub async fn get(&self) -> Result<deadpool_postgres::Client> {
        Ok(self.pool.get().await?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Option<i32>,
    pub email: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl User {
    pub async fn find(client: &deadpool_postgres::Client, email: &str) -> Result<Option<Self>> {
        let stmt = client
            .prepare("SELECT id, email, access_token, refresh_token FROM users WHERE email = $1")
            .await?;
        let rows = client.query(&stmt, &[&email]).await?;
        Ok(rows.first().map(|row| Self {
            id: Some(row.get(0)),
            email: row.get(1),
            access_token: row.get(2),
            refresh_token: row.get(3),
        }))
    }

    pub async fn upsert_with_tokens(
        client: &deadpool_postgres::Client,
        email: &str,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<Self> {
        let stmt = client
            .prepare(
                "INSERT INTO users (email, access_token, refresh_token) VALUES ($1, $2, $3)
                ON CONFLICT (email) DO UPDATE SET access_token = $2, refresh_token = $3
                RETURNING id, email, access_token, refresh_token",
            )
            .await?;
        let rows = client
            .query(&stmt, &[&email, &access_token, &refresh_token])
            .await?;
        Ok(Self {
            id: Some(rows.first().unwrap().get(0)),
            email: rows.first().unwrap().get(1),
            access_token: rows.first().unwrap().get(2),
            refresh_token: rows.first().unwrap().get(3),
        })
    }

    #[allow(unused)]
    pub async fn update_tokens(
        &self,
        client: &deadpool_postgres::Client,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<()> {
        let stmt = client
            .prepare("UPDATE users SET access_token = $1, refresh_token = $2 WHERE email = $3")
            .await?;
        client
            .execute(&stmt, &[&access_token, &refresh_token, &self.email])
            .await?;
        Ok(())
    }
}

/// Creates a Deadpool configuration from a database URL.
fn create_deadpool_config_from_url(url: &str) -> std::result::Result<Config, url::ParseError> {
    let parsed_url = Url::parse(url)?;

    let config = Config {
        user: Some(parsed_url.username().to_owned()),
        password: parsed_url.password().map(ToString::to_string),
        host: Some(parsed_url.host_str().unwrap().to_owned()),
        port: Some(parsed_url.port().unwrap_or(5432)),
        dbname: Some(
            parsed_url
                .path_segments()
                .map(|mut segments| segments.next().unwrap().to_owned())
                .unwrap(),
        ),
        ..Default::default()
    };

    // TODO
    // for (key, value) in parsed_url.query_pairs() {
    //     config.options.push((key.to_owned(), value.to_owned()));
    // }

    Ok(config)
}
