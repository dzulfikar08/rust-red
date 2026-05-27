use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    FlowDeploy,
    FlowDelete,
    NodeCreate,
    NodeDelete,
    UserLogin,
    UserLogout,
    LoginFailure,
    ConfigChange,
    PluginLoad,
    PluginUnload,
    AccessDenied,
    ApiKeyCreated,
    ApiKeyRevoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user: Option<String>,
    pub ip_address: Option<String>,
    pub details: serde_json::Value,
    pub success: bool,
}

impl AuditEvent {
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            user: None,
            ip_address: None,
            details: serde_json::Value::Null,
            success: true,
        }
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_audit_path")]
    pub path: String,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self { enabled: true, path: default_audit_path(), max_file_size_mb: default_max_file_size_mb() }
    }
}

impl AuditConfig {
    pub fn load(cfg: &config::Config) -> Self {
        match cfg.get::<Self>("audit") {
            Ok(s) => s,
            Err(config::ConfigError::NotFound(_)) => {
                log::info!("[audit] config section not found, using defaults");
                Self::default()
            }
            Err(e) => {
                log::warn!("[audit] config parse error: {e}, using defaults");
                Self::default()
            }
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_audit_path() -> String {
    "./audit.log".to_string()
}

fn default_max_file_size_mb() -> u64 {
    100
}

#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_event(&self, event: AuditEvent);
}

#[derive(Debug)]
pub struct FileAuditLogger {
    config: AuditConfig,
    write_lock: tokio::sync::Mutex<()>,
}

impl FileAuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self { config, write_lock: tokio::sync::Mutex::new(()) }
    }

    async fn should_rotate(&self) -> bool {
        let path = Path::new(&self.config.path);
        match tokio::fs::metadata(path).await {
            Ok(meta) => {
                let size_mb = meta.len() / (1024 * 1024);
                size_mb >= self.config.max_file_size_mb
            }
            Err(_) => false,
        }
    }

    async fn rotate(&self) {
        let path = Path::new(&self.config.path);
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let rotated_path = format!("{}.{}", self.config.path, timestamp);

        if let Err(e) = tokio::fs::rename(path, Path::new(&rotated_path)).await {
            log::error!("[audit] failed to rotate log file: {e}");
        } else {
            log::info!("[audit] rotated log to {rotated_path}");
        }
    }

    async fn write_event(&self, event: &AuditEvent) -> std::io::Result<()> {
        let _guard = self.write_lock.lock().await;

        if self.should_rotate().await {
            self.rotate().await;
        }

        let mut line =
            serde_json::to_string(event).unwrap_or_else(|_| "{{\"error\":\"serialization failed\"}}".to_string());
        line.push('\n');

        // Ensure parent directory exists
        if let Some(parent) = Path::new(&self.config.path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Use OpenOptions for append, creating the file if missing
        let file = tokio::fs::OpenOptions::new().create(true).append(true).open(&self.config.path).await?;

        let mut writer = tokio::io::BufWriter::new(file);
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl AuditLogger for FileAuditLogger {
    async fn log_event(&self, event: AuditEvent) {
        if !self.config.enabled {
            return;
        }

        if let Err(e) = self.write_event(&event).await {
            log::error!("[audit] failed to write event: {e}");
        }
    }
}

/// A no-op logger used when audit logging is disabled.
pub struct NullAuditLogger;

#[async_trait::async_trait]
impl AuditLogger for NullAuditLogger {
    async fn log_event(&self, _event: AuditEvent) {}
}

/// Convenience function to create the appropriate logger from config.
pub fn create_audit_logger(config: AuditConfig) -> Box<dyn AuditLogger> {
    if config.enabled {
        Box::new(FileAuditLogger::new(config))
    } else {
        log::info!("[audit] logging disabled");
        Box::new(NullAuditLogger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_log_event_writes_jsonl() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let config = AuditConfig { enabled: true, path: path.clone(), max_file_size_mb: 100 };
        let logger = FileAuditLogger::new(config);

        let event = AuditEvent::new(AuditEventType::FlowDeploy)
            .user("admin")
            .ip_address("127.0.0.1")
            .details(serde_json::json!({"flow_id": "abc123"}))
            .success(true);

        logger.log_event(event).await;

        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["event_type"], "flow_deploy");
        assert_eq!(parsed["user"], "admin");
        assert_eq!(parsed["ip_address"], "127.0.0.1");
        assert_eq!(parsed["success"], true);
    }

    #[tokio::test]
    async fn test_null_logger_does_nothing() {
        let logger = NullAuditLogger;
        let event = AuditEvent::new(AuditEventType::UserLogin);
        logger.log_event(event).await;
    }

    #[tokio::test]
    async fn test_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.log");
        let path_str = path.to_str().unwrap().to_string();

        // Write initial content to make the file large enough
        let big_content = "x".repeat(1024 * 1024 + 1); // > 1 MB
        std::fs::write(&path, &big_content).unwrap();

        let config = AuditConfig {
            enabled: true,
            path: path_str.clone(),
            max_file_size_mb: 1, // 1 MB threshold
        };
        let logger = FileAuditLogger::new(config);

        let event = AuditEvent::new(AuditEventType::ConfigChange);
        logger.log_event(event).await;

        // Original file should now be rotated (renamed), new file created
        let rotated: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().map(|n| n.starts_with("audit.log")).unwrap_or(false))
            .collect();
        assert_eq!(rotated.len(), 2); // one rotated, one new
    }
}
