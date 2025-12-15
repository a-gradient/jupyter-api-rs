use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Permission map keyed by resource name where each entry holds allowed actions.
pub type Permissions = HashMap<String, Vec<String>>;

/// Generic key/value blob for kernel resources (e.g., logo paths, JS/CSS assets).
pub type KernelResources = HashMap<String, String>;

/// Collection wrapper for kernelspec listings keyed by spec name.
pub type KernelSpecMap = HashMap<String, KernelSpec>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct APIStatus {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub started: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_activity: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub connections: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kernels: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Identity {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub username: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub display_name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub initials: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub avatar_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MeResponse {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub identity: Option<Identity>,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub permissions: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HelpLink {
  pub text: String,
  pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KernelSpecFile {
  pub language: String,
  pub argv: Vec<String>,
  #[serde(rename = "display_name")]
  pub display_name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub codemirror_mode: Option<Value>,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub env: HashMap<String, String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub help_links: Vec<HelpLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KernelSpec {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(
    rename = "KernelSpecFile",
    alias = "spec",
    skip_serializing_if = "Option::is_none"
  )]
  pub spec: Option<KernelSpecFile>,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub resources: KernelResources,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct KernelSpecsResponse {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default: Option<String>,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub kernelspecs: KernelSpecMap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Kernel {
  pub id: String,
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_activity: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub connections: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub execution_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
  pub session_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kernel: Option<Kernel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contents {
  pub name: String,
  pub path: String,
  #[serde(rename = "type")]
  pub content_type: String,
  pub writable: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub created: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_modified: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub mimetype: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub format: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hash: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hash_algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Checkpoint {
  pub id: String,
  pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Terminal {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_activity: Option<String>,
}
