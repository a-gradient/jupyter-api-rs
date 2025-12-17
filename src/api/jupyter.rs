use crate::api::{
  client::*, param::*, resp::*
};
use reqwest::{Method, Response};
use reqwest_websocket::WebSocket;
use serde_json::{json, Value};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait JupyterApi {
  async fn server_version(&self) -> Result<ServerVersion, ClientError>;

  async fn get_contents(
    &self,
    path: &str,
    params: Option<&ContentsGetParams>,
  ) -> Result<Contents, ClientError>;

  async fn create_contents(
    &self,
    path: &str,
    model: &CreateContentsModel,
  ) -> Result<Contents, ClientError>;

  async fn rename_contents(
    &self,
    path: &str,
    rename: &RenameContentsModel,
  ) -> Result<Contents, ClientError>;

  async fn save_contents(
    &self,
    path: &str,
    model: &SaveContentsModel,
  ) -> Result<Contents, ClientError>;

  async fn delete_contents(&self, path: &str) -> Result<(), ClientError>;

  async fn list_checkpoints(&self, path: &str) -> Result<Vec<Checkpoint>, ClientError>;

  async fn create_checkpoint(&self, path: &str) -> Result<Checkpoint, ClientError>;

  async fn restore_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), ClientError>;

  async fn delete_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), ClientError>;

  async fn get_session(&self, session_id: Uuid) -> Result<Session, ClientError>;

  async fn update_session(
    &self,
    session_id: Uuid,
    session: &Session,
  ) -> Result<Session, ClientError>;

  async fn delete_session(&self, session_id: Uuid) -> Result<(), ClientError>;

  async fn list_sessions(&self) -> Result<Vec<Session>, ClientError>;

  async fn create_session(&self, session: &Session) -> Result<Session, ClientError>;

  async fn list_kernels(&self) -> Result<Vec<Kernel>, ClientError>;

  async fn start_kernel(&self, options: &KernelStartOptions) -> Result<Kernel, ClientError>;

  async fn get_kernel(&self, kernel_id: Uuid) -> Result<Kernel, ClientError>;

  async fn delete_kernel(&self, kernel_id: Uuid) -> Result<(), ClientError>;

  async fn interrupt_kernel(&self, kernel_id: Uuid) -> Result<(), ClientError>;

  async fn restart_kernel(&self, kernel_id: Uuid) -> Result<Kernel, ClientError>;

  async fn kernel_specs(&self) -> Result<KernelSpecsResponse, ClientError>;

  async fn get_config_section(&self, section_name: &str) -> Result<Value, ClientError>;

  async fn patch_config_section(
    &self,
    section_name: &str,
    configuration: &ConfigPatchRequest,
  ) -> Result<Value, ClientError>;

  async fn list_terminals(&self) -> Result<Vec<Terminal>, ClientError>;

  async fn create_terminal(&self, name: Option<&str>) -> Result<Terminal, ClientError>;

  async fn get_terminal(&self, terminal_id: &str) -> Result<Terminal, ClientError>;

  async fn connect_terminal(&self, terminal_id: &str) -> Result<WebSocket, ClientError>;

  async fn delete_terminal(&self, terminal_id: &str) -> Result<(), ClientError>;

  async fn me(&self, params: Option<&PermissionsQueryParam>) -> Result<MeResponse, ClientError>;

  async fn status(&self) -> Result<APIStatus, ClientError>;

  async fn download_spec(&self) -> Result<String, ClientError>;
}

#[async_trait::async_trait]
impl JupyterApi for JupyterLabClient {
  async fn server_version(&self) -> Result<ServerVersion, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn get_contents(
    &self,
    path: &str,
    params: Option<&ContentsGetParams>,
  ) -> Result<Contents, ClientError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path_allow_empty(path)])?;
    let mut request = self.request(Method::GET, url);
    if let Some(query) = params {
      request = request.query(query);
    }
    self.send_json(request).await
  }

  async fn create_contents(
    &self,
    path: &str,
    model: &CreateContentsModel,
  ) -> Result<Contents, ClientError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path_allow_empty(path)])?;
    let request = self.request(Method::POST, url).json(model);
    self.send_json(request).await
  }

  async fn rename_contents(
    &self,
    path: &str,
    rename: &RenameContentsModel,
  ) -> Result<Contents, ClientError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::PATCH, url).json(rename);
    self.send_json(request).await
  }

  async fn save_contents(
    &self,
    path: &str,
    model: &SaveContentsModel,
  ) -> Result<Contents, ClientError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::PUT, url).json(model);
    self.send_json(request).await
  }

  async fn delete_contents(&self, path: &str) -> Result<(), ClientError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn list_checkpoints(&self, path: &str) -> Result<Vec<Checkpoint>, ClientError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn create_checkpoint(&self, path: &str) -> Result<Checkpoint, ClientError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
    ])?;
    let request = self.request(Method::POST, url);
    self.send_json(request).await
  }

  async fn restore_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), ClientError> {
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

  async fn delete_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), ClientError> {
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

  async fn get_session(&self, session_id: Uuid) -> Result<Session, ClientError> {
    let session = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn update_session(
    &self,
    session_id: Uuid,
    session: &Session,
  ) -> Result<Session, ClientError> {
    let session_id = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session_id),
    ])?;
    let request = self.request(Method::PATCH, url).json(session);
    self.send_json(request).await
  }

  async fn delete_session(&self, session_id: Uuid) -> Result<(), ClientError> {
    let session = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn list_sessions(&self) -> Result<Vec<Session>, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("sessions")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn create_session(&self, session: &Session) -> Result<Session, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("sessions")])?;
    let request = self.request(Method::POST, url).json(session);
    self.send_json(request).await
  }

  async fn list_kernels(&self) -> Result<Vec<Kernel>, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernels")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn start_kernel(&self, options: &KernelStartOptions) -> Result<Kernel, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernels")])?;
    let request = self.request(Method::POST, url).json(options);
    self.send_json(request).await
  }

  async fn get_kernel(&self, kernel_id: Uuid) -> Result<Kernel, ClientError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn delete_kernel(&self, kernel_id: Uuid) -> Result<(), ClientError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn interrupt_kernel(&self, kernel_id: Uuid) -> Result<(), ClientError> {
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

  async fn restart_kernel(&self, kernel_id: Uuid) -> Result<Kernel, ClientError> {
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

  async fn kernel_specs(&self) -> Result<KernelSpecsResponse, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernelspecs")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn get_config_section(&self, section_name: &str) -> Result<Value, ClientError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("config"),
      Segment::literal(section_name.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn patch_config_section(
    &self,
    section_name: &str,
    configuration: &ConfigPatchRequest,
  ) -> Result<Value, ClientError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("config"),
      Segment::literal(section_name.to_string()),
    ])?;
    let request = self.request(Method::PATCH, url).json(configuration);
    self.send_json(request).await
  }

  async fn list_terminals(&self) -> Result<Vec<Terminal>, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("terminals")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn create_terminal(&self, name: Option<&str>) -> Result<Terminal, ClientError> {
    if let Some(n) = name {
      if n.is_empty() {
        return Err(ClientError::InvalidInput("terminal name cannot be empty".to_string()));
      } else if !n.as_bytes().iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_' ) {
        // '-' is not allowed in terminal names
        return Err(ClientError::InvalidInput("terminal name contains invalid characters".to_string()));
      }
    }
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("terminals")])?;
    let payload = match name {
      Some(value) => json!({ "name": value }),
      None => json!({}),
    };
    let request = self.request(Method::POST, url).json(&payload);
    self.send_json(request).await
  }

  async fn get_terminal(&self, terminal_id: &str) -> Result<Terminal, ClientError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("terminals"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn connect_terminal(&self, terminal_id: &str) -> Result<WebSocket, ClientError> {
    let url = self.build_url(&[
      Segment::literal("terminals"),
      Segment::literal("websocket"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    let resp = self.send_ws(request).await?;
    resp.into_websocket().await.map_err(ClientError::Websocket)
  }

  async fn delete_terminal(&self, terminal_id: &str) -> Result<(), ClientError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("terminals"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn me(&self, params: Option<&PermissionsQueryParam>) -> Result<MeResponse, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("me")])?;
    let mut request = self.request(Method::GET, url);
    if let Some(query) = params {
      request = request.query(query);
    }
    self.send_json(request).await
  }

  async fn status(&self) -> Result<APIStatus, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("status")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn download_spec(&self) -> Result<String, ClientError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("spec.yaml")])?;
    let request = self.request(Method::GET, url);
    let response = self.send(request).await?;
    response.text().await.map_err(ClientError::Http)
  }
}

#[async_trait::async_trait]
pub trait JupyterLabApi {
  async fn get_files_stream(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Response, ClientError>;
  async fn get_files(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Vec<u8>, ClientError> {
    let response = self.get_files_stream(path, range).await?;
    response.bytes().await.map(|b| b.to_vec()).map_err(ClientError::Http)
  }

  /// List all JupyterLab workspaces.
  ///
  /// JupyterLab stores layout/user-state in workspaces, typically under `/lab/api/workspaces`.
  /// The payload is not strictly version-stable, so we return raw JSON.
  async fn list_workspaces(&self) -> Result<Workspaces, ClientError>;

  /// Fetch a single JupyterLab workspace by id.
  async fn get_workspace(&self, workspace_id: &str) -> Result<Workspace, ClientError>;
}

#[async_trait::async_trait]
impl JupyterLabApi for JupyterLabClient {
  async fn get_files_stream(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Response, ClientError> {
    let url = self.build_url(&[
      Segment::literal("files"),
      Segment::path_allow_empty(path),
    ])?;
    let mut request = self.request(Method::GET, url);
    let bytes_range = match range {
      Some((start, Some(end))) => format!("bytes={}-{}", start, end - 1),
      Some((start, None)) => format!("bytes={}-", start),
      None => "".to_string(),
    };
    if !bytes_range.is_empty() {
      request = request.header("Range", bytes_range);
    }
    self.send(request).await
  }

  async fn list_workspaces(&self) -> Result<Workspaces, ClientError> {
    let url = self.build_url(&[
      Segment::literal("lab"),
      Segment::literal("api"),
      Segment::literal("workspaces"),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json::<WorkspacesResp>(request).await.map(WorkspacesResp::inner)
  }

  async fn get_workspace(&self, workspace_id: &str) -> Result<Workspace, ClientError> {
    let url = self.build_url(&[
      Segment::literal("lab"),
      Segment::literal("api"),
      Segment::literal("workspaces"),
      Segment::literal(workspace_id.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }
}

#[cfg(test)]
mod tests {
  use crate::api::client::tests::_setup_client;

  use super::*;

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

  #[tokio::test]
  async fn test_download_contents() {
    let client = _setup_client();
    let data = client.get_files("/hello.txt", None).await.unwrap();
    let text = String::from_utf8_lossy(&data);
    println!("Downloaded hello.txt: {}", text);

    let data2 = client.get_files("/hello.txt", Some((1, Some(2)))).await.unwrap();
    let text2 = String::from_utf8_lossy(&data2);
    println!("Downloaded hello.txt (1-2): {}", text2);
    assert_eq!(&data[1..2], &data2);
  }

  #[tokio::test]
  async fn test_terminal() {
    let client = _setup_client();
    let name = "1testaaaZ_1";
    let terminal = client.create_terminal(Some(name)).await.unwrap();
    assert!(terminal.name == name);
    let terminal_fetched = client.get_terminal(name).await.unwrap();
    assert!(terminal_fetched.name == name);
    let socket = client.connect_terminal(name).await.unwrap();
    client.delete_terminal(name).await.unwrap();
    drop(socket);
  }
}
