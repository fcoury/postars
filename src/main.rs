mod api;
mod auth;
mod database;
mod graph;
mod index;

use std::net::SocketAddr;

use api::Server;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use postgres_queue::{initialize_database, TaskRegistry};
use tracing::info;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::auth::Token;

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    Serve {
        /// The address to bind to
        #[arg(short, long, default_value = "127.0.0.1:3001")]
        bind: SocketAddr,

        #[arg(short, long, env = "DATABASE_URL")]
        database_url: String,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Workers {
        #[arg(short, long, default_value = "10")]
        num_workers: usize,

        #[arg(short, long, env = "DATABASE_URL")]
        database_url: String,
    },
    Enqueue {
        #[arg(short, long, env = "DATABASE_URL")]
        database_url: String,

        task_name: String,

        task_data: Option<String>,
    },
}

#[derive(Subcommand, Clone, Debug)]
enum AuthCommand {
    Get,
    Set,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    setup_logging(&cli)?;

    match cli.command {
        Command::Serve { bind, database_url } => Ok(serve(bind, database_url).await?),
        Command::Auth { command } => match command {
            AuthCommand::Set => auth().await,
            AuthCommand::Get => {
                let token: Token = confy::load("postars", None)?;
                let json = serde_json::to_string_pretty(&token)?;
                println!("{}", json);
                Ok(())
            }
        },
        Command::Workers {
            num_workers,
            database_url,
        } => {
            info!("Starting {} workers...", num_workers);

            let pool = postgres_queue::connect(&database_url)
                .await
                .expect("Failed to connect to the database");

            initialize_database(&pool)
                .await
                .expect("Failed to initialize database");

            let mut registry = TaskRegistry::new();
            registry.register_task("full_index".to_string(), index::full_index_handler_sync);

            let tasks = registry
                .run(&pool, num_workers)
                .await
                .expect("Failed to run tasks");

            info!("Running {} tasks", tasks.len());

            // Wait for all tasks to complete
            for task in tasks {
                task.await.expect("Task failed");
            }

            Ok(())
        }
        Command::Enqueue {
            database_url,
            task_name,
            task_data,
        } => {
            let pool = postgres_queue::connect(&database_url)
                .await
                .expect("Failed to connect to the database");

            initialize_database(&pool)
                .await
                .expect("Failed to initialize database");

            let task_data = serde_json::from_str(&task_data.unwrap_or_else(|| "{}".to_string()))?;

            let task_id = postgres_queue::enqueue(
                &pool.get().await.unwrap(),
                &task_name,
                task_data,
                chrono::Utc::now(), // Run the task immediately
                None,               // No interval
            )
            .await
            .expect("Failed to enqueue task");
            println!("Enqueued task with ID: {}", task_id);

            Ok(())
        }
    }
}

fn setup_logging(cli: &Cli) -> anyhow::Result<()> {
    let log_level = if cli.debug {
        "debug,hyper=info"
    } else {
        "info"
    };

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(log_level))?,
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    Ok(())
}

async fn serve(bind: SocketAddr, database_url: String) -> anyhow::Result<()> {
    Server::new(bind, database_url).start().await
}

async fn auth() -> anyhow::Result<()> {
    let token = tokio::task::spawn_blocking(auth::auth).await??;
    confy::store("postars", None, token)?;
    println!("Auth saved.");

    Ok(())
}
