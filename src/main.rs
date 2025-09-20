use clap::Parser;
use env_sync::sync::{EnvSync, EnvSyncOptions};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
  name = "env-sync",
  about = "Easily update your local env file with a git-trackable file",
  version,
  author
)]
struct Cli {
  /// Path to the local .env file
  #[arg(short, long)]
  local: Option<PathBuf>,

  /// Path to the template file
  #[arg(short, long, default_value = ".env.template")]
  template: PathBuf,

  /// Verbose output (-v for verbose, -vv for very verbose)
  #[arg(short, long, action = clap::ArgAction::Count)]
  verbose: u8,
}

fn setup_tracing(verbose: u8) {
  use tracing_subscriber::fmt;
  use tracing_subscriber::prelude::*;

  let log_level = match verbose {
    1 => "debug",
    2 => "trace",
    _ => "info",
  };

  tracing_subscriber::registry()
    .with(fmt::layer())
    .with(tracing_subscriber::EnvFilter::new(
      std::env::var("RUST_LOG").unwrap_or_else(|_| log_level.into()),
    ))
    .init();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  setup_tracing(cli.verbose);

  let options = EnvSyncOptions {
    local_file: cli.local,
    template_file: cli.template,
  };

  EnvSync::sync_with_options(options)?;

  Ok(())
}
