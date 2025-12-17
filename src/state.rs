use std::sync::{Arc, atomic::AtomicU64};

use futures_util::join;
use ordermap::{Equivalent, OrderMap};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::api::{client::{ClientError, JupyterLabClient}, jupyter::JupyterApi, resp::{Kernel, Session, Terminal}};

pub struct Cached<K, V> {
  map: RwLock<OrderMap<K, V>>,
  pub last_updated: AtomicU64,
}

impl<K, V> Default for Cached<K, V> {
  fn default() -> Self {
    Self { map: Default::default(), last_updated: Default::default() }
  }
}

impl<K: std::hash::Hash + Eq, V> Cached<K, V> {
  pub fn now() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
  }

  pub fn clear(&self) {
    self.map.write().clear();
    self.last_updated.store(Self::now(), std::sync::atomic::Ordering::SeqCst);
  }

  pub fn insert(&self, key: K, value: V) -> Option<V> {
    let old = self.map.write().insert(key, value);
    self.last_updated.store(Self::now(), std::sync::atomic::Ordering::SeqCst);
    old
  }

  pub fn update<I: IntoIterator<Item = (K, V)>>(&self, iter: I) {
    let mut map = self.map.write();
    map.clear();
    for (k, v) in iter {
      map.insert(k, v);
    }
    self.last_updated.store(Self::now(), std::sync::atomic::Ordering::SeqCst);
  }

  pub fn get<Q>(&self, key: &Q) -> Option<V>
  where
    Q: ?Sized + std::hash::Hash + Equivalent<K>,
    V: Clone,
  {
    self.map.read().get(key).cloned()
  }
}

pub struct State {
  pub client: Arc<JupyterLabClient>,
  pub kernels: Cached<Uuid, Kernel>,
  pub sessions: Cached<Uuid, Session>,
  pub terminals: Cached<String, Terminal>,
}

impl State {
  pub fn new(client: Arc<JupyterLabClient>) -> Self {
    Self {
      client,
      kernels: Cached::default(),
      sessions: Cached::default(),
      terminals: Cached::default(),
    }
  }

  pub async fn update_sessions(&self) -> Result<(), ClientError> {
    let sessions = self.client.list_sessions().await?;
    self.sessions.update(sessions.into_iter().filter_map(|s| {
      let id = s.id.or_else(|| s.kernel.as_ref().map(|k| k.id));
      id.map(|id| (id, s))
    }));
    Ok(())
  }

  pub async fn update_kernels(&self) -> Result<(), ClientError> {
    let kernels = self.client.list_kernels().await?;
    self.kernels.update(kernels.into_iter().map(|k| (k.id, k)));
    Ok(())
  }

  pub async fn update_terminals(&self) -> Result<(), ClientError> {
    let terminals = self.client.list_terminals().await?;
    self.terminals.update(terminals.into_iter().map(|t| (t.name.clone(), t)));
    Ok(())
  }

  pub async fn refresh_all(&self) -> Result<(), ClientError> {
    let result = join!(
      self.update_kernels(),
      self.update_sessions(),
      self.update_terminals(),
    );
    result.0?;
    result.1?;
    result.2?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::api::client::tests::_setup_client;

use super::*;

  #[test]
  fn test_cached_insert_get() {
    let cache = Cached::<String, i32>::default();
    assert!(cache.get("key1").is_none());
    cache.insert("key1".to_string(), 42);
    assert_eq!(cache.get("key1"), Some(42));
  }

  #[tokio::test]
  async fn test_state_refresh() {
    let client = _setup_client();
    let state = State::new(Arc::new(client));
    state.refresh_all().await.unwrap();
    println!("Kernels: {:?}", state.kernels.map.read().keys());
    println!("Sessions: {:?}", state.sessions.map.read().keys());
    println!("Terminals: {:?}", state.terminals.map.read().keys());
  }
}
