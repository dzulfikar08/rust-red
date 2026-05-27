use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use propex::PropexSegment;
use tokio::sync::{Mutex, RwLock};

use super::{ElementId, RustRedError, Variant};
use crate::Result;
use crate::runtime::context::*;

inventory::submit! {
    ProviderMetadata { type_: "localfs", factory: FileContextStore::build }
}

struct SharedState {
    scopes: RwLock<HashMap<String, Variant>>,
    dirty: RwLock<HashSet<String>>,
    flush_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    base_dir: PathBuf,
    flush_interval_secs: u64,
}

impl SharedState {
    fn scope_to_path(&self, scope: &str) -> PathBuf {
        if scope == GLOBAL_CONTEXT_NAME {
            self.base_dir.join("global").join("global_context.json")
        } else {
            self.base_dir.join(scope).join("context.json")
        }
    }

    async fn flush_scope(&self, scope: &str, data: &Variant) -> Result<()> {
        let path = self.scope_to_path(scope);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let json_bytes =
            serde_json::to_string_pretty(data).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp_path = path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, &json_bytes).await?;
        tokio::fs::rename(&tmp_path, &path).await?;
        log::trace!("[LOCALFS] Flushed scope '{}' to '{}'", scope, path.display());
        Ok(())
    }

    async fn load_scope_from_disk(&self, scope: &str) -> Option<Variant> {
        let path = self.scope_to_path(scope);
        let contents = tokio::fs::read_to_string(&path).await.ok()?;
        serde_json::from_str(&contents).ok()
    }
}

struct FileContextStore {
    name: String,
    state: Arc<SharedState>,
}

impl FileContextStore {
    fn build(name: String, options: Option<&ContextStoreOptions>) -> crate::Result<Box<dyn ContextStore>> {
        let (base_dir, flush_interval_secs) = Self::parse_options(options);
        let state = Arc::new(SharedState {
            scopes: RwLock::new(HashMap::new()),
            dirty: RwLock::new(HashSet::new()),
            flush_handle: Mutex::new(None),
            base_dir,
            flush_interval_secs,
        });
        Ok(Box::new(FileContextStore { name, state }))
    }

    fn parse_options(options: Option<&ContextStoreOptions>) -> (PathBuf, u64) {
        let default_dir = PathBuf::from(".rust-red/context");
        let default_flush_secs: u64 = 30;
        let Some(opts) = options else {
            return (default_dir, default_flush_secs);
        };
        let dir = opts
            .options
            .get("dir")
            .and_then(|v| v.clone().into_string().ok())
            .map(PathBuf::from)
            .unwrap_or(default_dir);
        let flush_secs = opts
            .options
            .get("flush_interval_secs")
            .and_then(|v| v.clone().into_uint().ok())
            .unwrap_or(default_flush_secs);
        (dir, flush_secs)
    }

    async fn ensure_scope_loaded(&self, scope: &str) {
        {
            let scopes = self.state.scopes.read().await;
            if scopes.contains_key(scope) {
                return;
            }
        }
        if let Some(data) = self.state.load_scope_from_disk(scope).await {
            let mut scopes = self.state.scopes.write().await;
            if !scopes.contains_key(scope) {
                scopes.insert(scope.to_string(), data);
            }
        }
    }
}

#[async_trait]
impl ContextStore for FileContextStore {
    async fn name(&self) -> &str {
        &self.name
    }

    async fn open(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.state.base_dir).await?;
        log::info!("[LOCALFS] Opened file context store '{}' at '{}'", self.name, self.state.base_dir.display());
        let state = Arc::clone(&self.state);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(state.flush_interval_secs.max(1)));
            interval.tick().await; // skip first immediate tick
            loop {
                interval.tick().await;
                let dirty_scopes: Vec<String> = {
                    let mut dirty = state.dirty.write().await;
                    dirty.drain().collect()
                };
                if dirty_scopes.is_empty() {
                    continue;
                }
                let scopes = state.scopes.read().await;
                for scope in &dirty_scopes {
                    if let Some(data) = scopes.get(scope) {
                        if let Err(e) = state.flush_scope(scope, data).await {
                            log::error!("[LOCALFS] Failed to flush scope '{}': {}", scope, e);
                            state.dirty.write().await.insert(scope.clone());
                        }
                    }
                }
            }
        });
        let mut flush_guard = self.state.flush_handle.lock().await;
        *flush_guard = Some(handle);
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        {
            let mut flush_guard = self.state.flush_handle.lock().await;
            if let Some(handle) = flush_guard.take() {
                handle.abort();
            }
        }
        let dirty_scopes: Vec<String> = {
            let mut dirty = self.state.dirty.write().await;
            dirty.drain().collect()
        };
        if !dirty_scopes.is_empty() {
            let scopes = self.state.scopes.read().await;
            for scope in &dirty_scopes {
                if let Some(data) = scopes.get(scope) {
                    self.state.flush_scope(scope, data).await?;
                }
            }
        }
        log::info!("[LOCALFS] Closed file context store '{}'", self.name);
        Ok(())
    }

    async fn get_one(&self, scope: &str, path: &[PropexSegment]) -> Result<Variant> {
        self.ensure_scope_loaded(scope).await;
        let scopes = self.state.scopes.read().await;
        if let Some(scope_map) = scopes.get(scope)
            && let Some(value) = scope_map.get_segs(path)
        {
            return Ok(value.clone());
        }
        Err(RustRedError::OutOfRange.into())
    }

    async fn get_many(&self, scope: &str, keys: &[&str]) -> Result<Vec<Variant>> {
        self.ensure_scope_loaded(scope).await;
        let scopes = self.state.scopes.read().await;
        if let Some(scope_map) = scopes.get(scope) {
            let mut result = Vec::with_capacity(keys.len());
            for key in keys {
                if let Some(value) = scope_map.get_nav(key, &[]) {
                    result.push(value.clone());
                }
            }
            return Ok(result);
        }
        Err(RustRedError::OutOfRange.into())
    }

    async fn get_keys(&self, scope: &str) -> Result<Vec<String>> {
        self.ensure_scope_loaded(scope).await;
        let scopes = self.state.scopes.read().await;
        if let Some(scope_map) = scopes.get(scope) {
            return Ok(scope_map.as_object().unwrap().keys().cloned().collect::<Vec<_>>());
        }
        Err(RustRedError::OutOfRange.into())
    }

    async fn set_one(&self, scope: &str, path: &[PropexSegment], value: Variant) -> Result<()> {
        self.ensure_scope_loaded(scope).await;
        {
            let mut scopes = self.state.scopes.write().await;
            let scope_map = scopes.entry(scope.to_string()).or_insert_with(Variant::empty_object);
            scope_map.set_segs_property(path, value, true)?;
        }
        self.state.dirty.write().await.insert(scope.to_string());
        Ok(())
    }

    async fn set_many(&self, scope: &str, pairs: Vec<(String, Variant)>) -> Result<()> {
        self.ensure_scope_loaded(scope).await;
        {
            let mut scopes = self.state.scopes.write().await;
            let scope_map = scopes.entry(scope.to_string()).or_insert_with(Variant::empty_object);
            for (key, value) in pairs {
                let _ = scope_map.as_object_mut().unwrap().insert(key, value);
            }
        }
        self.state.dirty.write().await.insert(scope.to_string());
        Ok(())
    }

    async fn remove_one(&self, scope: &str, path: &[PropexSegment]) -> Result<Variant> {
        self.ensure_scope_loaded(scope).await;
        let result = {
            let mut scopes = self.state.scopes.write().await;
            if let Some(scope_map) = scopes.get_mut(scope) {
                if let Some(value) = scope_map.as_object_mut().unwrap().remove_segs_property(path) {
                    Ok(value)
                } else {
                    Err(RustRedError::OutOfRange.into())
                }
            } else {
                Err(RustRedError::OutOfRange.into())
            }
        };
        if result.is_ok() {
            self.state.dirty.write().await.insert(scope.to_string());
        }
        result
    }

    async fn delete(&self, scope: &str) -> Result<()> {
        {
            let mut scopes = self.state.scopes.write().await;
            scopes.remove(scope);
        }
        {
            let mut dirty = self.state.dirty.write().await;
            dirty.remove(scope);
        }
        let path = self.state.scope_to_path(scope);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            if let Some(parent) = path.parent() {
                let _ = tokio::fs::remove_dir(parent).await;
            }
        }
        Ok(())
    }

    async fn clean(&self, _active_nodes: &[ElementId]) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn build_store(tmp: &TempDir) -> Box<dyn ContextStore> {
        let dir_str = tmp.path().to_string_lossy().to_string();
        let options = ContextStoreOptions {
            provider: "localfs".to_string(),
            options: {
                let mut map = HashMap::new();
                map.insert("dir".to_string(), config::Value::from(dir_str));
                map.insert("flush_interval_secs".to_string(), config::Value::from(300u64));
                map
            },
        };
        FileContextStore::build("file0".to_string(), Some(&options)).unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve_property() {
        let tmp = TempDir::new().unwrap();
        let store = build_store(&tmp);
        store.open().await.unwrap();

        assert!(store.get_one("nodeX", &propex::parse("foo").unwrap()).await.is_err());
        store.set_one("nodeX", &propex::parse("foo").unwrap(), "test".into()).await.unwrap();
        assert_eq!(store.get_one("nodeX", &propex::parse("foo").unwrap()).await.unwrap(), "test".into());

        store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_persist_across_close_and_open() {
        let tmp = TempDir::new().unwrap();
        {
            let store = build_store(&tmp);
            store.open().await.unwrap();
            store.set_one("nodeX", &propex::parse("foo").unwrap(), "bar".into()).await.unwrap();
            store.close().await.unwrap();
        }
        {
            let store = build_store(&tmp);
            store.open().await.unwrap();
            let val = store.get_one("nodeX", &propex::parse("foo").unwrap()).await.unwrap();
            assert_eq!(val, "bar".into());
            store.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_nested_properties() {
        let tmp = TempDir::new().unwrap();
        let store = build_store(&tmp);
        store.open().await.unwrap();

        store.set_one("nodeX", &propex::parse("foo.bar").unwrap(), "test".into()).await.unwrap();
        assert_eq!(
            store.get_one("nodeX", &propex::parse("foo").unwrap()).await.unwrap(),
            json!({"bar": "test"}).into()
        );

        store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_scope() {
        let tmp = TempDir::new().unwrap();
        let store = build_store(&tmp);
        store.open().await.unwrap();

        store.set_one("scope1", &propex::parse("key").unwrap(), "value".into()).await.unwrap();
        store.delete("scope1").await.unwrap();
        assert!(store.get_one("scope1", &propex::parse("key").unwrap()).await.is_err());

        store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_scope_isolation() {
        let tmp = TempDir::new().unwrap();
        let store = build_store(&tmp);
        store.open().await.unwrap();

        store.set_one("nodeX", &propex::parse("foo").unwrap(), "testX".into()).await.unwrap();
        store.set_one("nodeY", &propex::parse("foo").unwrap(), "testY".into()).await.unwrap();

        assert_eq!(store.get_one("nodeX", &propex::parse("foo").unwrap()).await.unwrap(), "testX".into());
        assert_eq!(store.get_one("nodeY", &propex::parse("foo").unwrap()).await.unwrap(), "testY".into());

        store.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_global_scope_persistence() {
        let tmp = TempDir::new().unwrap();
        {
            let store = build_store(&tmp);
            store.open().await.unwrap();
            store.set_one("global", &propex::parse("counter").unwrap(), 42.into()).await.unwrap();
            store.close().await.unwrap();
        }
        {
            let store = build_store(&tmp);
            store.open().await.unwrap();
            let val = store.get_one("global", &propex::parse("counter").unwrap()).await.unwrap();
            assert_eq!(val, 42.into());
            store.close().await.unwrap();
        }
    }
}
