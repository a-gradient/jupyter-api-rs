use futures_util::SinkExt;
use reqwest_websocket::Message;

use crate::api::{client::{ClientError, JupyterLabClient}, jupyter::JupyterApi};

pub struct TerminalService {
  pub client: JupyterLabClient,
  pub name: String,
  pub ws: reqwest_websocket::WebSocket,
  pub buffer: Vec<String>,
}

#[derive(Debug)]
pub enum TerminalError {
  Client(ClientError),
  WebSocket(reqwest_websocket::Error),
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

  pub async fn send_message(&mut self, message: &str) -> Result<(), TerminalError> {
    self.ws
      .send(Message::Text(message.to_string()))
      .await
      .map_err(TerminalError::WebSocket)
  }
}
