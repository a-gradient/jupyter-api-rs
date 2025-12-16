use std::{
  net::SocketAddr,
  path::PathBuf,
};

use anyhow::bail;
use clap::{value_parser, ArgAction, Args, ValueHint};
use reqwest::Url;

const SCP_BIND_ADDR: &str = "0.0.0.0:8022";

#[derive(Args, Debug)]
#[command(about = "Expose a Jupyter deployment over SCP")]
pub struct ScpArgs {
  #[arg(value_name = "JUPYTER_URL", help = "Full Jupyter URL (supports ?token=<value>)")]
  pub(crate) endpoint_url: Url,
  #[arg(long, value_name = "TOKEN", env = "JUPYTER_TOKEN", help = "Override the token provided in the Jupyter URL")]
  pub(crate) token: Option<String>,
  #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, env = "JUPYTER_TOKEN_FILE", conflicts_with = "token", help = "Load the API token from a file")]
  pub(crate) token_file: Option<PathBuf>,
  #[arg(long, value_name = "IP:PORT", env = "JUPYTER_SHELL_SCP_BIND_ADDR", default_value = SCP_BIND_ADDR, help = "Address to bind the SCP server to")]
  pub(crate) bind: SocketAddr,
  #[arg(short = 'p', long, value_name = "PORT", env = "JUPYTER_SHELL_SCP_BIND_PORT", help = "Port to bind the SCP server to (overrides --bind)")]
  pub(crate) bind_port: Option<u16>,
  #[arg(long = "timeout", value_name = "SECONDS", env = "JUPYTER_SHELL_SCP_HTTP_TIMEOUT", value_parser = value_parser!(u64).range(1..=3600), help = "HTTP client timeout in seconds")]
  pub(crate) http_timeout_secs: Option<u64>,
  #[arg(long, action = ArgAction::SetTrue, env = "JUPYTER_SHELL_SCP_ACCEPT_INVALID_CERTS", help = "Disable TLS certificate verification for the Jupyter endpoint")]
  pub(crate) accept_invalid_certs: bool,
  #[arg(long, value_name = "PATH", env = "JUPYTER_SHELL_API_BASE_PATH", help = "Override the API base path instead of auto-detecting it")]
  pub(crate) api_base_path: Option<String>,
}

pub(crate) async fn run(_args: ScpArgs) -> anyhow::Result<()> {
  bail!("SCP subcommand is not implemented yet");
}
