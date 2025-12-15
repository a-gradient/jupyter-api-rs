use crate::api::{
  param::*,
  resp::{
    APIStatus, Checkpoint, Contents, Kernel, KernelSpecsResponse, MeResponse, Session,
    Terminal,
  },
};
use reqwest::{
  header::{HeaderValue, AUTHORIZATION},
  Client, ClientBuilder, Method, RequestBuilder, Response, StatusCode, Url,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fmt, time::Duration};
use uuid::Uuid;
pub struct JupyterRestClient {
  client: Client,
  base_url: Url,
  auth_header: Option<HeaderValue>,
}

#[derive(Debug)]
pub struct RestClientBuilder {
  base_url: Url,
  client_builder: ClientBuilder,
  auth_header: Option<HeaderValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerVersion {
  pub version: String,
}

#[derive(Debug)]
pub enum RestError {
  InvalidBaseUrl(String),
  Http(reqwest::Error),
  Api { status: StatusCode, message: String },
  InvalidHeader(String),
}

impl fmt::Display for RestError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RestError::InvalidBaseUrl(msg) => write!(f, "invalid base url: {msg}"),
      RestError::Http(err) => write!(f, "http error: {err}"),
      RestError::Api { status, message } => {
        if message.is_empty() {
          write!(f, "api error: {status}")
        } else {
          write!(f, "api error: {status} - {message}")
        }
      }
      RestError::InvalidHeader(msg) => write!(f, "invalid auth header: {msg}"),
    }
  }
}

impl std::error::Error for RestError {}

impl From<reqwest::Error> for RestError {
  fn from(value: reqwest::Error) -> Self {
    RestError::Http(value)
  }
}

impl JupyterRestClient {
  pub fn new(base_url: impl AsRef<str>) -> Result<Self, RestError> {
    Self::from_client(base_url, Client::new(), None)
  }

  pub fn with_token(base_url: impl AsRef<str>, token: impl AsRef<str>) -> Result<Self, RestError> {
    let header = build_token_header(token.as_ref())?;
    Self::from_client(base_url, Client::new(), Some(header))
  }

  pub fn builder(base_url: impl AsRef<str>) -> Result<RestClientBuilder, RestError> {
    RestClientBuilder::new(base_url)
  }

  pub fn from_client(
    base_url: impl AsRef<str>,
    client: Client,
    auth_header: Option<HeaderValue>,
  ) -> Result<Self, RestError> {
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

  pub async fn server_version(&self) -> Result<ServerVersion, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn get_contents(
    &self,
    path: &str,
    params: Option<&ContentsGetParams>,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path_allow_empty(path)])?;
    let mut request = self.request(Method::GET, url);
    if let Some(query) = params {
      request = request.query(query);
    }
    self.send_json(request).await
  }

  pub async fn create_contents(
    &self,
    path: &str,
    model: &CreateContentsModel,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path_allow_empty(path)])?;
    let request = self.request(Method::POST, url).json(model);
    self.send_json(request).await
  }

  pub async fn rename_contents(
    &self,
    path: &str,
    rename: &RenameContentsModel,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::PATCH, url).json(rename);
    self.send_json(request).await
  }

  pub async fn save_contents(
    &self,
    path: &str,
    model: &SaveContentsModel,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::PUT, url).json(model);
    self.send_json(request).await
  }

  pub async fn delete_contents(&self, path: &str) -> Result<(), RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  pub async fn list_checkpoints(&self, path: &str) -> Result<Vec<Checkpoint>, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn create_checkpoint(&self, path: &str) -> Result<Checkpoint, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
    ])?;
    let request = self.request(Method::POST, url);
    self.send_json(request).await
  }

  pub async fn restore_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
      Segment::literal(checkpoint_id.to_string()),
    ])?;
    let request = self.request(Method::POST, url);
    self.send_empty(request).await
  }

  pub async fn delete_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
      Segment::literal(checkpoint_id.to_string()),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  pub async fn get_session(&self, session_id: Uuid) -> Result<Session, RestError> {
    let session = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn update_session(
    &self,
    session_id: Uuid,
    session: &Session,
  ) -> Result<Session, RestError> {
    let session_id = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session_id),
    ])?;
    let request = self.request(Method::PATCH, url).json(session);
    self.send_json(request).await
  }

  pub async fn delete_session(&self, session_id: Uuid) -> Result<(), RestError> {
    let session = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  pub async fn list_sessions(&self) -> Result<Vec<Session>, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("sessions")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn create_session(&self, session: &Session) -> Result<Session, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("sessions")])?;
    let request = self.request(Method::POST, url).json(session);
    self.send_json(request).await
  }

  pub async fn list_kernels(&self) -> Result<Vec<Kernel>, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernels")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn start_kernel(&self, options: &KernelStartOptions) -> Result<Kernel, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernels")])?;
    let request = self.request(Method::POST, url).json(options);
    self.send_json(request).await
  }

  pub async fn get_kernel(&self, kernel_id: Uuid) -> Result<Kernel, RestError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn delete_kernel(&self, kernel_id: Uuid) -> Result<(), RestError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  pub async fn interrupt_kernel(&self, kernel_id: Uuid) -> Result<(), RestError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
      Segment::literal("interrupt"),
    ])?;
    let request = self.request(Method::POST, url);
    self.send_empty(request).await
  }

  pub async fn restart_kernel(&self, kernel_id: Uuid) -> Result<Kernel, RestError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
      Segment::literal("restart"),
    ])?;
    let request = self.request(Method::POST, url);
    self.send_json(request).await
  }

  pub async fn kernel_specs(&self) -> Result<KernelSpecsResponse, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernelspecs")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn get_config_section(&self, section_name: &str) -> Result<Value, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("config"),
      Segment::literal(section_name.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn patch_config_section(
    &self,
    section_name: &str,
    configuration: &ConfigPatchRequest,
  ) -> Result<Value, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("config"),
      Segment::literal(section_name.to_string()),
    ])?;
    let request = self.request(Method::PATCH, url).json(configuration);
    self.send_json(request).await
  }

  pub async fn list_terminals(&self) -> Result<Vec<Terminal>, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("terminals")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn create_terminal(&self, name: Option<&str>) -> Result<Terminal, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("terminals")])?;
    let payload = match name {
      Some(value) => json!({ "name": value }),
      None => json!({}),
    };
    let request = self.request(Method::POST, url).json(&payload);
    self.send_json(request).await
  }

  pub async fn get_terminal(&self, terminal_id: &str) -> Result<Terminal, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("terminals"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn delete_terminal(&self, terminal_id: &str) -> Result<(), RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("terminals"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  pub async fn me(&self, params: Option<&PermissionsQueryParam>) -> Result<MeResponse, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("me")])?;
    let mut request = self.request(Method::GET, url);
    if let Some(query) = params {
      request = request.query(query);
    }
    self.send_json(request).await
  }

  pub async fn status(&self) -> Result<APIStatus, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("status")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  pub async fn download_spec(&self) -> Result<String, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("spec.yaml")])?;
    let request = self.request(Method::GET, url);
    let response = self.send(request).await?;
    response.text().await.map_err(RestError::Http)
  }

  fn request(&self, method: Method, url: Url) -> RequestBuilder {
    let request = self.client.request(method, url);
    match &self.auth_header {
      Some(header) => request.header(AUTHORIZATION, header.clone()),
      None => request,
    }
  }

  async fn send_json<T>(&self, request: RequestBuilder) -> Result<T, RestError>
  where
    T: DeserializeOwned,
  {
    let response = self.send(request).await?;
    response.json::<T>().await.map_err(RestError::Http)
  }

  async fn send_empty(&self, request: RequestBuilder) -> Result<(), RestError> {
    self.send(request).await?;
    Ok(())
  }

  async fn send(&self, request: RequestBuilder) -> Result<Response, RestError> {
    let response = request.send().await.map_err(RestError::Http)?;
    if response.status().is_success() {
      Ok(response)
    } else {
      let status = response.status();
      let message = response.text().await.unwrap_or_default();
      Err(RestError::Api { status, message })
    }
  }

  fn build_url(&self, segments: &[Segment]) -> Result<Url, RestError> {
    let mut url = self.base_url.clone();
    {
      let mut parts = url
        .path_segments_mut()
        .map_err(|_| RestError::InvalidBaseUrl("supplied base url cannot be a base".into()))?;
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

impl RestClientBuilder {
  pub fn new(base_url: impl AsRef<str>) -> Result<Self, RestError> {
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

  pub fn auto_token(mut self, token: impl AsRef<str>) -> Result<Self, RestError> {
    let value = build_token_header(token.as_ref()).map_err(|err| {
      RestError::InvalidHeader(err.to_string())
    })?;
    self.auth_header = Some(value);
    Ok(self)
  }

  pub fn token(mut self, token: impl AsRef<str>) -> Result<Self, RestError> {
    let header = build_token_header(token.as_ref())?;
    self.auth_header = Some(header);
    Ok(self)
  }

  pub fn custom_auth_header(mut self, header: HeaderValue) -> Self {
    self.auth_header = Some(header);
    self
  }

  pub fn build(self) -> Result<JupyterRestClient, RestError> {
    let client = self.client_builder.build().map_err(RestError::Http)?;
    Ok(JupyterRestClient {
      client,
      base_url: self.base_url,
      auth_header: self.auth_header,
    })
  }
}

fn parse_base_url(raw: &str) -> Result<Url, RestError> {
  Url::parse(raw).map_err(|err| RestError::InvalidBaseUrl(err.to_string()))
}

fn build_token_header(token: &str) -> Result<HeaderValue, RestError> {
  let value = format!("token {}", token);
  HeaderValue::from_str(&value).map_err(|err| RestError::InvalidHeader(err.to_string()))
}

#[derive(Debug, Clone)]
enum Segment {
  Literal(String),
  Path {
    value: String,
    keep_trailing_slash_if_empty: bool,
  },
}

impl Segment {
  fn literal(value: impl Into<String>) -> Self {
    Segment::Literal(value.into())
  }

  fn path(value: impl Into<String>) -> Self {
    Segment::Path {
      value: value.into(),
      keep_trailing_slash_if_empty: false,
    }
  }

  fn path_allow_empty(value: impl Into<String>) -> Self {
    Segment::Path {
      value: value.into(),
      keep_trailing_slash_if_empty: true,
    }
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  pub(crate) fn _setup_client() -> JupyterRestClient {
    RestClientBuilder::new("http://localhost:8888").unwrap()
      .auto_token(include_str!("../../.secret").trim()).unwrap()
      .build().unwrap()
  }

  #[test]
  fn test_builder() {
    let builder = RestClientBuilder::new("http://localhost:8888").unwrap();
    let client = builder
      .timeout(Duration::from_secs(10))
      .user_agent("jupyter-rest-client/0.1")
      .auto_token(include_str!("../../.secret").trim()).unwrap()
      .build()
      .unwrap();
    assert_eq!(client.base_url().as_str(), "http://localhost:8888/");
  }

  #[tokio::test]
  async fn test_status() {
    let client = _setup_client();
    let status = client.status().await.unwrap();
    println!("API Status: {:?}", status);
    assert!(status.started.is_some());
  }

  #[tokio::test]
  async fn test_server_version() {
    let client = _setup_client();
    let version = client.server_version().await.unwrap();
    println!("Server Version: {:?}", version);
    assert!(version.version.len() > 0);
  }

  #[tokio::test]
  async fn test_me() {
    let client = _setup_client();
    let me = client.me(None).await.unwrap();
    println!("Me: {:?}", me);
    assert!(me.identity.is_some());
  }

  #[tokio::test]
  async fn test_list_kernels() {
    let client = _setup_client();
    let kernels = client.list_kernels().await.unwrap();
    println!("Kernels: {:?}", kernels);
  }

  #[tokio::test]
  async fn test_kernel_specs() {
    let client = _setup_client();
    let specs = client.kernel_specs().await.unwrap();
    println!("Kernel Specs: {:?}", specs);
    assert!(specs.default.is_some());
  }

  #[tokio::test]
  async fn test_list_sessions() {
    let client = _setup_client();
    let sessions = client.list_sessions().await.unwrap();
    println!("Sessions: {:?}", sessions);
  }

  #[tokio::test]
  async fn test_list_contents() {
    let client = _setup_client();
    let contents = client.get_contents("/Untitled Folder", None).await.unwrap();
    println!("Contents: {:?}", contents);
    assert!(contents.content_type == "directory");

    let contents = client.get_contents("/hello.txt", Some(&ContentsGetParams { entry_type: None, format: None, content: Some(false), hash: Some(true) })).await.unwrap();
    // assert!(contents.)
    println!("Contents: {:?}", contents);
    assert_eq!(contents.content_type, "file");
    assert_eq!(contents.content, None);
    assert!(contents.hash.is_some());
    assert_eq!(contents.hash_algorithm.as_deref(), Some("sha256"));
  }
}
