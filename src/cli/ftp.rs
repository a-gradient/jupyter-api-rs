use std::{
  net::SocketAddr,
  path::PathBuf,
  sync::Arc,
};

use clap::{value_parser, ArgAction, Args, ValueHint};
use jupyter_shell::{fs::FsService, ftp};
use reqwest::Url;
use tracing::info;

use crate::cli::{DEFAULT_JUPYTER_URL, TokenArgs};

const FTP_BIND_ADDR: &str = "0.0.0.0:8021";

pub(crate) async fn run(args: FtpArgs) -> anyhow::Result<()> {
  let token_args = TokenArgs {
    endpoint_url: args.endpoint_url,
    token: args.token,
    token_file: args.token_file,
    api_base_path: args.api_base_path,
    http_timeout_secs: args.http_timeout_secs,
    accept_invalid_certs: args.accept_invalid_certs,
  };
  let base_url = token_args.derive_base_url()?;

  let client = token_args.build_client()?;

  let fs = FsService::new(Arc::new(client));
  let server = ftp::server_builder(fs).build()?;

  let bind = if let Some(port) = args.bind_port {
    SocketAddr::new(args.bind.ip(), port)
  } else {
    args.bind
  };
  info!(
    %base_url,
    %bind,
    tls_verification_disabled = args.accept_invalid_certs,
    "Serving Jupyter over FTP"
  );

  server.listen(bind.to_string()).await?;
  info!("FTP server listener exited");
  Ok(())
}

#[derive(Args, Debug)]
#[command(about = "Expose a Jupyter deployment over FTP")]
pub struct FtpArgs {
  #[arg(value_name = "JUPYTER_URL", default_value = DEFAULT_JUPYTER_URL, help = "Full Jupyter URL (supports ?token=<value>)")]
  endpoint_url: Url,
  #[arg(long, value_name = "TOKEN", env = "JUPYTER_TOKEN", help = "Override the token provided in the Jupyter URL")]
  token: Option<String>,
  #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, env = "JUPYTER_TOKEN_FILE", conflicts_with = "token", help = "Load the API token from a file")]
  token_file: Option<PathBuf>,

  #[arg(long = "timeout", value_name = "SECONDS", env = "JUPYTER_SHELL_HTTP_TIMEOUT", value_parser = value_parser!(u64).range(1..=3600), help = "HTTP client timeout in seconds")]
  http_timeout_secs: Option<u64>,
  #[arg(long, action = ArgAction::SetTrue, env = "JUPYTER_SHELL_ACCEPT_INVALID_CERTS", help = "Disable TLS certificate verification for the Jupyter endpoint")]
  accept_invalid_certs: bool,
  #[arg(long, value_name = "PATH", env = "JUPYTER_SHELL_API_BASE_PATH", help = "Override the API base path instead of auto-detecting it")]
  api_base_path: Option<String>,

  #[arg(long, value_name = "IP:PORT", env = "JUPYTER_SHELL_BIND_ADDR", default_value = FTP_BIND_ADDR, help = "Address to bind the FTP server to")]
  bind: SocketAddr,
  #[arg(short = 'p', long, value_name = "PORT", env = "JUPYTER_SHELL_BIND_PORT", help = "Port to bind the FTP server to (overrides --bind)")]
  bind_port: Option<u16>,
}
