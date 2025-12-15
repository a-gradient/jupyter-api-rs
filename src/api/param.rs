use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Supported content formats for payloads handled by the Contents API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentsFormat {
  Json,
  Text,
  Base64,
}

/// Type filter accepted by `GET /api/contents/{path}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentsTypeQuery {
  File,
  Directory,
}

/// Resource kinds that can be created or saved via the Contents service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentsEntryType {
  Directory,
  File,
  Notebook,
}

/// Query parameters accepted by `GET /api/contents/{path}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ContentsGetParams {
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub entry_type: Option<ContentsTypeQuery>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub format: Option<ContentsFormat>,
  #[serde(
    default,
    skip_serializing_if = "Option::is_none",
    with = "opt_bool_as_int"
  )]
  pub content: Option<bool>,
  #[serde(
    default,
    skip_serializing_if = "Option::is_none",
    with = "opt_bool_as_int"
  )]
  pub hash: Option<bool>,
}

/// Body accepted by `POST /api/contents/{path}` for copy/creation requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CreateContentsModel {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub copy_from: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ext: Option<String>,
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub entry_type: Option<ContentsEntryType>,
}

/// Payload accepted by `PATCH /api/contents/{path}` to rename entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenameContentsModel {
  pub path: String,
}

/// Payload accepted by `PUT /api/contents/{path}` for saving/uploading files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SaveContentsModel {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub entry_type: Option<ContentsEntryType>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub format: Option<ContentsFormat>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<String>,
}

/// Body accepted by `POST /api/kernels` when starting a new kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelStartOptions {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
}

/// Arbitrary configuration object used by `PATCH /api/config/{section_name}`.
pub type ConfigPatchRequest = HashMap<String, Value>;

/// Optional permissions filter accepted by `GET /api/me`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PermissionsQueryParam {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub permissions: Option<String>,
}

mod opt_bool_as_int {
  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize<S>(value: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    match value {
      Some(flag) => {
        let encoded: u8 = if *flag { 1 } else { 0 };
        serializer.serialize_some(&encoded)
      }
      None => serializer.serialize_none(),
    }
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
  where
    D: Deserializer<'de>,
  {
    let raw = Option::<u8>::deserialize(deserializer)?;
    Ok(raw.map(|value| value != 0))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serialize_contents_get_params_flags() {
    let params = ContentsGetParams {
      entry_type: Some(ContentsTypeQuery::File),
      format: Some(ContentsFormat::Text),
      content: Some(true),
      hash: Some(false),
    };

    let value = serde_json::to_value(&params).unwrap();
    assert_eq!(value.get("type"), Some(&serde_json::Value::from("file")));
    assert_eq!(value.get("format"), Some(&serde_json::Value::from("text")));
    assert_eq!(value.get("content"), Some(&serde_json::Value::from(1)));
    assert_eq!(value.get("hash"), Some(&serde_json::Value::from(0)));
  }
}
