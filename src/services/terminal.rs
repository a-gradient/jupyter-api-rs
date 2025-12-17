use futures_util::{
  SinkExt, Stream, StreamExt, stream::{SplitSink, SplitStream}
};
use reqwest_websocket::Message;
use serde_json::json;

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
  pub async fn connect(
    client: JupyterLabClient,
    terminal_name: &str,
  ) -> Result<TerminalService, TerminalError> {
    let ws = client.connect_terminal(terminal_name).await.map_err(TerminalError::Client)?;
    Ok(TerminalService {
      client,
      name: terminal_name.to_string(),
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
}
