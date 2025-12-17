use std::{
  io::ErrorKind,
  path::{Path, PathBuf},
  sync::Arc,
};

use anyhow::{anyhow, bail, Context};
use clap::{value_parser, ArgAction, Args, ValueHint};
use jupyter_shell::{
  api::client::ClientError,
  fs::{Entry, FsError, FsService},
};
use reqwest::{StatusCode, Url};
use tokio::fs;
use tracing::{debug, info, warn};

use crate::cli::{DEFAULT_JUPYTER_URL, TokenArgs};

#[derive(Args, Debug)]
#[command(about = "Expose a Jupyter deployment over SCP")]
pub struct ScpArgs {
  #[arg(long = "endpoint", value_name = "JUPYTER_URL", default_value = DEFAULT_JUPYTER_URL, help = "Full Jupyter URL (supports ?token=<value>)")]
  endpoint_url: Url,
  #[arg(long, value_name = "TOKEN", env = "JUPYTER_TOKEN", help = "Override the token provided in the Jupyter URL")]
  token: Option<String>,
  #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, env = "JUPYTER_TOKEN_FILE", conflicts_with = "token", help = "Load the API token from a file")]
  token_file: Option<PathBuf>,

  #[arg(long = "timeout", value_name = "SECONDS", env = "JUPYTER_SHELL_SCP_HTTP_TIMEOUT", value_parser = value_parser!(u64).range(1..=3600), help = "HTTP client timeout in seconds")]
  http_timeout_secs: Option<u64>,
  #[arg(long, action = ArgAction::SetTrue, env = "JUPYTER_SHELL_SCP_ACCEPT_INVALID_CERTS", help = "Disable TLS certificate verification for the Jupyter endpoint")]
  accept_invalid_certs: bool,
  #[arg(long, value_name = "PATH", env = "JUPYTER_SHELL_API_BASE_PATH", help = "Override the API base path instead of auto-detecting it")]
  api_base_path: Option<String>,

  #[arg(value_name = "PATH", num_args = 2.., value_hint = ValueHint::AnyPath, help = "Source and destination specifiers that follow scp syntax (e.g. ./file or user@remote:/dst)")]
  paths: Vec<String>,
  #[arg(short = 'r', long, action = ArgAction::SetTrue, help = "Recursively copy entire directories")]
  recursive: bool,
}

pub(crate) async fn run(args: ScpArgs) -> anyhow::Result<()> {
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
  let (source_ops, dest_op) = parse_operands(&args.paths)?;
  let plan = determine_transfer_plan(&base_url, source_ops, dest_op)?;

  info!(mode = plan.label(), source_count = plan.source_count(), recursive = args.recursive, "Starting SCP transfer");
  match plan {
    TransferPlan::Upload { sources, destination } => {
      upload_paths(&fs, &sources, &destination, args.recursive).await?;
    }
    TransferPlan::Download { sources, destination } => {
      download_paths(&fs, &sources, &destination, args.recursive).await?;
    }
  }
  info!("SCP transfer completed");
  Ok(())
}

#[derive(Debug)]
enum TransferPlan {
  Upload {
    sources: Vec<LocalOperand>,
    destination: RemoteOperand,
  },
  Download {
    sources: Vec<RemoteOperand>,
    destination: LocalOperand,
  },
}

impl TransferPlan {
  fn label(&self) -> &'static str {
    match self {
      TransferPlan::Upload { .. } => "upload",
      TransferPlan::Download { .. } => "download",
    }
  }

  fn source_count(&self) -> usize {
    match self {
      TransferPlan::Upload { sources, .. } => sources.len(),
      TransferPlan::Download { sources, .. } => sources.len(),
    }
  }
}

#[derive(Debug, Clone)]
struct LocalOperand {
  raw: String,
  path: PathBuf,
  explicit_dir: bool,
}

impl LocalOperand {
  fn basename(&self) -> anyhow::Result<String> {
    self
      .path
      .file_name()
      .map(|os| os.to_string_lossy().into_owned())
      .ok_or_else(|| anyhow!("unable to infer a filename for {}", self.raw))
  }
}

#[derive(Debug, Clone)]
struct RemoteOperand {
  raw: String,
  host: Option<String>,
  normalized: String,
  explicit_dir: bool,
}

#[derive(Debug)]
enum Operand {
  Local(LocalOperand),
  Remote(RemoteOperand),
}

fn parse_operands(values: &[String]) -> anyhow::Result<(Vec<Operand>, Operand)> {
  if values.len() < 2 {
    bail!("expected at least one source and one destination operand");
  }
  let mut operands = Vec::with_capacity(values.len() - 1);
  for raw in &values[..values.len() - 1] {
    operands.push(parse_operand(raw)?);
  }
  let destination = parse_operand(values.last().unwrap())?;
  Ok((operands, destination))
}

fn parse_operand(raw: &str) -> anyhow::Result<Operand> {
  if let Some((target, path_fragment)) = split_remote_spec(raw) {
    let (_, host) = split_user_and_host(&target);
    let parsed_host = host.filter(|value| !value.is_empty()).map(|value| trim_ipv6_brackets(value).to_string());
    let explicit_dir = path_fragment.ends_with('/') || path_fragment.is_empty();
    let normalized = normalize_remote_path(&path_fragment);
    return Ok(Operand::Remote(RemoteOperand {
      raw: raw.to_string(),
      host: parsed_host,
      normalized,
      explicit_dir,
    }));
  }

  Ok(Operand::Local(LocalOperand {
    raw: raw.to_string(),
    path: PathBuf::from(raw),
    explicit_dir: raw.ends_with('/'),
  }))
}

fn split_remote_spec(raw: &str) -> Option<(String, String)> {
  let mut bracket_depth: usize = 0;
  for (idx, ch) in raw.char_indices() {
    match ch {
      '[' => bracket_depth += 1,
      ']' => bracket_depth = bracket_depth.saturating_sub(1),
      ':' if bracket_depth == 0 => {
        let (target, rest) = raw.split_at(idx);
        let remainder = rest.get(1..).unwrap_or("").to_string();
        return Some((target.to_string(), remainder));
      }
      _ => {}
    }
  }
  None
}

fn split_user_and_host(target: &str) -> (Option<&str>, Option<&str>) {
  if target.is_empty() {
    return (None, None);
  }
  match target.rsplit_once('@') {
    Some((user, host)) => (Some(user), Some(host)),
    None => (None, Some(target)),
  }
}

fn trim_ipv6_brackets(host: &str) -> &str {
  host
    .trim_start_matches('[')
    .trim_end_matches(']')
}

fn determine_transfer_plan(
  base_url: &Url,
  sources: Vec<Operand>,
  destination: Operand,
) -> anyhow::Result<TransferPlan> {
  if sources.is_empty() {
    bail!("no sources provided");
  }

  match destination {
    Operand::Remote(dest) => {
      ensure_host_alignment(base_url, &dest)?;
      let mut locals = Vec::new();
      for operand in sources {
        match operand {
          Operand::Local(local) => locals.push(local),
          Operand::Remote(remote) => {
            bail!(
              "remote source '{}' is not supported when destination is also remote",
              remote.raw
            );
          }
        }
      }
      Ok(TransferPlan::Upload {
        sources: locals,
        destination: dest,
      })
    }
    Operand::Local(dest) => {
      let mut remotes = Vec::new();
      for operand in sources {
        match operand {
          Operand::Remote(remote) => {
            ensure_host_alignment(base_url, &remote)?;
            remotes.push(remote);
          }
          Operand::Local(local) => {
            bail!(
              "source '{}' is local while destination '{}' is also local; provide at least one remote operand",
              local.raw,
              dest.raw
            );
          }
        }
      }
      Ok(TransferPlan::Download {
        sources: remotes,
        destination: dest,
      })
    }
  }
}

fn ensure_host_alignment(base_url: &Url, remote: &RemoteOperand) -> anyhow::Result<()> {
  let Some(expected) = base_url.host_str() else {
    return Ok(());
  };
  if let Some(host) = remote.host.as_deref() {
    if host == "remote" || host == "@remote" {
      return Ok(());
    }
    if !host.eq_ignore_ascii_case(expected) {
      warn!(
        "remote operand '{}' references host '{}' but the endpoint resolves to '{}'",
        remote.raw,
        host,
        expected
      );
    }
  }
  Ok(())
}

async fn upload_paths(
  fs: &FsService,
  sources: &[LocalOperand],
  dest: &RemoteOperand,
  recursive: bool,
) -> anyhow::Result<()> {
  if sources.is_empty() {
    bail!("no local sources were provided");
  }

  let dest_entry = fetch_remote_entry(fs, &dest.normalized).await?;
  let mut dest_is_dir = false;
  if let Some(entry) = &dest_entry {
    if entry.kind.is_directory() {
      dest_is_dir = true;
    } else if dest.explicit_dir {
      bail!("destination '{}' exists but is not a directory", dest.raw);
    }
  } else if dest.explicit_dir || sources.len() > 1 {
    dest_is_dir = true;
  }

  if sources.len() > 1 && !dest_is_dir {
    bail!("destination '{}' must be a directory when copying multiple sources", dest.raw);
  }

  if dest_is_dir && dest_entry.is_none() {
    ensure_remote_directory(fs, &dest.normalized).await?;
  }

  for source in sources {
    let metadata = fs::metadata(&source.path)
      .await
      .with_context(|| format!("failed to read metadata for {}", source.path.display()))?;
    let target_path = if dest_is_dir {
      let name = source.basename()?;
      join_remote_paths(&dest.normalized, &name)
    } else {
      dest.normalized.clone()
    };

    if metadata.is_dir() {
      if !recursive {
        bail!("{} is a directory (use --recursive to enable directory copies)", source.raw);
      }
      upload_directory(fs, &source.path, &target_path).await?;
    } else if metadata.is_file() {
      upload_file(fs, &source.path, &target_path).await?;
    } else {
      bail!("{} is neither a file nor a directory", source.raw);
    }
  }

  Ok(())
}

async fn download_paths(
  fs: &FsService,
  sources: &[RemoteOperand],
  dest: &LocalOperand,
  recursive: bool,
) -> anyhow::Result<()> {
  if sources.is_empty() {
    bail!("no remote sources were provided");
  }

  let dest_metadata = match fs::metadata(&dest.path).await {
    Ok(meta) => Some(meta),
    Err(err) if err.kind() == ErrorKind::NotFound => None,
    Err(err) => {
      return Err(err).with_context(|| format!("failed to read metadata for {}", dest.path.display()));
    }
  };

  if let Some(meta) = &dest_metadata {
    if dest.explicit_dir && !meta.is_dir() {
      bail!("destination '{}' exists but is not a directory", dest.raw);
    }
  }

  let mut dest_is_dir = dest_metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
  if dest_metadata.is_none() && (dest.explicit_dir || sources.len() > 1) {
    fs::create_dir_all(&dest.path)
      .await
      .with_context(|| format!("failed to create destination directory {}", dest.path.display()))?;
    dest_is_dir = true;
  }

  if sources.len() > 1 && !dest_is_dir {
    bail!("destination '{}' must be a directory when copying multiple sources", dest.raw);
  }

  for remote in sources {
    let entry = fetch_remote_entry(fs, &remote.normalized)
      .await?
      .ok_or_else(|| anyhow!("remote path '{}' does not exist", remote.raw))?;
    let mut target_path = dest.path.clone();
    if dest_is_dir {
      target_path.push(&entry.name);
    }
    if entry.kind.is_directory() && !recursive {
      bail!("{} is a directory (use --recursive to enable directory copies)", remote.raw);
    }
    download_entry(fs, entry, &remote.normalized, &target_path, recursive).await?;
  }

  Ok(())
}

async fn upload_directory(fs: &FsService, local_dir: &Path, remote_dir: &str) -> anyhow::Result<()> {
  let mut stack = vec![(local_dir.to_path_buf(), remote_dir.to_string())];
  while let Some((current_local, current_remote)) = stack.pop() {
    ensure_remote_directory(fs, &current_remote).await?;
    let mut entries = fs::read_dir(&current_local)
      .await
      .with_context(|| format!("failed to list directory {}", current_local.display()))?;
    while let Some(entry) = entries
      .next_entry()
      .await
      .with_context(|| format!("failed to iterate directory {}", current_local.display()))?
    {
      let path = entry.path();
      let name = entry.file_name();
      let child = name.to_string_lossy().into_owned();
      let remote_child = join_remote_paths(&current_remote, &child);
      let metadata = entry
        .metadata()
        .await
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
      if metadata.is_dir() {
        stack.push((path, remote_child));
      } else if metadata.is_file() {
        upload_file(fs, &path, &remote_child).await?;
      } else {
        bail!("{} is neither a file nor a directory", path.display());
      }
    }
  }
  Ok(())
}

async fn upload_file(fs: &FsService, local_path: &Path, remote_path: &str) -> anyhow::Result<()> {
  let bytes = fs::read(local_path)
    .await
    .with_context(|| format!("failed to read {}", local_path.display()))?;
  fs
    .upload(remote_path, &bytes)
    .await
    .with_context(|| format!("failed to upload {} to {}", local_path.display(), remote_path))?;
  debug!(local = %local_path.display(), remote = remote_path, bytes = bytes.len(), "Uploaded file");
  Ok(())
}

async fn download_entry(
  fs: &FsService,
  entry: Entry,
  remote_path: &str,
  local_path: &Path,
  recursive: bool,
) -> anyhow::Result<()> {
  if !entry.kind.is_directory() {
    return download_file(fs, remote_path, local_path).await;
  }
  if !recursive {
    bail!("{} is a directory (use --recursive to enable directory copies)", remote_path);
  }
  let mut stack = vec![(entry, remote_path.to_string(), local_path.to_path_buf())];
  while let Some((current_entry, current_remote, current_local)) = stack.pop() {
    if current_entry.kind.is_directory() {
      fs::create_dir_all(&current_local)
        .await
        .with_context(|| format!("failed to create directory {}", current_local.display()))?;
      let children = fs
        .ls(&current_remote)
        .await
        .with_context(|| format!("failed to list remote directory {}", current_remote))?;
      for child in children {
        let child_remote = join_remote_paths(&current_remote, &child.name);
        let child_local = current_local.join(&child.name);
        if child.kind.is_directory() {
          stack.push((child, child_remote, child_local));
        } else {
          download_file(fs, &child_remote, &child_local).await?;
        }
      }
    } else {
      download_file(fs, &current_remote, &current_local).await?;
    }
  }
  Ok(())
}

async fn download_file(fs: &FsService, remote_path: &str, local_path: &Path) -> anyhow::Result<()> {
  let file = fs
    .download(remote_path)
    .await
    .with_context(|| format!("failed to download {}", remote_path))?;
  if let Some(parent) = local_path.parent() {
    fs::create_dir_all(parent)
      .await
      .with_context(|| format!("failed to create parent directories for {}", local_path.display()))?;
  }
  fs::write(local_path, &file.bytes)
    .await
    .with_context(|| format!("failed to write {}", local_path.display()))?;
  debug!(remote = remote_path, local = %local_path.display(), bytes = file.bytes.len(), "Downloaded file");
  Ok(())
}

async fn ensure_remote_directory(fs: &FsService, path: &str) -> anyhow::Result<()> {
  if path == "/" {
    return Ok(());
  }
  match fetch_remote_entry(fs, path).await? {
    Some(entry) => {
      if !entry.kind.is_directory() {
        bail!("remote path '{}' exists but is not a directory", path);
      }
    }
    None => {
      fs
        .mkdir(path)
        .await
        .with_context(|| format!("failed to create remote directory {}", path))?;
    }
  }
  Ok(())
}

async fn fetch_remote_entry(fs: &FsService, path: &str) -> anyhow::Result<Option<Entry>> {
  match fs.metadata(path).await {
    Ok(entry) => Ok(Some(entry)),
    Err(FsError::Client(ClientError::Api { status, .. })) if status == StatusCode::NOT_FOUND => Ok(None),
    Err(err) => Err(err.into()),
  }
}

fn join_remote_paths(base: &str, child: &str) -> String {
  let child = child.trim_matches('/');
  if base == "/" {
    format!("/{}", child)
  } else {
    format!("{}/{}", base.trim_end_matches('/'), child)
  }
}

fn normalize_remote_path(path: &str) -> String {
  let mut components = Vec::new();
  for part in path.split('/') {
    match part {
      "" | "." => {}
      ".." => {
        components.pop();
      }
      other => components.push(other.to_string()),
    }
  }
  if components.is_empty() {
    "/".into()
  } else {
    format!("/{}", components.join("/"))
  }
}
