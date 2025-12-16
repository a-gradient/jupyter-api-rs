use crate::api::{
  client::*, param::*, resp::*
};
use reqwest::Method;
use serde_json::{json, Value};
use uuid::Uuid;

impl JupyterRestClient {
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
}
