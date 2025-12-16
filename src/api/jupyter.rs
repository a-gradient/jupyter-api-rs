use crate::api::{
  client::*, param::*, resp::*
};
use reqwest::{Method, Response};
use serde_json::{json, Value};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait JupyterApi {
  async fn server_version(&self) -> Result<ServerVersion, RestError>;

  async fn get_contents(
    &self,
    path: &str,
    params: Option<&ContentsGetParams>,
  ) -> Result<Contents, RestError>;

  async fn create_contents(
    &self,
    path: &str,
    model: &CreateContentsModel,
  ) -> Result<Contents, RestError>;

  async fn rename_contents(
    &self,
    path: &str,
    rename: &RenameContentsModel,
  ) -> Result<Contents, RestError>;

  async fn save_contents(
    &self,
    path: &str,
    model: &SaveContentsModel,
  ) -> Result<Contents, RestError>;

  async fn delete_contents(&self, path: &str) -> Result<(), RestError>;

  async fn list_checkpoints(&self, path: &str) -> Result<Vec<Checkpoint>, RestError>;

  async fn create_checkpoint(&self, path: &str) -> Result<Checkpoint, RestError>;

  async fn restore_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), RestError>;

  async fn delete_checkpoint(
    &self,
    path: &str,
    checkpoint_id: &str,
  ) -> Result<(), RestError>;

  async fn get_session(&self, session_id: Uuid) -> Result<Session, RestError>;

  async fn update_session(
    &self,
    session_id: Uuid,
    session: &Session,
  ) -> Result<Session, RestError>;

  async fn delete_session(&self, session_id: Uuid) -> Result<(), RestError>;

  async fn list_sessions(&self) -> Result<Vec<Session>, RestError>;

  async fn create_session(&self, session: &Session) -> Result<Session, RestError>;

  async fn list_kernels(&self) -> Result<Vec<Kernel>, RestError>;

  async fn start_kernel(&self, options: &KernelStartOptions) -> Result<Kernel, RestError>;

  async fn get_kernel(&self, kernel_id: Uuid) -> Result<Kernel, RestError>;

  async fn delete_kernel(&self, kernel_id: Uuid) -> Result<(), RestError>;

  async fn interrupt_kernel(&self, kernel_id: Uuid) -> Result<(), RestError>;

  async fn restart_kernel(&self, kernel_id: Uuid) -> Result<Kernel, RestError>;

  async fn kernel_specs(&self) -> Result<KernelSpecsResponse, RestError>;

  async fn get_config_section(&self, section_name: &str) -> Result<Value, RestError>;

  async fn patch_config_section(
    &self,
    section_name: &str,
    configuration: &ConfigPatchRequest,
  ) -> Result<Value, RestError>;

  async fn list_terminals(&self) -> Result<Vec<Terminal>, RestError>;

  async fn create_terminal(&self, name: Option<&str>) -> Result<Terminal, RestError>;

  async fn get_terminal(&self, terminal_id: &str) -> Result<Terminal, RestError>;

  async fn delete_terminal(&self, terminal_id: &str) -> Result<(), RestError>;

  async fn me(&self, params: Option<&PermissionsQueryParam>) -> Result<MeResponse, RestError>;

  async fn status(&self) -> Result<APIStatus, RestError>;

  async fn download_spec(&self) -> Result<String, RestError>;
}

#[async_trait::async_trait]
impl JupyterApi for JupyterRestClient {
  async fn server_version(&self) -> Result<ServerVersion, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn get_contents(
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

  async fn create_contents(
    &self,
    path: &str,
    model: &CreateContentsModel,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path_allow_empty(path)])?;
    let request = self.request(Method::POST, url).json(model);
    self.send_json(request).await
  }

  async fn rename_contents(
    &self,
    path: &str,
    rename: &RenameContentsModel,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::PATCH, url).json(rename);
    self.send_json(request).await
  }

  async fn save_contents(
    &self,
    path: &str,
    model: &SaveContentsModel,
  ) -> Result<Contents, RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::PUT, url).json(model);
    self.send_json(request).await
  }

  async fn delete_contents(&self, path: &str) -> Result<(), RestError> {
    let url =
      self.build_url(&[Segment::literal("api"), Segment::literal("contents"), Segment::path(path)])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn list_checkpoints(&self, path: &str) -> Result<Vec<Checkpoint>, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("contents"),
      Segment::path(path),
      Segment::literal("checkpoints"),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn create_checkpoint(&self, path: &str) -> Result<Checkpoint, RestError> {
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

  async fn delete_checkpoint(
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

  async fn get_session(&self, session_id: Uuid) -> Result<Session, RestError> {
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

  async fn delete_session(&self, session_id: Uuid) -> Result<(), RestError> {
    let session = session_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("sessions"),
      Segment::literal(session),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn list_sessions(&self) -> Result<Vec<Session>, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("sessions")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn create_session(&self, session: &Session) -> Result<Session, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("sessions")])?;
    let request = self.request(Method::POST, url).json(session);
    self.send_json(request).await
  }

  async fn list_kernels(&self) -> Result<Vec<Kernel>, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernels")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn start_kernel(&self, options: &KernelStartOptions) -> Result<Kernel, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernels")])?;
    let request = self.request(Method::POST, url).json(options);
    self.send_json(request).await
  }

  async fn get_kernel(&self, kernel_id: Uuid) -> Result<Kernel, RestError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn delete_kernel(&self, kernel_id: Uuid) -> Result<(), RestError> {
    let kernel = kernel_id.to_string();
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("kernels"),
      Segment::literal(kernel),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn interrupt_kernel(&self, kernel_id: Uuid) -> Result<(), RestError> {
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

  async fn restart_kernel(&self, kernel_id: Uuid) -> Result<Kernel, RestError> {
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

  async fn kernel_specs(&self) -> Result<KernelSpecsResponse, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("kernelspecs")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn get_config_section(&self, section_name: &str) -> Result<Value, RestError> {
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
  ) -> Result<Value, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("config"),
      Segment::literal(section_name.to_string()),
    ])?;
    let request = self.request(Method::PATCH, url).json(configuration);
    self.send_json(request).await
  }

  async fn list_terminals(&self) -> Result<Vec<Terminal>, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("terminals")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn create_terminal(&self, name: Option<&str>) -> Result<Terminal, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("terminals")])?;
    let payload = match name {
      Some(value) => json!({ "name": value }),
      None => json!({}),
    };
    let request = self.request(Method::POST, url).json(&payload);
    self.send_json(request).await
  }

  async fn get_terminal(&self, terminal_id: &str) -> Result<Terminal, RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("terminals"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn delete_terminal(&self, terminal_id: &str) -> Result<(), RestError> {
    let url = self.build_url(&[
      Segment::literal("api"),
      Segment::literal("terminals"),
      Segment::literal(terminal_id.to_string()),
    ])?;
    let request = self.request(Method::DELETE, url);
    self.send_empty(request).await
  }

  async fn me(&self, params: Option<&PermissionsQueryParam>) -> Result<MeResponse, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("me")])?;
    let mut request = self.request(Method::GET, url);
    if let Some(query) = params {
      request = request.query(query);
    }
    self.send_json(request).await
  }

  async fn status(&self) -> Result<APIStatus, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("status")])?;
    let request = self.request(Method::GET, url);
    self.send_json(request).await
  }

  async fn download_spec(&self) -> Result<String, RestError> {
    let url = self.build_url(&[Segment::literal("api"), Segment::literal("spec.yaml")])?;
    let request = self.request(Method::GET, url);
    let response = self.send(request).await?;
    response.text().await.map_err(RestError::Http)
  }
}

#[async_trait::async_trait]
pub trait JupyterLabApi {
  async fn get_files_stream(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Response, RestError>;
  async fn get_files(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Vec<u8>, RestError> {
    let response = self.get_files_stream(path, range).await?;
    response.bytes().await.map(|b| b.to_vec()).map_err(RestError::Http)
  }
}

#[async_trait::async_trait]
impl JupyterLabApi for JupyterRestClient {
  async fn get_files_stream(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Response, RestError> {
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
}
