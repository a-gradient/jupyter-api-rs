use std::{env, fmt, sync::Arc};

use jupyter_shell::{api::client::JupyterRestClient, fs::FsService, ftp};
use reqwest::Url;

const FTP_BIND_ADDR: &str = "0.0.0.0:8021";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = CliArgs::from_env()?;
  let rest = Arc::new(JupyterRestClient::with_token(args.base_url.as_str(), &args.token)?);
  let fs = Arc::new(FsService::new(rest));
  let server = ftp::server_builder(fs).build()?;

  println!(
    "Serving {} over FTP on {} (token elided)",
    args.base_url, FTP_BIND_ADDR
  );

  server.listen(FTP_BIND_ADDR).await?;
  Ok(())
}

struct CliArgs {
  base_url: Url,
  token: String,
}

impl CliArgs {
  fn from_env() -> Result<Self, CliError> {
    let raw = env::args().nth(1).ok_or(CliError::MissingUrl)?;
    Self::from_launch_url(&raw)
  }

  fn from_launch_url(raw: &str) -> Result<Self, CliError> {
    let mut url = Url::parse(raw).map_err(|err| CliError::InvalidUrl(err.to_string()))?;
    let token = url
      .query_pairs()
      .find(|(key, _)| key.eq_ignore_ascii_case("token"))
      .map(|(_, value)| value.into_owned())
      .ok_or(CliError::MissingToken)?;
    url.set_query(None);
    url.set_fragment(None);
    let sanitized_path = sanitize_base_path(url.path());
    url.set_path(&sanitized_path);
    Ok(Self { base_url: url, token })
  }
}

#[derive(Debug)]
enum CliError {
  MissingUrl,
  MissingToken,
  InvalidUrl(String),
}

impl fmt::Display for CliError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CliError::MissingUrl => write!(f, "usage: jupyter_shell <jupyter-url-with-token>"),
      CliError::MissingToken => write!(f, "the provided url does not contain a token query parameter"),
      CliError::InvalidUrl(err) => write!(f, "failed to parse url: {err}"),
    }
  }
}

impl std::error::Error for CliError {}

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
    let mut normalized = String::from("/");
    normalized.push_str(&kept.join("/"));
    if !normalized.ends_with('/') {
      normalized.push('/');
    }
    normalized
  }
}

fn is_frontend_route(segment: &str) -> bool {
  matches!(segment, "lab" | "tree" | "notebooks" | "voila" | "retro" | "console")
}
