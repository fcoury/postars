use deadpool_postgres::{Config, CreatePoolError, Pool, PoolError, Runtime};
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

pub async fn get_or_create_user(client: &deadpool_postgres::Client, email: &str) -> Result<i32> {
    let stmt = client
        .prepare("SELECT id, email FROM users WHERE email = $1")
        .await?;
    let rows = client.query(&stmt, &[&email]).await?;

    if let Some(row) = rows.first() {
        Ok(row.get(0))
    } else {
        let stmt = client
            .prepare("INSERT INTO users (email) VALUES ($1) RETURNING id, email")
            .await?;
        let rows = client.query(&stmt, &[&email]).await?;
        Ok(rows[0].get(0))
    }
}

pub async fn insert_or_replace_token(
    client: &deadpool_postgres::Client,
    user_email: &str,
    access_token: &str,
    refresh_token: &str,
) -> Result<()> {
    let user_id = get_or_create_user(client, user_email).await?;

    let stmt = client
        .prepare("SELECT id FROM tokens WHERE user_id = $1")
        .await?;
    let rows = client.query(&stmt, &[&user_id]).await?;

    let stmt = if rows.first().is_some() {
        client
                .prepare("UPDATE tokens SET access_token = $2, refresh_token = $3, updated_at = NOW() WHERE id = $1")
                .await?
    } else {
        client
            .prepare(
                "INSERT INTO tokens (user_id, access_token, refresh_token) VALUES ($1, $2, $3)",
            )
            .await?
    };

    client
        .execute(&stmt, &[&user_id, &access_token, &refresh_token])
        .await?;
    Ok(())
}
