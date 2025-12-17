use std::{
  fmt,
  path::{Component, Path, PathBuf},
  time::SystemTime,
};

use async_trait::async_trait;
use libunftp::{
  auth::DefaultUser,
  storage::{Error, ErrorKind, Fileinfo, Metadata, Permissions, StorageBackend},
  ServerBuilder,
};
use reqwest::StatusCode;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
  api::client::ClientError,
  fs::{Entry, EntryKind, FsError, FsService},
};

/// Convenience alias for configuring a libunftp server backed by a [`FsService`].
pub type FtpServerBuilder = ServerBuilder<FsStorage, DefaultUser>;

/// Construct a libunftp [`ServerBuilder`] that serves files via the Jupyter Contents API.
pub fn server_builder(fs: FsService) -> FtpServerBuilder {
  ServerBuilder::new(Box::new(move || FsStorage::new(fs.clone())))
}

#[derive(Clone)]
pub struct FsStorage {
  fs: FsService,
}

impl FsStorage {
  pub fn new(fs: FsService) -> Self {
    Self { fs }
  }
}

impl fmt::Debug for FsStorage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FsStorage").finish()
  }
}

#[async_trait]
impl StorageBackend<DefaultUser> for FsStorage {
  type Metadata = FsMetadata;

  async fn metadata<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
  ) -> Result<Self::Metadata, Error> {
    let target = normalize_request_path(path);
    trace!(%target, "FTP metadata lookup");
    let entry = self.fs.metadata(&target).await.map_err(map_fs_error)?;
    Ok(FsMetadata::from(entry))
  }

  async fn list<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
  ) -> Result<Vec<Fileinfo<PathBuf, Self::Metadata>>, Error>
  where
    Self::Metadata: Metadata,
  {
    let target = normalize_request_path(path);
    trace!(%target, "FTP directory listing");
    let entries = self.fs.ls(&target).await.map_err(map_fs_error)?;
    Ok(entries.into_iter().map(entry_to_fileinfo).collect())
  }

  async fn get<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
    start_pos: u64,
  ) -> Result<Box<dyn AsyncRead + Send + Sync + Unpin>, Error> {
    let target = normalize_request_path(path);
    debug!(%target, start = start_pos, "FTP file read requested");
    let download = self
      .fs
      .download_reader_from(&target, start_pos)
      .await
      .map_err(map_fs_error)?;
    Ok(download.reader)
  }

  async fn put<P, R>(
    &self,
    _user: &DefaultUser,
    mut input: R,
    path: P,
    start_pos: u64,
  ) -> Result<u64, Error>
  where
    P: AsRef<Path> + Send + fmt::Debug,
    R: AsyncRead + Send + Sync + Unpin + 'static,
  {
    if start_pos != 0 {
      return Err(Error::from(ErrorKind::CommandNotImplemented));
    }
    let target = normalize_request_path(path);
    debug!(%target, start = start_pos, "FTP file write requested");
    let mut buffer = Vec::new();
    input
      .read_to_end(&mut buffer)
      .await
      .map_err(|err| Error::new(ErrorKind::LocalError, err))?;
    let size = buffer.len() as u64;
    self.fs.upload(&target, buffer).await.map_err(map_fs_error)?;
    debug!(%target, bytes = size, "FTP file write completed");
    Ok(size)
  }

  async fn del<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
  ) -> Result<(), Error> {
    let target = normalize_request_path(path);
    debug!(%target, "FTP delete requested");
    self.fs.rm(&target).await.map_err(map_fs_error)
  }

  async fn mkd<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
  ) -> Result<(), Error> {
    let target = normalize_request_path(path);
    debug!(%target, "FTP mkdir requested");
    self.fs.mkdir(&target).await.map_err(map_fs_error)?;
    Ok(())
  }

  async fn rename<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    from: P,
    to: P,
  ) -> Result<(), Error> {
    let source = normalize_request_path(from);
    let dest = normalize_request_path(to);
    debug!(source = %source, dest = %dest, "FTP rename requested");
    self.fs.rename(&source, &dest).await.map_err(map_fs_error)?;
    Ok(())
  }

  async fn rmd<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
  ) -> Result<(), Error> {
    let target = normalize_request_path(path);
    debug!(%target, "FTP rmdir requested");
    self.fs.rmdir(&target, false).await.map_err(map_fs_error)
  }

  async fn cwd<P: AsRef<Path> + Send + fmt::Debug>(
    &self,
    _user: &DefaultUser,
    path: P,
  ) -> Result<(), Error> {
    let target = normalize_request_path(path);
    trace!(%target, "FTP cwd validation");
    let entry = self.fs.metadata(&target).await.map_err(map_fs_error)?;
    if entry.kind.is_directory() {
      Ok(())
    } else {
      Err(Error::from(ErrorKind::PermanentDirectoryNotAvailable))
    }
  }
}

#[derive(Clone)]
pub struct FsMetadata {
  entry: Entry,
}

impl From<Entry> for FsMetadata {
  fn from(entry: Entry) -> Self {
    Self { entry }
  }
}

impl FsMetadata {
  fn is_directory(&self) -> bool {
    matches!(self.entry.kind, EntryKind::Directory)
  }
}

impl Metadata for FsMetadata {
  fn len(&self) -> u64 {
    self.entry.size.unwrap_or(0)
  }

  fn is_dir(&self) -> bool {
    self.is_directory()
  }

  fn is_file(&self) -> bool {
    !self.is_directory()
  }

  fn is_symlink(&self) -> bool {
    false
  }

  fn modified(&self) -> Result<SystemTime, Error> {
    self
      .entry
      .last_modified
      .clone()
      .or_else(|| self.entry.created.clone())
      .map(SystemTime::from)
      .ok_or_else(|| Error::from(ErrorKind::LocalError))
  }

  fn gid(&self) -> u32 {
    0
  }

  fn uid(&self) -> u32 {
    0
  }

  fn permissions(&self) -> Permissions {
    let writable_bits = if self.entry.writable { 0o755 } else { 0o555 };
    Permissions(writable_bits)
  }
}

fn entry_to_fileinfo(entry: Entry) -> Fileinfo<PathBuf, FsMetadata> {
  Fileinfo {
    path: absolute_entry_path(&entry.path),
    metadata: FsMetadata::from(entry),
  }
}

fn absolute_entry_path(raw: &str) -> PathBuf {
  if raw.is_empty() {
    PathBuf::from("/")
  } else if raw.starts_with('/') {
    PathBuf::from(raw)
  } else {
    let mut buf = PathBuf::from("/");
    for part in raw.split('/') {
      if part.is_empty() {
        continue;
      }
      buf.push(part);
    }
    buf
  }
}

fn normalize_request_path<P: AsRef<Path>>(path: P) -> String {
  let mut components = Vec::new();
  for component in path.as_ref().components() {
    match component {
      Component::RootDir | Component::Prefix(_) => {
        components.clear();
      }
      Component::CurDir => {}
      Component::ParentDir => {
        components.pop();
      }
      Component::Normal(part) => {
        components.push(part.to_string_lossy().into_owned());
      }
    }
  }
  if components.is_empty() {
    "/".into()
  } else {
    format!("/{}", components.join("/"))
  }
}

fn map_fs_error(err: FsError) -> Error {
  debug!(error = ?err, "FsService error surfaced to FTP client");
  match err {
    FsError::Client(e) => map_client_error(e),
    FsError::NotAFile(_) => Error::from(ErrorKind::PermanentFileNotAvailable),
    FsError::NotADirectory(_) => Error::from(ErrorKind::PermanentDirectoryNotAvailable),
    FsError::MissingContent(_) | FsError::InvalidPayload(_) => Error::new(ErrorKind::LocalError, err),
    FsError::Decode(inner) => Error::new(ErrorKind::LocalError, inner),
    FsError::NotImplemented(feature) => Error::new(ErrorKind::CommandNotImplemented, feature),
  }
}

fn map_client_error(err: ClientError) -> Error {
  trace!(error = ?err, "mapping Client error to FTP status");
  match err {
    ClientError::Api { status, .. } => match status {
      StatusCode::NOT_FOUND => Error::from(ErrorKind::PermanentFileNotAvailable),
      StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => Error::from(ErrorKind::PermissionDenied),
      StatusCode::CONFLICT => Error::from(ErrorKind::PermanentDirectoryNotEmpty),
      _ => Error::from(ErrorKind::LocalError),
    },
    other => Error::new(ErrorKind::LocalError, other),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Utc;

  #[test]
  fn normalize_handles_relative_segments() {
    assert_eq!(normalize_request_path(""), "/");
    assert_eq!(normalize_request_path("/"), "/");
    assert_eq!(normalize_request_path("folder/child"), "/folder/child");
    assert_eq!(normalize_request_path("/folder/./child"), "/folder/child");
    assert_eq!(normalize_request_path("/folder/../child"), "/child");
    assert_eq!(normalize_request_path("../../nested"), "/nested");
  }

  #[test]
  fn metadata_reflects_entry_kind() {
    let entry = Entry {
      name: "sample".into(),
      path: "sample".into(),
      kind: EntryKind::Directory,
      writable: true,
      created: Some(Utc::now()),
      last_modified: Some(Utc::now()),
      size: Some(10),
      mimetype: None,
      hash: None,
      hash_algorithm: None,
    };
    let metadata = FsMetadata::from(entry.clone());
    assert!(metadata.is_dir());
    assert!(!metadata.is_file());
    assert_eq!(metadata.permissions().0, 0o755);

    let mut file_entry = entry;
    file_entry.kind = EntryKind::File;
    file_entry.writable = false;
    let file_meta = FsMetadata::from(file_entry);
    assert!(!file_meta.is_dir());
    assert!(file_meta.is_file());
    assert_eq!(file_meta.permissions().0, 0o555);
  }
}
