use anyhow::Context;
use clap::Parser;

pub mod cli;

fn init_tracing() -> anyhow::Result<()> {
  use tracing_subscriber::EnvFilter;

  let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug,libunftp=warn,reqwest=info,hyper_util=warn"));
  tracing_subscriber::fmt()
    .with_env_filter(filter)
    // .with_target(false)
    .compact()
    .try_init()
    .ok();
  Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  init_tracing().context("failed to initialize logging")?;
  let cli = cli::ftp::FtpArgs::parse();
  cli::ftp::run(cli).await
    .context("FTP server exited with an error")?;
  Ok(())
}
