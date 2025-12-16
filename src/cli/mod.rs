use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use reqwest::Url;
use tracing::info;

pub mod ftp;
pub mod scp;

#[derive(Parser, Debug)]
#[command(name = "jupyter_shell", version, about = "Expose a Jupyter deployment over remote file protocols")]
pub struct Cli {
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  #[command(about = "Expose a Jupyter deployment over FTP")]
  Ftp(ftp::FtpArgs),
  #[command(about = "Expose a Jupyter deployment over SCP")]
  Scp(scp::ScpArgs),
}

#[derive(Debug)]
pub struct TokenArgs {
  endpoint_url: Url,
  token: Option<String>,
  token_file: Option<PathBuf>,
  api_base_path: Option<String>,
}

impl TokenArgs {
  fn derive_base_url(&self) -> anyhow::Result<Url> {
    let mut url = self.endpoint_url.clone();
    url.set_query(None);

    let normalized_path = match self.api_base_path.as_deref() {
      Some(custom) => normalize_path(custom),
      None => sanitize_base_path(url.path()),
    };

    url.set_path(&normalized_path);
    Ok(url)
  }

  fn resolve_token(&self) -> anyhow::Result<String> {
    if let Some(path) = &self.token_file {
      let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read token file {}", path.display()))?;
      let token = contents.trim().to_string();
      if token.is_empty() {
        bail!("token file {} was empty", path.display());
      }
      info!("token: {}", token[0..std::cmp::min(4, token.len())].to_string() + "****");
      return Ok(token);
    }

    if let Some(token) = self
      .token
      .as_ref()
      .map(|value| value.trim())
      .filter(|value| !value.is_empty())
    {
      return Ok(token.to_string());
    }

    if let Some(token) = extract_token_from_url(&self.endpoint_url) {
      return Ok(token);
    }

    bail!("no API token supplied; use --token, --token-file, or append ?token=<value> to the URL");
  }
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
