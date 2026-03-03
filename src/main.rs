use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod client;
mod config;
mod model;
mod server;

#[derive(Parser)]
#[command(
    name = "status-upstream",
    version,
    about = "Status page upstream monitoring system"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the central monitoring server
    Server {
        /// Path to server configuration file
        #[arg(short, long, default_value = "config/server.toml")]
        config: PathBuf,
    },
    /// Run the client agent that performs checks and reports to the server
    Client {
        /// Path to client configuration file
        #[arg(short, long, default_value = "config/client.toml")]
        config: PathBuf,
        /// Run checks once and exit (used by server for local checks)
        #[arg(long)]
        once: bool,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            match cli.command {
                Commands::Server { config } => server::run(&config).await,
                Commands::Client { config, once } => client::run(&config, once).await,
            }
        })
}
