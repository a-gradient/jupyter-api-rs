use reqwest::{
  header::{HeaderValue, AUTHORIZATION},
  Client, ClientBuilder, Method, RequestBuilder, Response, StatusCode, Url,
};
use reqwest_websocket::{RequestBuilderExt, UpgradeResponse};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

#[derive(Debug)]
pub struct JupyterLabClient {
  client: Client,
  base_url: Url,
  auth_header: Option<HeaderValue>,
}

#[derive(Debug)]
pub struct JupyterLabClientBuilder {
  base_url: Url,
  client_builder: ClientBuilder,
  auth_header: Option<HeaderValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerVersion {
  pub version: String,
}

#[derive(Debug)]
pub enum ClientError {
  InvalidBaseUrl(String),
  InvalidInput(String),
  Http(reqwest::Error),
  Websocket(reqwest_websocket::Error),
  Api { status: StatusCode, message: String },
  InvalidHeader(String),
}

impl fmt::Display for ClientError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ClientError::InvalidBaseUrl(msg) => write!(f, "invalid base url: {msg}"),
      ClientError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
      ClientError::Http(err) => write!(f, "http error: {err}"),
      ClientError::Websocket(err) => write!(f, "websocket error: {err}"),
      ClientError::Api { status, message } => {
        if message.is_empty() {
          write!(f, "api error: {status}")
        } else {
          write!(f, "api error: {status} - {message}")
        }
      }
      ClientError::InvalidHeader(msg) => write!(f, "invalid auth header: {msg}"),
    }
  }
}

impl std::error::Error for ClientError {}

impl From<reqwest::Error> for ClientError {
  fn from(value: reqwest::Error) -> Self {
    ClientError::Http(value)
  }
}

impl JupyterLabClient {
  pub fn new(base_url: impl AsRef<str>) -> Result<Self, ClientError> {
    Self::from_client(base_url, Client::new(), None)
  }

  pub fn with_token(base_url: impl AsRef<str>, token: impl AsRef<str>) -> Result<Self, ClientError> {
    let header = build_token_header(token.as_ref())?;
    Self::from_client(base_url, Client::new(), Some(header))
  }

  pub fn builder(base_url: impl AsRef<str>) -> Result<JupyterLabClientBuilder, ClientError> {
    JupyterLabClientBuilder::new(base_url)
  }

  pub fn from_client(
    base_url: impl AsRef<str>,
    client: Client,
    auth_header: Option<HeaderValue>,
  ) -> Result<Self, ClientError> {
    let base_url = parse_base_url(base_url.as_ref())?;
    Ok(Self {
      client,
      base_url,
      auth_header,
    })
  }

  pub fn base_url(&self) -> &Url {
    &self.base_url
  }

  pub fn http_client(&self) -> &Client {
    &self.client
  }

  pub(super) fn request(&self, method: Method, url: Url) -> RequestBuilder {
    let request = self.client.request(method, url);
    match &self.auth_header {
      Some(header) => request.header(AUTHORIZATION, header.clone()),
      None => request,
    }
  }

  pub(super) async fn send_json<T>(&self, request: RequestBuilder) -> Result<T, ClientError>
  where
    T: DeserializeOwned,
  {
    let response = self.send(request).await?;
    response.json::<T>().await.map_err(ClientError::Http)
  }

  pub(super) async fn send_empty(&self, request: RequestBuilder) -> Result<(), ClientError> {
    self.send(request).await?;
    Ok(())
  }

  pub(super) async fn send_ws(&self, request: RequestBuilder) -> Result<UpgradeResponse, ClientError> {
    let (client, request) = request.build_split();
    let mut request = request?;
    let url = request.url_mut();
    match url.scheme() {
      "http" => url.set_scheme("ws"),
      "https" => url.set_scheme("wss"),
      _ => Err(()),
    }.map_err(|_| ClientError::InvalidBaseUrl(format!("could not set_scheme from {url}")))?;
    let request = RequestBuilder::from_parts(client, request);
    let response = request.upgrade().send().await.map_err(ClientError::Websocket)?;
    let status = response.status();
    if status != StatusCode::SWITCHING_PROTOCOLS {
      let message = response.into_inner().text().await.unwrap_or_default();
      return Err(ClientError::Api { status, message });
    }
    Ok(response)
  }

  pub(super) async fn send(&self, request: RequestBuilder) -> Result<Response, ClientError> {
    let response = request.send().await.map_err(ClientError::Http)?;
    if response.status().is_success() {
      Ok(response)
    } else {
      let status = response.status();
      let message = response.text().await.unwrap_or_default();
      Err(ClientError::Api { status, message })
    }
  }

  pub(super) fn build_url(&self, segments: &[Segment]) -> Result<Url, ClientError> {
    let mut url = self.base_url.clone();
    {
      let mut parts = url
        .path_segments_mut()
        .map_err(|_| ClientError::InvalidBaseUrl("supplied base url cannot be a base".into()))?;
      parts.pop_if_empty();
      for segment in segments {
        match segment {
          Segment::Literal(value) => {
            parts.push(value);
          }
          Segment::Path {
            value,
            keep_trailing_slash_if_empty,
          } => {
            let trimmed = value.trim_matches('/');
            if trimmed.is_empty() {
              if *keep_trailing_slash_if_empty {
                parts.push("");
              }
            } else {
              for chunk in trimmed.split('/') {
                if chunk.is_empty() {
                  continue;
                }
                parts.push(chunk);
              }
            }
          }
        }
      }
    }
    Ok(url)
  }
}

impl JupyterLabClientBuilder {
  pub fn new(base_url: impl AsRef<str>) -> Result<Self, ClientError> {
    let base_url = parse_base_url(base_url.as_ref())?;
    Ok(Self {
      base_url,
      client_builder: Client::builder(),
      auth_header: None,
    })
  }

  pub fn client_builder(mut self, builder: ClientBuilder) -> Self {
    self.client_builder = builder;
    self
  }

  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.client_builder = self.client_builder.timeout(timeout);
    self
  }

  pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
    self.client_builder = self.client_builder.user_agent(user_agent.into());
    self
  }

  pub fn danger_accept_invalid_certs(mut self, accept_invalid: bool) -> Self {
    self.client_builder = self.client_builder.danger_accept_invalid_certs(accept_invalid);
    self
  }

  pub fn auto_token(mut self, token: impl AsRef<str>) -> Result<Self, ClientError> {
    let value = build_token_header(token.as_ref()).map_err(|err| {
      ClientError::InvalidHeader(err.to_string())
    })?;
    self.auth_header = Some(value);
    Ok(self)
  }

  pub fn token(mut self, token: impl AsRef<str>) -> Result<Self, ClientError> {
    let header = build_token_header(token.as_ref())?;
    self.auth_header = Some(header);
    Ok(self)
  }

  pub fn custom_auth_header(mut self, header: HeaderValue) -> Self {
    self.auth_header = Some(header);
    self
  }

  pub fn build(self) -> Result<JupyterLabClient, ClientError> {
    let client = self.client_builder.build().map_err(ClientError::Http)?;
    Ok(JupyterLabClient {
      client,
      base_url: self.base_url,
      auth_header: self.auth_header,
    })
  }
}

fn parse_base_url(raw: &str) -> Result<Url, ClientError> {
  Url::parse(raw).map_err(|err| ClientError::InvalidBaseUrl(err.to_string()))
}

fn build_token_header(token: &str) -> Result<HeaderValue, ClientError> {
  let value = format!("token {}", token);
  HeaderValue::from_str(&value).map_err(|err| ClientError::InvalidHeader(err.to_string()))
}

#[derive(Debug, Clone)]
pub(super) enum Segment {
  Literal(String),
  Path {
    value: String,
    keep_trailing_slash_if_empty: bool,
  },
}

impl Segment {
  pub fn literal(value: impl Into<String>) -> Self {
    Segment::Literal(value.into())
  }

  pub fn path(value: impl Into<String>) -> Self {
    Segment::Path {
      value: value.into(),
      keep_trailing_slash_if_empty: false,
    }
  }

  pub fn path_allow_empty(value: impl Into<String>) -> Self {
    Segment::Path {
      value: value.into(),
      keep_trailing_slash_if_empty: true,
    }
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  pub(crate) fn _setup_client() -> JupyterLabClient {
    JupyterLabClientBuilder::new("http://localhost:8888").unwrap()
      .auto_token(include_str!("../../.secret").trim()).unwrap()
      .build().unwrap()
  }

  #[test]
  fn test_builder() {
    let builder = JupyterLabClientBuilder::new("http://localhost:8888").unwrap();
    let client = builder
      .timeout(Duration::from_secs(10))
      .user_agent("jupyter-api-rs/0.1")
      .auto_token(include_str!("../../.secret").trim()).unwrap()
      .build()
      .unwrap();
    assert_eq!(client.base_url().as_str(), "http://localhost:8888/");
  }
}
