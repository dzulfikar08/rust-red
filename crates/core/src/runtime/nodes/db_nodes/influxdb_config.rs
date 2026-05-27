use async_trait::async_trait;
use serde::Deserialize;

use crate::runtime::engine::Engine;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct InfluxDbConfig {
    #[serde(default = "default_url")]
    url: String,
    token: String,
    org: String,
    bucket: String,
    #[serde(default = "default_version")]
    version: String,
}

fn default_url() -> String {
    "http://localhost:8086".to_string()
}
fn default_version() -> String {
    "v2".to_string()
}

#[derive(Debug)]
#[global_node("influxdb-config", red_name = "influxdb-config", module = "rust-red")]
pub(crate) struct InfluxDbConfigNode {
    base: BaseGlobalNodeState,
    config: InfluxDbConfig,
    client: reqwest::Client,
}

impl InfluxDbConfigNode {
    pub fn build(
        engine: &Engine,
        config: &RedGlobalNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn GlobalNodeBehavior>> {
        let influx_config = InfluxDbConfig::deserialize(&config.rest)?;

        let client = reqwest::Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                if let Ok(auth_val) = reqwest::header::HeaderValue::from_str(&format!("Token {}", influx_config.token))
                {
                    headers.insert(reqwest::header::AUTHORIZATION, auth_val);
                }
                headers
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client for InfluxDB: {}", e))?;

        let state = BaseGlobalNodeState {
            id: config.id,
            name: config.name.clone(),
            type_str: "influxdb-config",
            ordering: config.ordering,
            context: engine.get_context_manager().new_context(engine.context(), config.id.to_string()),
            disabled: config.disabled,
        };
        Ok(Box::new(InfluxDbConfigNode { base: state, config: influx_config, client }))
    }

    #[allow(dead_code)]
    pub fn url(&self) -> &str {
        &self.config.url
    }

    #[allow(dead_code)]
    pub fn org(&self) -> &str {
        &self.config.org
    }

    #[allow(dead_code)]
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// Write line protocol data to InfluxDB v2.
    ///
    /// `precision` should be one of "ns", "us", "ms", "s". Defaults to "ns" if empty.
    pub async fn write_line_protocol(&self, line_protocol: &str, precision: &str) -> crate::Result<()> {
        let prec = if precision.is_empty() { "ns" } else { precision };
        let url = format!(
            "{}/api/v2/write?org={}&bucket={}&precision={}",
            self.config.url.trim_end_matches('/'),
            urlencoding::encode(&self.config.org),
            urlencoding::encode(&self.config.bucket),
            prec,
        );

        let resp = self
            .client
            .post(&url)
            .body(line_protocol.to_string())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("InfluxDB write request failed: {}", e))?;

        let status = resp.status();
        if status.is_success() {
            log::debug!("[influxdb-config:{}] Write succeeded (status={})", self.name(), status);
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_else(|_| "<no body>".to_string());
            log::warn!("[influxdb-config:{}] Write failed (status={}): {}", self.name(), status, body);
            Err(anyhow::anyhow!("InfluxDB write failed (status={}): {}", status, body))
        }
    }

    /// Execute a Flux query against InfluxDB v2 and return the response body.
    ///
    /// Requests JSON output via `Accept: application/json`.
    pub async fn query_flux(&self, query: &str) -> crate::Result<String> {
        let url = format!(
            "{}/api/v2/query?org={}",
            self.config.url.trim_end_matches('/'),
            urlencoding::encode(&self.config.org),
        );

        let resp = self
            .client
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/vnd.flux")
            .body(query.to_string())
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("InfluxDB query request failed: {}", e))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_else(|_| "<no body>".to_string());

        if status.is_success() {
            log::debug!("[influxdb-config:{}] Query succeeded (status={})", self.name(), status);
            Ok(body)
        } else {
            log::warn!("[influxdb-config:{}] Query failed (status={}): {}", self.name(), status, body);
            Err(anyhow::anyhow!("InfluxDB query failed (status={}): {}", status, body))
        }
    }
}

#[async_trait]
impl GlobalNodeBehavior for InfluxDbConfigNode {
    fn get_base(&self) -> &BaseGlobalNodeState {
        &self.base
    }
}
