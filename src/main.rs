use std::{
  fs,
  net::SocketAddr,
  path::PathBuf,
  sync::Arc,
  time::Duration,
};

use anyhow::{bail, Context};
use clap::{value_parser, ArgAction, Parser, ValueHint};
use jupyter_shell::{api::client::JupyterRestClient, fs::FsService, ftp};
use reqwest::Url;

const FTP_BIND_ADDR: &str = "0.0.0.0:8021";
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 30;
const APP_USER_AGENT: &str = concat!("jupyter-shell/", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();
  let base_url = derive_base_url(&cli)?;
  let token = resolve_token(&cli)?;

  let mut builder = JupyterRestClient::builder(base_url.as_str())?
    .timeout(Duration::from_secs(cli.http_timeout_secs))
    .user_agent(APP_USER_AGENT);

  if cli.accept_invalid_certs {
    builder = builder.danger_accept_invalid_certs(true);
  }

  builder = builder.token(&token)?;

  let rest = Arc::new(
    builder
      .build()
      .context("failed to build Jupyter REST client")?,
  );

  let fs = Arc::new(FsService::new(rest));
  let server = ftp::server_builder(fs).build()?;

  let bind = if let Some(port) = cli.bind_port {
    SocketAddr::new(cli.bind.ip(), port)
  } else {
    cli.bind
  };
  println!(
    "Serving {} over FTP on {} (TLS verification: {})",
    base_url,
    bind,
    if cli.accept_invalid_certs { "disabled" } else { "enabled" }
  );

  server.listen(bind.to_string()).await?;
  Ok(())
}

fn derive_base_url(cli: &Cli) -> anyhow::Result<Url> {
  let mut url = cli.endpoint_url.clone();
  url.set_query(None);

  let normalized_path = match cli.api_base_path.as_deref() {
    Some(custom) => normalize_path(custom),
    None => sanitize_base_path(url.path()),
  };

  url.set_path(&normalized_path);
  Ok(url)
}

fn resolve_token(cli: &Cli) -> anyhow::Result<String> {
  if let Some(path) = &cli.token_file {
    let contents = fs::read_to_string(path)
      .with_context(|| format!("failed to read token file {}", path.display()))?;
    let token = contents.trim().to_string();
    if token.is_empty() {
      bail!("token file {} was empty", path.display());
    }
    return Ok(token);
  }

  if let Some(token) = cli
    .token
    .as_ref()
    .map(|value| value.trim())
    .filter(|value| !value.is_empty())
  {
    return Ok(token.to_string());
  }

  if let Some(token) = extract_token_from_url(&cli.endpoint_url) {
    return Ok(token);
  }

  bail!("no API token supplied; use --token, --token-file, or append ?token=<value> to the URL");
}

fn extract_token_from_url(url: &Url) -> Option<String> {
  url
    .query_pairs()
    .find_map(|(key, value)| (key == "token").then(|| value.into_owned()))
    .and_then(|token| {
      let trimmed = token.trim().to_string();
      (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn sanitize_base_path(path: &str) -> String {
  let mut kept = Vec::new();
  for segment in path.trim_start_matches('/').split('/') {
    if segment.is_empty() {
      continue;
    }
    if is_frontend_route(segment) {
      break;
    }
    kept.push(segment);
  }

  if kept.is_empty() {
    "/".into()
  } else {
    normalize_path(&kept.join("/"))
  }
}

fn normalize_path(path: &str) -> String {
  let trimmed = path.trim_matches('/');
  if trimmed.is_empty() {
    "/".into()
  } else {
    let mut normalized = String::from("/");
    normalized.push_str(trimmed);
    if !normalized.ends_with('/') {
      normalized.push('/');
    }
    normalized
  }
}

fn is_frontend_route(segment: &str) -> bool {
  matches!(segment, "lab" | "tree" | "notebooks" | "voila" | "retro" | "console")
}

#[derive(Parser, Debug)]
#[command(name = "jupyter_shell", version, about = "Expose a Jupyter deployment over FTP")]
struct Cli {
  #[arg(value_name = "JUPYTER_URL", help = "Full Jupyter URL (supports ?token=<value>)")]
  endpoint_url: Url,
  #[arg(long, value_name = "TOKEN", env = "JUPYTER_TOKEN", help = "Override the token provided in the Jupyter URL")]
  token: Option<String>,
  #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, env = "JUPYTER_TOKEN_FILE", conflicts_with = "token", help = "Load the API token from a file")]
  token_file: Option<PathBuf>,
  #[arg(long, value_name = "IP:PORT", env = "JUPYTER_SHELL_BIND_ADDR", default_value = FTP_BIND_ADDR, help = "Address to bind the FTP server to")]
  bind: SocketAddr,
  #[arg(short = 'p', long, value_name = "PORT", env = "JUPYTER_SHELL_BIND_PORT", help = "Port to bind the FTP server to (overrides --bind)")]
  bind_port: Option<u16>,
  #[arg(long, value_name = "SECONDS", env = "JUPYTER_SHELL_HTTP_TIMEOUT", default_value_t = DEFAULT_HTTP_TIMEOUT_SECS, value_parser = value_parser!(u64).range(1..=300), help = "HTTP client timeout in seconds")]
  http_timeout_secs: u64,
  #[arg(long, action = ArgAction::SetTrue, env = "JUPYTER_SHELL_ACCEPT_INVALID_CERTS", help = "Disable TLS certificate verification for the Jupyter endpoint")]
  accept_invalid_certs: bool,
  #[arg(long, value_name = "PATH", env = "JUPYTER_SHELL_API_BASE_PATH", help = "Override the API base path instead of auto-detecting it")]
  api_base_path: Option<String>,
}
