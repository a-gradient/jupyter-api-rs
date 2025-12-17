use futures_util::{
  SinkExt, Stream, StreamExt, stream::{SplitSink, SplitStream}
};
use reqwest_websocket::Message;
use serde_json::json;
use std::time::Duration;
use reqwest::StatusCode;

use crate::api::{client::{ClientError, JupyterLabClient}, jupyter::JupyterApi};

pub struct TerminalService {
  pub client: JupyterLabClient,
  pub name: String,
  pub ws: reqwest_websocket::WebSocket,
  pub buffer: Vec<String>,
}

pub struct TerminalSplit {
  pub client: JupyterLabClient,
  pub name: String,
  pub sink: TerminalInputSink,
  pub stream: TerminalOutputStream,
}

pub struct TerminalInputSink {
  pub sink: SplitSink<reqwest_websocket::WebSocket, Message>,
}

impl TerminalInputSink {
  pub async fn send_message(&mut self, input: InputMessage) -> Result<(), TerminalError> {
    let msg_value = serde_json::Value::try_from(input).map_err(TerminalError::Json)?;
    let msg_text = serde_json::to_string(&msg_value).map_err(TerminalError::Json)?;
    self.sink
      .send(Message::Text(msg_text))
      .await
      .map_err(TerminalError::WebSocket)
  }
}

pub struct TerminalOutputStream {
  pub stream: SplitStream<reqwest_websocket::WebSocket>,
}

impl Stream for TerminalOutputStream {
  type Item = Result<OutputMessage, TerminalError>;

  fn poll_next(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    match futures_util::ready!(self.stream.poll_next_unpin(cx)) {
      Some(Ok(Message::Text(text))) => {
        let msg_value: serde_json::Value =
          match serde_json::from_str(&text).map_err(TerminalError::Json) {
            Ok(v) => v,
            Err(e) => return std::task::Poll::Ready(Some(Err(e))),
          };
        let output_msg = match OutputMessage::try_from(msg_value).map_err(TerminalError::Json) {
          Ok(msg) => msg,
          Err(e) => return std::task::Poll::Ready(Some(Err(e))),
        };
        std::task::Poll::Ready(Some(Ok(output_msg)))
      },
      Some(Ok(_)) => std::task::Poll::Ready(None),
      Some(Err(e)) => std::task::Poll::Ready(Some(Err(TerminalError::WebSocket(e)))),
      None => std::task::Poll::Ready(None),
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
  #[error("Jupyter client error: {0}")]
  Client(ClientError),
  #[error("WebSocket error: {0}")]
  WebSocket(reqwest_websocket::Error),
  #[error("JSON error: {0}")]
  Json(serde_json::Error),
  #[error("Timed out after {0:?}")]
  Timeout(Duration),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCallResult {
  pub stdout: String,
  pub disconnect_code: Option<i32>,
}

pub enum InputMessage {
  /// stdin,$0
  Stdin(String),
  /// set_size,$0,$1,??,??
  Resize { cols: u16, rows: u16 },
}

impl TryFrom<InputMessage> for serde_json::Value {
  type Error = serde_json::Error;

  fn try_from(value: InputMessage) -> Result<Self, Self::Error> {
    match value {
      InputMessage::Stdin(data) => Ok(json!(["stdin", data])),
      InputMessage::Resize { cols, rows } => Ok(json!(["set_size", cols, rows, 800, 600])),
    }
  }
}

pub enum OutputMessage {
  /// setup,{}
  Init {},
  /// stdout,$0
  Stdout(String),
  /// disconnect,$0
  Disconnect(i32),
}

impl TryFrom<serde_json::Value> for OutputMessage {
  type Error = serde_json::Error;

  fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
    use serde::de::Error;
    let arr = value.as_array().ok_or_else(|| serde_json::Error::custom("expected array"))?;
    match arr.get(0).and_then(|v| v.as_str()) {
      Some("stdout") => {
        let data = arr.get(1).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        Ok(OutputMessage::Stdout(data))
      }
      Some("setup") => Ok(OutputMessage::Init {}),
      Some("disconnect") => {
        let code = arr.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        Ok(OutputMessage::Disconnect(code))
      }
      _ => {
        warn!("unknown terminal message: {:?}", value);
        Err(serde_json::Error::custom("unknown message type"))
      },
    }
  }
}

impl TerminalService {
  /// Ensure a terminal exists and is retrievable via `get_terminal`.
  ///
  /// If `force` is true, a missing terminal will be created.
  /// After creation, this will retry `get_terminal` up to `retry_count` times
  /// to allow the server time to make the terminal visible.
  pub async fn get(
    client: &JupyterLabClient,
    terminal_name: &str,
    force: bool,
    retry_count: usize,
  ) -> Result<crate::api::resp::Terminal, TerminalError> {
    let resolved_name = match client.get_terminal(terminal_name).await {
      Ok(terminal) => terminal.name,
      Err(ClientError::Api { status, .. }) if status == StatusCode::NOT_FOUND && force => {
        let terminal = client
          .create_terminal(Some(terminal_name))
          .await
          .map_err(TerminalError::Client)?;
        terminal.name
      }
      Err(err) => return Err(TerminalError::Client(err)),
    };

    let mut attempt = 0usize;
    loop {
      match client.get_terminal(&resolved_name).await {
        Ok(terminal) => return Ok(terminal),
        Err(ClientError::Api { status, .. })
          if status == StatusCode::NOT_FOUND && attempt < retry_count =>
        {
          // Exponential-ish backoff: 50ms, 100ms, 200ms... capped.
          let exp = (attempt as u32).min(10);
          let delay_ms = 50u64.saturating_mul(1u64 << exp);
          attempt += 1;
          if attempt < retry_count {
            println!("Retrying to get terminal '{}' in {}ms...retry={}", terminal_name, delay_ms, retry_count - attempt);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
          }
        }
        Err(err) => return Err(TerminalError::Client(err)),
      }
    }
  }

  pub async fn connect(
    client: JupyterLabClient,
    terminal_name: &str,
    force: bool,
  ) -> Result<TerminalService, TerminalError> {
    let terminal = Self::get(&client, terminal_name, force, 10).await?;

    let ws = client
      .connect_terminal(&terminal.name)
      .await
      .map_err(TerminalError::Client)?;

    Ok(TerminalService {
      client,
      name: terminal.name,
      ws,
      buffer: Vec::new(),
    })
  }

  pub async fn send_message(&mut self, input: InputMessage) -> Result<(), TerminalError> {
    let msg_value = serde_json::Value::try_from(input).map_err(TerminalError::Json)?;
    let msg_text = serde_json::to_string(&msg_value).map_err(TerminalError::Json)?;
    self.ws
      .send(Message::Text(msg_text))
      .await
      .map_err(TerminalError::WebSocket)
  }

  pub async fn read_message(&mut self) -> Result<Option<OutputMessage>, TerminalError> {
    match self.ws.next().await {
      Some(Ok(Message::Text(text))) => {
        let msg_value: serde_json::Value =
          serde_json::from_str(&text).map_err(TerminalError::Json)?;
        let output_msg = OutputMessage::try_from(msg_value).map_err(TerminalError::Json)?;
        Ok(Some(output_msg))
      },
      Some(Ok(_)) => Ok(None),
      Some(Err(e)) => Err(TerminalError::WebSocket(e)),
      None => Ok(None),
    }
  }

  pub fn split(self) -> TerminalSplit {
    let TerminalService { client, name, ws, .. } = self;
    let (sink, stream) = ws.split();
    TerminalSplit {
      client,
      name,
      sink: TerminalInputSink { sink },
      stream: TerminalOutputStream { stream },
    }
  }

  /// Run a single command on this terminal, then request shell exit.
  ///
  /// This consumes the terminal connection. The returned output is whatever the terminal
  /// emitted on stdout before disconnect.
  pub async fn call(
    self,
    command: impl AsRef<str>,
    timeout: Option<Duration>,
  ) -> Result<TerminalCallResult, TerminalError> {
    let mut service = self;

    let raw = command.as_ref();
    let mut cmd = raw.to_string();
    if !cmd.ends_with('\n') {
      cmd.push('\n');
    }
    service.send_message(InputMessage::Stdin(cmd)).await?;
    service
      .send_message(InputMessage::Stdin("exit\n".to_string()))
      .await?;

    let read_until_disconnect = async move {
      let mut stdout = String::new();
      let mut disconnect_code = None;
      loop {
        let Some(msg) = service.read_message().await? else {
          break;
        };
        match msg {
          OutputMessage::Init {} => {}
          OutputMessage::Stdout(data) => stdout.push_str(&data),
          OutputMessage::Disconnect(code) => {
            disconnect_code = Some(code);
            break;
          }
        }
      }
      Ok::<TerminalCallResult, TerminalError>(TerminalCallResult {
        stdout,
        disconnect_code,
      })
    };

    match timeout {
      Some(dur) => tokio::time::timeout(dur, read_until_disconnect)
        .await
        .map_err(|_| TerminalError::Timeout(dur))?,
      None => read_until_disconnect.await,
    }
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use crate::api::{
    client::tests::_setup_client,
    jupyter::JupyterApi,
  };

  use super::TerminalService;

  #[tokio::test]
  async fn test_terminal_service_get_force_create() {
    let client = _setup_client();
    let terminal_name = format!("{}", 3024);

    let terminal = TerminalService::get(&client, &terminal_name, true, 10)
      .await
      .unwrap();
    assert_eq!(terminal.name, terminal_name);

    // Clean up terminal resource
    let client = _setup_client();
    client.delete_terminal(&terminal_name).await.unwrap();
  }

  #[tokio::test]
  async fn test_terminal_service_call_echo() {
    let client = _setup_client();
    let terminal_name = format!("{}", 3025);

    let service = TerminalService::connect(client, &terminal_name, true).await.unwrap();
    let created_name = service.name.clone();
    let marker = "__JUPYTER_SHELL_CALL_TEST__";
    let result = service
      .call(format!("echo {marker}"), Some(Duration::from_secs(10)))
      .await
      .unwrap();

    assert!(result.stdout.contains(marker), "stdout did not contain marker; stdout={:?}", result.stdout);

    // Clean up terminal resource
    let client = _setup_client();
    client.delete_terminal(&created_name).await.ok();
  }
}
