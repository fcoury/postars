mod api;
mod auth;

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
    Serve,
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
        Command::Serve => Ok(serve().await?),
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

async fn serve() -> anyhow::Result<()> {
    Server::new("127.0.0.1:3001".parse()?).start().await
}

async fn auth() -> anyhow::Result<()> {
    let token = tokio::task::spawn_blocking(auth::auth).await??;
    confy::store("postars", None, token)?;
    println!("Auth saved.");

    Ok(())
}
