use deadpool_postgres::{Config, Pool, Runtime};
use url::Url;

#[derive(Clone)]
pub struct Database {
    pool: Pool,
}

impl Database {
    pub async fn new(database_url: String) -> anyhow::Result<Self> {
        let config = create_deadpool_config_from_url(&database_url)?;
        let pool = config.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)?;
        Ok(Self { pool })
    }

    pub async fn get(&self) -> anyhow::Result<deadpool_postgres::Client> {
        Ok(self.pool.get().await?)
    }
}

/// Creates a Deadpool configuration from a database URL.
fn create_deadpool_config_from_url(url: &str) -> Result<Config, url::ParseError> {
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
