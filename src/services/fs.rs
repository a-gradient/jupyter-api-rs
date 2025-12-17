use std::{fmt, pin::Pin, sync::Arc};
use std::io;
use std::task::{Context, Poll};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio_util::io::StreamReader;
use futures_util::TryStreamExt;

use crate::api::{
  client::{JupyterLabClient, ClientError}, jupyter::{JupyterApi, JupyterLabApi}, param::{ContentsEntryType, ContentsFormat, ContentsGetParams, RenameContentsModel, SaveContentsModel}, resp::{ContentValue, Contents}
};

/// High-level convenience helpers for interacting with the Jupyter contents API
/// using file system-like verbs.
#[derive(Clone)]
pub struct FsService {
  inner: Arc<JupyterLabClient>,
}

impl FsService {
  pub fn new(inner: Arc<JupyterLabClient>) -> Self {
    Self { inner }
  }

  /// List directory contents or return metadata for a single file.
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn ls(&self, path: &str) -> Result<Vec<Entry>, FsError> {
    debug!("fs: ls {}", path);
    let mut params = ContentsGetParams::default();
    params.content = Some(true);
    let contents = self
      .inner
      .get_contents(path, Some(&params))
      .await
      .map_err(FsError::from)?;

    if EntryKind::from_content_type(&contents.content_type).is_directory() {
      return match contents.content {
        Some(ContentValue::Contents(entries)) => {
          trace!(entry_count = entries.len(), "directory listing resolved");
          Ok(entries.into_iter().map(Entry::from).collect())
        }
        Some(ContentValue::Text(_)) => Err(FsError::InvalidPayload(contents.path)),
        None => Err(FsError::MissingContent(contents.path)),
      };
    }

    trace!("path is a file; returning metadata only");
    Ok(vec![Entry::from(contents)])
  }

  /// Fetch metadata for a path without downloading its payload.
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn metadata(&self, path: &str) -> Result<Entry, FsError> {
    debug!("fs: metadata {}", path);
    let mut params = ContentsGetParams::default();
    params.content = Some(false);
    let contents = self
      .inner
      .get_contents(path, Some(&params))
      .await
      .map_err(FsError::from)?;
    let entry = Entry::from(contents);
    trace!(kind = ?entry.kind, "metadata fetched");
    Ok(entry)
  }

  /// Upload raw bytes to the given Jupyter path, creating or overwriting a file.
  #[tracing::instrument(skip(self, data), fields(path = %path, chunk = ?chunk))]
  async fn _upload(&self, path: &str, data: impl AsRef<[u8]>, chunk: Option<isize>) -> Result<Entry, FsError> {
    let payload = data.as_ref();
    trace!(bytes = payload.len(), "uploading chunk to Jupyter contents service");
    let encoded = STANDARD.encode(payload);
    let mut model = SaveContentsModel::default();
    model.entry_type = Some(ContentsEntryType::File);
    model.format = Some(ContentsFormat::Base64);
    model.content = Some(encoded);
    model.chunk = chunk;

    let contents = self
      .inner
      .save_contents(path, &model)
      .await
      .map_err(FsError::from)?;
    Ok(Entry::from(contents))
  }

  fn _check_uploaded(&self, entry: &Entry, total_len: u64) -> Result<(), FsError> {
    trace!(remote_size = ?entry.size, expected = total_len, path = %entry.path, "validating uploaded file length");
    if let Some(uploaded_len) = entry.size && uploaded_len != total_len {
      return Err(FsError::InvalidPayload(format!(
        "uploaded chunk size mismatch for {}: expected {}, got {}",
        entry.path,
        total_len,
        uploaded_len
      )));
    }
    Ok(())
  }

  #[tracing::instrument(skip(self, data), fields(path = %path))]
  pub async fn upload(&self, path: &str, data: impl AsRef<[u8]>) -> Result<Entry, FsError> {
    let data = data.as_ref();
    debug!(len=data.len(), "fs: upload {}", path);
    let total_len = data.len() as u64;
    trace!(bytes = total_len, "uploading file in a single request");
    let entry = self._upload(path, data, None).await?;
    self._check_uploaded(&entry, total_len)?;
    Ok(entry)

  }

  #[tracing::instrument(skip(self, data), fields(path = %path, chunk_size = chunk_size))]
  pub async fn upload_chunked(&self, path: &str, data: impl AsRef<[u8]>, chunk_size: u64) -> Result<Entry, FsError> {
    let data = data.as_ref();
    debug!(len=data.len(), chunk_size, "fs: upload_chunked {}", path);
    let total_len = data.len() as u64;
    trace!(bytes = total_len, chunk_size = chunk_size, "uploading file in chunks");
    let mut offset = 0u64;
    for idx in 1.. {
      let end = (offset + chunk_size).min(total_len);
      let chunk_data = &data[offset as usize..end as usize];
      let is_last_chunk = end >= total_len;
      let chunk_idx = if is_last_chunk { -1 } else { idx };
      trace!(chunk_idx = chunk_idx, offset = offset, end = end, is_last_chunk = is_last_chunk, "uploading chunk");
      let entry = self._upload(path, chunk_data, Some(chunk_idx)).await?;
      // println!("uploaded {idx} chunk {offset}-{end} => {:?}", entry.size);
      offset = end;
      if is_last_chunk {
        self._check_uploaded(&entry, offset)?;
        return Ok(entry)
      }
    }
    unreachable!()
  }

  /// Download a remote file/notebook and return its bytes along with metadata.
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn _download_use_contents(&self, path: &str) -> Result<FileContent, FsError> {
    trace!("downloading via /api/contents endpoint");
    let mut params = ContentsGetParams::default();
    params.content = Some(true);
    params.format = Some(ContentsFormat::Base64);

    let mut contents = self
      .inner
      .get_contents(path, Some(&params))
      .await
      .map_err(FsError::from)?;

    let kind = EntryKind::from_content_type(&contents.content_type);
    if !kind.is_file_like() {
      return Err(FsError::NotAFile(contents.path));
    }

    let payload = contents
      .content
      .take()
      .ok_or_else(|| FsError::MissingContent(contents.path.clone()))?;
    let bytes = decode_file_bytes(contents.format.as_deref(), payload)?;
    let entry = Entry::from(contents);
    trace!(remote_size = ?entry.size, "downloaded payload via contents endpoint");
    Ok(FileContent { entry, bytes })
  }

  #[tracing::instrument(skip(self), fields(path = %path, range = ?range))]
  pub async fn _download_use_files(&self, path: &str, range: Option<(u64, Option<u64>)>) -> Result<Vec<u8>, FsError> {
    trace!("downloading via /files endpoint");
    Ok(self.inner.get_files(path, range).await?)
  }

  #[tracing::instrument(skip(self), fields(path = %path, range = ?range))]
  async fn _download_use_files_reader(
    &self,
    path: &str,
    range: Option<(u64, Option<u64>)>,
  ) -> Result<Box<dyn AsyncRead + Send + Sync + Unpin>, FsError> {
    trace!("streaming via /files endpoint");
    let response = self.inner.get_files_stream(path, range).await?;
    let stream = response
      .bytes_stream()
      .map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    Ok(Box::new(StreamReader::new(stream)))
  }

  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn download(&self, path: &str) -> Result<FileContent, FsError> {
    debug!("fs: download {}", path);
    trace!("attempting optimized /files download");
    if let Ok(payload) = self._download_use_files(path, None).await {
      let entry = self.metadata(path).await?;
      trace!("downloaded via /files endpoint");
      return Ok(FileContent { entry, bytes: payload } );
    }
    trace!("falling back to contents fallback download");
    self._download_use_contents(path).await
  }

  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn download_reader(&self, path: &str) -> Result<FileDownload, FsError> {
    self.download_reader_from(path, 0).await
  }

  #[tracing::instrument(skip(self), fields(path = %path, start = start_pos))]
  pub async fn download_reader_from(&self, path: &str, start_pos: u64) -> Result<FileDownload, FsError> {
    debug!(start = start_pos, "fs: download_reader {}", path);
    let range = (start_pos > 0).then_some((start_pos, None));
    match self._download_use_files_reader(path, range).await {
      Ok(reader) => {
        let entry = self.metadata(path).await?;
        trace!("streamed via /files endpoint");
        Ok(FileDownload { entry, reader })
      }
      Err(err) => {
        trace!(error = ?err, "streaming via /files failed; falling back to contents endpoint");
        let FileContent { entry, mut bytes } = self._download_use_contents(path).await?;
        if start_pos > 0 {
          let offset = usize::try_from(start_pos).map_err(|_| {
            FsError::InvalidPayload(format!(
              "requested offset {} exceeds platform capacity for {}",
              start_pos, path
            ))
          })?;
          if offset > bytes.len() {
            return Err(FsError::InvalidPayload(format!(
              "requested offset {} exceeds file length {} for {}",
              start_pos,
              bytes.len(),
              path
            )));
          }
          bytes.drain(0..offset);
        }
        let reader = boxed_vec_reader(bytes);
        Ok(FileDownload { entry, reader })
      }
    }
  }

  /// Fetch the server-provided hash for a file (without downloading content).
  ///
  /// Note: Jupyter decides which algorithm to return (via `hash_algorithm`).
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn remote_hashsum(&self, path: &str) -> Result<(String, String), FsError> {
    trace!("fetching remote hash via contents API");
    let mut params = ContentsGetParams::default();
    params.content = Some(false);
    params.hash = Some(true);

    let contents = self
      .inner
      .get_contents(path, Some(&params))
      .await
      .map_err(FsError::from)?;

    let kind = EntryKind::from_content_type(&contents.content_type);
    if !kind.is_file_like() {
      return Err(FsError::NotAFile(contents.path));
    }

    let digest = contents.hash.ok_or_else(|| {
      FsError::InvalidPayload(format!(
        "server did not return hash for {}",
        contents.path
      ))
    })?;

    let algorithm = contents.hash_algorithm.ok_or_else(|| {
      FsError::InvalidPayload(format!(
        "server did not return hash_algorithm for {}",
        contents.path
      ))
    })?;

    Ok((algorithm, digest))
  }

  /// Compute the SHA-256 hash for a file.
  ///
  /// Prefers a server-provided SHA-256 from `GET /api/contents` (hash=true).
  /// Falls back to streaming the file and computing SHA-256 locally.
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn sha256sum(&self, path: &str) -> Result<String, FsError> {
    match self.remote_hashsum(path).await {
      Ok((algorithm, digest)) if algorithm.eq_ignore_ascii_case("sha256") => {
        trace!("using server-provided sha256");
        return Ok(digest);
      }
      Ok((algorithm, _)) => {
        trace!(hash_algorithm = %algorithm, "server returned non-sha256 hash; computing local sha256");
      }
      Err(err) => {
        trace!(error = ?err, "server hash unavailable; computing local sha256");
      }
    }

    let mut download = self.download_reader(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
      let read = download
        .reader
        .read(&mut buf)
        .await
        .map_err(|err| FsError::InvalidPayload(format!("failed to read {}: {}", path, err)))?;
      if read == 0 {
        break;
      }
      hasher.update(&buf[..read]);
    }
    let digest = format!("{:x}", hasher.finalize());
    trace!("completed sha256 hash");
    Ok(digest)
  }

  /// Remove a file or directory from the Jupyter server.
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn rm(&self, path: &str) -> Result<(), FsError> {
    trace!("deleting entry via contents API");
    self
      .inner
      .delete_contents(path)
      .await
      .map_err(FsError::from)?;
    Ok(())
  }

  /// Create a directory at the provided fully-qualified Jupyter path.
  #[tracing::instrument(skip(self), fields(path = %path))]
  pub async fn mkdir(&self, path: &str) -> Result<Entry, FsError> {
    trace!("creating directory");
    let mut model = SaveContentsModel::default();
    model.entry_type = Some(ContentsEntryType::Directory);
    let contents = self
      .inner
      .save_contents(path, &model)
      .await
      .map_err(FsError::from)?;
    Ok(Entry::from(contents))
  }

  /// Rename or move an entry to a new path.
  #[tracing::instrument(skip(self), fields(from = %from, to = %to))]
  pub async fn rename(&self, from: &str, to: &str) -> Result<Entry, FsError> {
    trace!("renaming entry");
    let payload = RenameContentsModel {
      path: trim_leading_slash(to).to_string(),
    };
    let contents = self
      .inner
      .rename_contents(from, &payload)
      .await
      .map_err(FsError::from)?;
    Ok(Entry::from(contents))
  }

  /// Remove a directory after verifying the target is not a plain file.
  #[tracing::instrument(skip(self), fields(path = %path, recursive = recursive))]
  pub async fn rmdir(&self, path: &str, recursive: bool) -> Result<(), FsError> {
    debug!(recursive, "fs: rmdir {}", path);
    let mut params = ContentsGetParams::default();
    params.content = Some(!recursive);
    let metadata = self
      .inner
      .get_contents(path, Some(&params))
      .await
      .map_err(FsError::from)?;
    if !EntryKind::from_content_type(&metadata.content_type).is_directory() {
      return Err(FsError::NotADirectory(metadata.path));
    }
    if let Some(ContentValue::Contents(v)) = metadata.content && v.len() > 0 {
      return Err(FsError::InvalidPayload(format!(
        "directory {} is not empty",
        metadata.path
      )));
    }
    self
      .inner
      .delete_contents(path)
      .await
      .map_err(FsError::from)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
  File,
  Directory,
  Notebook,
  Other(String),
}

impl EntryKind {
  fn from_content_type(value: &str) -> Self {
    match value {
      "file" => EntryKind::File,
      "directory" => EntryKind::Directory,
      "notebook" => EntryKind::Notebook,
      other => EntryKind::Other(other.to_string()),
    }
  }

  pub fn is_directory(&self) -> bool {
    matches!(self, EntryKind::Directory)
  }

  pub fn is_file_like(&self) -> bool {
    matches!(self, EntryKind::File | EntryKind::Notebook | EntryKind::Other(_))
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
  pub name: String,
  pub path: String,
  pub kind: EntryKind,
  pub writable: bool,
  pub created: Option<DateTime<Utc>>,
  pub last_modified: Option<DateTime<Utc>>,
  pub size: Option<u64>,
  pub mimetype: Option<String>,
  pub hash: Option<String>,
  pub hash_algorithm: Option<String>,
}

impl Entry {
  fn from(contents: Contents) -> Self {
    let Contents {
      name,
      path,
      content_type,
      writable,
      created,
      last_modified,
      size,
      mimetype,
      hash,
      hash_algorithm,
      ..
    } = contents;

    Entry {
      name,
      path,
      kind: EntryKind::from_content_type(&content_type),
      writable,
      created,
      last_modified,
      size,
      mimetype,
      hash,
      hash_algorithm,
    }
  }
}

pub struct FileDownload {
  pub entry: Entry,
  pub reader: Box<dyn AsyncRead + Unpin + Send + Sync>,
}

fn boxed_vec_reader(bytes: Vec<u8>) -> Box<dyn AsyncRead + Send + Sync + Unpin> {
  Box::new(VecReader::new(bytes))
}

struct VecReader {
  data: Arc<[u8]>,
  position: usize,
}

impl VecReader {
  fn new(bytes: Vec<u8>) -> Self {
    Self {
      data: Arc::from(bytes),
      position: 0,
    }
  }
}

impl AsyncRead for VecReader {
  fn poll_read(
    mut self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<Result<(), std::io::Error>> {
    if self.position >= self.data.len() {
      return Poll::Ready(Ok(()));
    }
    let remaining = &self.data[self.position..];
    let len = remaining.len().min(buf.remaining());
    buf.put_slice(&remaining[..len]);
    self.position += len;
    Poll::Ready(Ok(()))
  }
}

#[derive(Debug, Clone)]
pub struct FileContent {
  pub entry: Entry,
  pub bytes: Vec<u8>,
}

fn decode_file_bytes(format: Option<&str>, payload: ContentValue) -> Result<Vec<u8>, FsError> {
  match payload {
    ContentValue::Text(data) => match format.unwrap_or("text") {
      "base64" => {
        STANDARD.decode(data.trim()).map_err(FsError::from)
      },
      _ => Ok(data.into_bytes()),
    },
    ContentValue::Contents(_) => Err(FsError::InvalidPayload(
      "expected file payload, received directory listing".into(),
    )),
  }
}

fn trim_leading_slash(path: &str) -> &str {
  let trimmed = path.trim_start_matches('/');
  if trimmed.is_empty() {
    path.trim_matches('/')
  } else {
    trimmed
  }
}

#[derive(Debug)]
pub enum FsError {
  Client(ClientError),
  NotAFile(String),
  NotADirectory(String),
  MissingContent(String),
  InvalidPayload(String),
  Decode(base64::DecodeError),
  NotImplemented(String),
}

impl fmt::Display for FsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FsError::Client(err) => write!(f, "client api error: {err}"),
      FsError::NotAFile(path) => write!(f, "{path} is not a file"),
      FsError::NotADirectory(path) => write!(f, "{path} is not a directory"),
      FsError::MissingContent(path) => write!(f, "no content returned for {path}"),
      FsError::InvalidPayload(reason) => write!(f, "invalid payload: {reason}"),
      FsError::Decode(err) => write!(f, "failed to decode file payload: {err}"),
      FsError::NotImplemented(feature) => write!(f, "not implemented: {feature}"),
    }
  }
}

impl std::error::Error for FsError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      FsError::Client(err) => Some(err),
      FsError::Decode(err) => Some(err),
      _ => None,
    }
  }
}

impl From<ClientError> for FsError {
  fn from(value: ClientError) -> Self {
    FsError::Client(value)
  }
}

impl From<base64::DecodeError> for FsError {
  fn from(value: base64::DecodeError) -> Self {
    FsError::Decode(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_contents(kind: &str) -> Contents {
    Contents {
      name: "example".into(),
      path: "example".into(),
      content_type: kind.into(),
      writable: true,
      created: None,
      last_modified: None,
      size: Some(42),
      mimetype: Some("text/plain".into()),
      content: None,
      format: Some("text".into()),
      hash: Some("abc".into()),
      hash_algorithm: Some("sha256".into()),
    }
  }

  #[test]
  fn entry_kind_mapping() {
    assert!(EntryKind::from_content_type("directory").is_directory());
    assert!(EntryKind::from_content_type("file").is_file_like());
  }

  #[test]
  fn entry_from_contents_transfers_metadata() {
    let entry = Entry::from(sample_contents("file"));
    assert_eq!(entry.kind, EntryKind::File);
    assert_eq!(entry.size, Some(42));
    assert_eq!(entry.mimetype.as_deref(), Some("text/plain"));
    assert_eq!(entry.hash_algorithm.as_deref(), Some("sha256"));
  }

  #[test]
  fn decode_base64_payload_to_bytes() {
    let encoded = STANDARD.encode("payload");
    let bytes = decode_file_bytes(Some("base64"), ContentValue::Text(encoded)).unwrap();
    assert_eq!(bytes, b"payload");
    let bytes = decode_file_bytes(Some("base64"), ContentValue::Text("MTIz".into())).unwrap();
    assert_eq!(bytes, b"123");
  }

  #[test]
  fn decode_text_payload_to_bytes() {
    let bytes = decode_file_bytes(Some("text"), ContentValue::Text("hello".into())).unwrap();
    assert_eq!(bytes, b"hello");
  }

  #[tokio::test]
  async fn test_ls_directory() {
    let client = crate::api::client::tests::_setup_client();
    let fs = FsService::new(Arc::new(client));
    fs.rm("1.txt").await.ok();
    let result = fs.ls("/Untitled Folder").await.unwrap();
    println!("Directory listing: {:?}", result.iter().map(|e| &e.name).collect::<Vec<_>>());
    let entry = fs.upload("Untitled Folder/1.txt", "123").await.unwrap();
    println!("Uploaded entry: {:?}", entry);
    let download = fs.download("Untitled Folder/1.txt").await.unwrap();
    println!("Downloaded entry: {:?}", download);
    let entries2 = fs.ls("/Untitled Folder").await.unwrap();
    assert_eq!(entries2.len(), result.len() + 1);
    assert!(entries2.iter().any(|e| e.name == "1.txt"));
    fs.rm("Untitled Folder/1.txt").await.unwrap();
    let entries2 = fs.ls("/Untitled Folder").await.unwrap();
    assert_eq!(entries2.len(), result.len());
  }

  #[tokio::test]
  async fn test_upload_chunked() {
    let client = crate::api::client::tests::_setup_client();
    let fs = FsService::new(Arc::new(client));

    fs.rm("chunked.txt").await.ok();
    let data = b"The quick brown fox jumps over the lazy dog".to_vec();
    let entry = fs.upload_chunked("chunked.txt", &data, 10).await.unwrap();
    assert_eq!(entry.size, Some(data.len() as u64));
    let download = fs.download("chunked.txt").await.unwrap();
    assert_eq!(download.bytes, data);
    fs.rm("chunked.txt").await.unwrap();
  }

  #[tokio::test]
  async fn test_dir() {
    let client = crate::api::client::tests::_setup_client();
    let fs = FsService::new(Arc::new(client));

    fs.rmdir("test_dir", true).await.ok();
    let dir_entry = fs.mkdir("test_dir").await.unwrap();
    assert!(dir_entry.kind.is_directory());

    let metadata = fs.metadata("test_dir").await.unwrap();
    assert!(metadata.kind.is_directory());

    let file_entry = fs.upload("test_dir/file.txt", "hello").await.unwrap();
    assert!(file_entry.kind.is_file_like());

    let dir_listing = fs.ls("test_dir").await.unwrap();
    assert_eq!(dir_listing.len(), 1);
    assert_eq!(dir_listing[0].name, "file.txt");

    fs.rmdir("test_dir", false).await.unwrap_err(); // should fail because not empty
    fs.rm("test_dir/file.txt").await.unwrap();
    fs.rmdir("test_dir", false).await.unwrap(); // should succeed now
  }
}
