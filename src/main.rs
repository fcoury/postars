mod api;
mod auth;
mod graph;

use std::net::SocketAddr;

use api::Server;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
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
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
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
        Command::Serve { bind } => Ok(serve(bind).await?),
        Command::Auth { command } => match command {
            AuthCommand::Set => auth().await,
            AuthCommand::Get => {
                let token: Token = confy::load("postars", None)?;
                let json = serde_json::to_string_pretty(&token)?;
                println!("{}", json);
                Ok(())
            }
        },
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

async fn serve(bind: SocketAddr) -> anyhow::Result<()> {
    Server::new(bind).start().await
}

async fn auth() -> anyhow::Result<()> {
    let token = tokio::task::spawn_blocking(auth::auth).await??;
    confy::store("postars", None, token)?;
    println!("Auth saved.");

    Ok(())
}
