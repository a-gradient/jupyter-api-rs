use std::{io::IsTerminal, path::PathBuf};

use anyhow::{anyhow, Context};
use clap::{value_parser, ArgAction, Args, ValueHint};
use crossterm::terminal;
use futures_util::StreamExt;
use jupyter_shell::{
  api::jupyter::JupyterApi,
  services::terminal::{InputMessage, OutputMessage, TerminalError, TerminalService, TerminalSplit},
};
use reqwest::Url;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  sync::mpsc,
};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, info, warn};

use crate::cli::{DEFAULT_JUPYTER_URL, TokenArgs};

const STDIN_CHANNEL_CAPACITY: usize = 32;
const RESIZE_CHANNEL_CAPACITY: usize = 8;

#[derive(Args, Debug)]
#[command(about = "Open an interactive shell against a Jupyter terminal")]
pub struct SshArgs {
  #[arg(long = "endpoint", value_name = "JUPYTER_URL", default_value = DEFAULT_JUPYTER_URL, help = "Full Jupyter URL (supports ?token=<value>)")]
  endpoint_url: Url,
  #[arg(long, value_name = "TOKEN", env = "JUPYTER_TOKEN", help = "Override the token provided in the Jupyter URL")]
  token: Option<String>,
  #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, env = "JUPYTER_TOKEN_FILE", conflicts_with = "token", help = "Load the API token from a file")]
  token_file: Option<PathBuf>,

  #[arg(long = "timeout", value_name = "SECONDS", env = "JUPYTER_SHELL_SSH_HTTP_TIMEOUT", value_parser = value_parser!(u64).range(1..=3600), help = "HTTP client timeout in seconds")]
  http_timeout_secs: Option<u64>,
  #[arg(long, action = ArgAction::SetTrue, env = "JUPYTER_SHELL_SSH_ACCEPT_INVALID_CERTS", help = "Disable TLS certificate verification for the Jupyter endpoint")]
  accept_invalid_certs: bool,
  #[arg(long, value_name = "PATH", env = "JUPYTER_SHELL_API_BASE_PATH", help = "Override the API base path instead of auto-detecting it")]
  api_base_path: Option<String>,

  #[arg(long, value_name = "NAME", help = "Attach to an existing terminal instead of creating a new one")]
  terminal: Option<String>,
  #[arg(long, action = ArgAction::SetTrue, help = "Leave the terminal running instead of deleting it on exit")]
  keep_terminal: bool,
  #[arg(long, action = ArgAction::SetTrue, help = "Do not place the local TTY into raw mode")]
  no_raw: bool,
}

pub(crate) async fn run(args: SshArgs) -> anyhow::Result<()> {
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

  let (terminal_name, created_terminal) = match args.terminal.clone() {
    Some(name) => {
      client
        .get_terminal(&name)
        .await
        .with_context(|| format!("failed to fetch terminal {name}"))?;
      (name, false)
    }
    None => {
      let terminal = client
        .create_terminal(None)
        .await
        .context("failed to create a terminal session")?;
      (terminal.name, true)
    }
  };

  info!(%base_url, terminal = %terminal_name, created = created_terminal, "Opening SSH session against Jupyter");
  let _raw_guard = RawModeGuard::new(!args.no_raw)?;

  let service = TerminalService::connect(client, &terminal_name)
    .await
    .with_context(|| format!("failed to connect to terminal {terminal_name}"))?;
  let TerminalSplit {
    client,
    name,
    mut sink,
    mut stream,
  } = service.split();

  if let Some((cols, rows)) = current_terminal_size() {
    sink.send_message(InputMessage::Resize { cols, rows })
      .await
      .map_err(to_anyhow)?;
  }

  let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(STDIN_CHANNEL_CAPACITY);
  let stdin_task = tokio::spawn(read_stdin(stdin_tx));

  #[cfg(unix)]
  let mut resize_rx: ResizeChannel = spawn_resize_listener()?;
  #[cfg(not(unix))]
  let mut resize_rx: ResizeChannel = ();

  let mut stdout = tokio::io::stdout();
  let mut stdin_closed = false;
  let mut ws_closed = false;

  loop {
    if stdin_closed || ws_closed {
      break;
    }

    tokio::select! {
      biased;

      msg = stream.next(), if !ws_closed => {
        match msg {
          Some(Ok(OutputMessage::Stdout(data))) => {
            stdout.write_all(data.as_bytes()).await.with_context(|| "failed to write to stdout")?;
            stdout.flush().await.with_context(|| "failed to flush stdout")?;
          }
          Some(Ok(OutputMessage::Init {})) => {
            // Ignore init messages
          }
          Some(Err(err)) => {
            ws_closed = true;
            warn!(error = %err, "WebSocket error received from terminal");
          }
          Some(Ok(OutputMessage::Disconnect(_))) | None => {
            ws_closed = true;
            debug!("WebSocket stream closed by server");
          }
        }
      }

      maybe_chunk = stdin_rx.recv(), if !stdin_closed => {
        match maybe_chunk {
          Some(chunk) => {
            if chunk.is_empty() {
              continue;
            }
            sink.send_message(InputMessage::Stdin(String::from_utf8_lossy(&chunk).into_owned()))
              .await
              .with_context(|| "failed to send stdin data to terminal")?;
          }
          None => {
            stdin_closed = true;
            let _ = sink.send_message(InputMessage::Stdin("\u{0004}".to_string())).await;
          }
        }
      }

      maybe_resize = recv_resize(&mut resize_rx) => {
        if let Some((cols, rows)) = maybe_resize {
          sink.send_message(InputMessage::Resize { cols, rows })
            .await
            .with_context(|| "failed to send resize message to terminal")?;
        }
      }
    }
  }
  stdin_task.abort();
  drop(_raw_guard);
  info!("SSH session ended since {}", if ws_closed { "the server closed the connection" } else { "stdin was closed" });

  // if let Err(err) = stdin_task.await {
  //   warn!(error = %err, "stdin reader task failed");
  // }

  if created_terminal && !args.keep_terminal {
    client
      .delete_terminal(&name)
      .await
      .with_context(|| format!("failed to delete terminal {name}"))?;
    info!(terminal = %name, "Deleted Jupyter terminal after session");
  }

  Ok(())
}

async fn read_stdin(tx: mpsc::Sender<Vec<u8>>) {
  let mut stdin = tokio::io::stdin();
  let mut buf = [0u8; 1024];
  loop {
    match stdin.read(&mut buf).await {
      Ok(0) => break,
      Ok(len) => {
        if tx.send(buf[..len].to_vec()).await.is_err() {
          break;
        }
      }
      Err(err) => {
        warn!(error = %err, "failed to read stdin");
        break;
      }
    }
  }
}

fn to_anyhow(err: TerminalError) -> anyhow::Error {
  anyhow!("terminal error: {err:?}")
}

fn current_terminal_size() -> Option<(u16, u16)> {
  terminal::size().ok()
}

#[cfg(unix)]
type ResizeChannel = mpsc::Receiver<(u16, u16)>;
#[cfg(not(unix))]
type ResizeChannel = ();

#[cfg(unix)]
async fn recv_resize(rx: &mut ResizeChannel) -> Option<(u16, u16)> {
  rx.recv().await
}

#[cfg(not(unix))]
async fn recv_resize(_: &mut ResizeChannel) -> Option<(u16, u16)> {
  future::pending().await
}

struct RawModeGuard {
  enabled: bool,
}

impl RawModeGuard {
  fn new(enable: bool) -> anyhow::Result<Self> {
    if !enable {
      return Ok(Self { enabled: false });
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
      debug!("stdin or stdout is not a tty; skipping raw mode");
      return Ok(Self { enabled: false });
    }
    terminal::enable_raw_mode().context("failed to enable raw terminal mode")?;
    Ok(Self { enabled: true })
  }
}

impl Drop for RawModeGuard {
  fn drop(&mut self) {
    if self.enabled {
      if let Err(err) = terminal::disable_raw_mode() {
        warn!(error = %err, "failed to restore terminal mode");
      }
    }
  }
}

#[cfg(unix)]
fn spawn_resize_listener() -> anyhow::Result<ResizeChannel> {
  let (tx, rx) = mpsc::channel::<(u16, u16)>(RESIZE_CHANNEL_CAPACITY);
  let mut sig = signal(SignalKind::window_change()).context("failed to watch SIGWINCH")?;
  tokio::spawn(async move {
    while sig.recv().await.is_some() {
      if let Some(size) = current_terminal_size() {
        if tx.send(size).await.is_err() {
          break;
        }
      }
    }
  });
  Ok(rx)
}
