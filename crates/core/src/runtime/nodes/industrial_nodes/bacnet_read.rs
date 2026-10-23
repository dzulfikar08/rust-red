use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::bacnet_client;
use super::bacnet_config::BacnetConfigNode;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct BacnetReadConfig {
    #[serde(default, alias = "configNode")]
    config_node: String,
    #[serde(rename = "objectType")]
    #[serde(default = "default_object_type")]
    object_type: String,
    #[serde(rename = "objectInstance")]
    object_instance: u32,
    #[serde(default = "default_property")]
    property: String,
    /// More properties to read in the same message. When more than one property is requested in
    /// total, `msg.payload` is an object keyed by property name.
    #[serde(default)]
    properties: Vec<String>,
    /// Whether to subscribe to COV (Change of Value) notifications.
    /// When true, the node sends SubscribeCOV for the object, renews it before it runs out, and emits a
    /// message for every notification that carries one of the requested properties.
    #[serde(default, rename = "subscribeCov")]
    subscribe_cov: bool,
    /// COV subscription lifetime in seconds. 0 means the config node's `covLifetime`, and 300 when that is 0 too.
    #[serde(default, rename = "covLifetime")]
    cov_lifetime: u32,
    /// Optional COV increment threshold. A numeric value is only emitted when it differs from the last emitted
    /// one by at least this amount (the first notification is always emitted).
    #[serde(default, rename = "covIncrement")]
    cov_increment: Option<f64>,
}

fn default_object_type() -> String {
    "analogInput".to_string()
}
fn default_property() -> String {
    "presentValue".to_string()
}

#[derive(Debug)]
#[flow_node("bacnet read", red_name = "bacnet-read", module = "node-red")]
struct BacnetReadNode {
    base: BaseFlowNodeState,
    config: BacnetReadConfig,
}

impl BacnetReadNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let read_config = BacnetReadConfig::deserialize(&config.rest)?;
        Ok(Box::new(BacnetReadNode { base: base_node, config: read_config }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;

        let global = engine
            .find_global_node_by_ref(&self.config.config_node)
            .ok_or_else(|| anyhow::anyhow!("Config node not found"))?;

        Ok(global)
    }

    async fn cov_status(&self, fill: StatusFill, text: String, cancel: CancellationToken) {
        let shape = if matches!(fill, StatusFill::Green) { StatusShape::Dot } else { StatusShape::Ring };
        self.report_status(StatusObject { fill: Some(fill), shape: Some(shape), text: Some(text) }, cancel).await;
    }

    /// Hold a COV subscription for the node's object: subscribe, renew at half the lifetime, emit notifications.
    async fn run_cov(self: Arc<Self>, config_node: Arc<dyn GlobalNodeBehavior>, stop: CancellationToken) {
        use std::time::Duration;
        let Some(cfg_inner) = config_node.as_any().downcast_ref::<BacnetConfigNode>() else { return };
        let (target, timeout, retries, device_id, config_lifetime) = {
            let conn = cfg_inner.connection.lock().await;
            (conn.target_addr(), conn.apdu_timeout_ms(), conn.retries(), conn.device_id(), conn.cov_lifetime())
        };
        let timeout = Duration::from_millis(timeout as u64);
        let lifetime = if self.config.cov_lifetime > 0 {
            self.config.cov_lifetime
        } else if config_lifetime > 0 {
            config_lifetime
        } else {
            300
        };
        let setup = async {
            let object_type = bacnet_client::object_type_id(&self.config.object_type)
                .ok_or_else(|| format!("Unknown BACnet object type '{}'", self.config.object_type))?;
            let addr = tokio::net::lookup_host(target.as_str())
                .await
                .map_err(|e| format!("Cannot resolve BACnet target {target}: {e}"))?
                .next()
                .ok_or_else(|| format!("Cannot resolve BACnet target {target}"))?;
            let requested = self.requested_properties();
            let mut ids = Vec::new();
            for name in &requested {
                ids.push(bacnet_client::property_id(name).ok_or_else(|| format!("Unknown BACnet property '{name}'"))?);
            }
            Ok::<_, String>((object_type, addr, requested, ids))
        };
        let (object_type, addr, requested, ids) = match setup.await {
            Ok(v) => v,
            Err(e) => {
                log::error!("[bacnet-read:{}] {e}", self.name());
                self.cov_status(StatusFill::Red, e, stop.clone()).await;
                return;
            }
        };
        let process_id = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.id().to_string().hash(&mut h);
            (h.finish() as u32) & 0x00FF_FFFF | 1
        };
        let mut last_emitted: Option<f64> = None;
        'subscription: while !stop.is_cancelled() {
            let mut sub = match bacnet_client::CovSubscription::open(
                addr,
                process_id,
                object_type,
                self.config.object_instance,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    self.cov_status(StatusFill::Red, e, stop.clone()).await;
                    tokio::select! { _ = stop.cancelled() => return, _ = tokio::time::sleep(Duration::from_secs(5)) => continue };
                }
            };
            loop {
                if let Err(e) = sub.subscribe(lifetime, timeout, retries).await {
                    log::warn!("[bacnet-read:{}] SubscribeCOV failed: {e}", self.name());
                    self.cov_status(StatusFill::Red, e, stop.clone()).await;
                    tokio::select! { _ = stop.cancelled() => return, _ = tokio::time::sleep(Duration::from_secs(5)) => continue 'subscription };
                }
                self.cov_status(StatusFill::Green, format!("COV {}s", lifetime), stop.clone()).await;
                let renew_at = tokio::time::Instant::now() + Duration::from_secs((lifetime as u64 / 2).max(1));
                loop {
                    let next = tokio::select! {
                        _ = stop.cancelled() => {
                            sub.cancel(timeout).await;
                            return;
                        }
                        next = sub.next(renew_at) => next,
                    };
                    match next {
                        Ok(Some(event)) => {
                            if let Some(msg) =
                                self.cov_message(&event, &requested, &ids, &target, device_id, &mut last_emitted)
                            {
                                if let Err(e) = self
                                    .fan_out_one(Envelope { port: 0, msg: MsgHandle::new(msg) }, stop.child_token())
                                    .await
                                {
                                    log::warn!("[bacnet-read:{}] Cannot send the notification: {e}", self.name());
                                }
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            self.cov_status(StatusFill::Red, e, stop.clone()).await;
                            tokio::select! { _ = stop.cancelled() => return, _ = tokio::time::sleep(Duration::from_secs(5)) => continue 'subscription };
                        }
                    }
                }
            }
        }
    }

    /// The message for a COV notification, or `None` when it carries none of the requested properties or
    /// the change is below `covIncrement`.
    fn cov_message(
        &self,
        event: &bacnet_client::CovEvent,
        requested: &[String],
        ids: &[u32],
        target: &str,
        device_id: u32,
        last_emitted: &mut Option<f64>,
    ) -> Option<Msg> {
        let mut values = serde_json::Map::new();
        for name in requested.iter().zip(ids) {
            if let Some((_, v)) = event.values.iter().find(|(id, _)| id == name.1) {
                values.insert(name.0.clone(), v.clone());
            }
        }
        if values.is_empty() {
            return None;
        }
        if let (Some(inc), Some(now)) = (self.config.cov_increment, values.get(&requested[0]).and_then(|v| v.as_f64()))
        {
            if matches!(last_emitted, Some(last) if (now - *last).abs() < inc) {
                return None;
            }
            *last_emitted = Some(now);
        }
        let all: serde_json::Map<String, serde_json::Value> =
            event.values.iter().map(|(id, v)| (bacnet_client::property_name(*id), v.clone())).collect();
        let payload = if requested.len() == 1 {
            values.remove(&requested[0]).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Object(values)
        };
        let mut msg = Msg::default();
        msg.set("payload".to_string(), Variant::from(payload));
        msg.set(
            "bacnet".to_string(),
            Variant::from(serde_json::json!({
                "objectType": self.config.object_type,
                "objectInstance": self.config.object_instance,
                "property": self.config.property,
                "properties": requested,
                "endpoint": target,
                "deviceId": device_id,
                "cov": true,
                "values": all,
                "timeRemaining": event.time_remaining,
            })),
        );
        Some(msg)
    }

    /// Property names to read: `property` (may hold a comma separated list) plus `properties`, without duplicates.
    fn requested_properties(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for name in self.config.property.split(',').chain(self.config.properties.iter().map(String::as_str)) {
            let name = name.trim();
            if !name.is_empty() && !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        }
        if out.is_empty() {
            out.push(default_property());
        }
        out
    }
}

#[async_trait]
impl FlowNodeBehavior for BacnetReadNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[bacnet-read:{}] {}", self.name(), e);
                self.report_status(
                    StatusObject {
                        fill: Some(StatusFill::Red),
                        shape: Some(StatusShape::Ring),
                        text: Some(e.to_string()),
                    },
                    stop_token.clone(),
                )
                .await;
                stop_token.cancelled().await;
                return;
            }
        };

        if self.config.subscribe_cov {
            let this = self.clone();
            let cfg = config_node.clone();
            let stop = stop_token.clone();
            tokio::spawn(async move { this.run_cov(cfg, stop).await });
        }

        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            let this = self.clone();
            let cfg = config_node.clone();
            let send_cancel = cancel.child_token();
            with_uow(this.as_ref(), cancel.child_token(), |node, msg| {
                let cfg = cfg.clone();
                async move {
                    let cfg_inner = cfg.as_any().downcast_ref::<BacnetConfigNode>().unwrap();
                    let (target, timeout, retries, device_id) = {
                        let conn = cfg_inner.connection.lock().await;
                        (conn.target_addr(), conn.apdu_timeout_ms(), conn.retries(), conn.device_id())
                    };

                    let object_type = bacnet_client::object_type_id(&node.config.object_type)
                        .ok_or_else(|| anyhow::anyhow!("Unknown BACnet object type '{}'", node.config.object_type))?;
                    let requested = node.requested_properties();
                    let addr = tokio::net::lookup_host(target.as_str())
                        .await
                        .map_err(|e| anyhow::anyhow!("Cannot resolve BACnet target {target}: {e}"))?
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Cannot resolve BACnet target {target}"))?;

                    log::debug!(
                        "[bacnet-read:{}] Read {}:{} {:?} from {} (device_id={})",
                        node.name(),
                        node.config.object_type,
                        node.config.object_instance,
                        requested,
                        target,
                        device_id
                    );

                    let mut values = serde_json::Map::new();
                    for name in &requested {
                        let property = bacnet_client::property_id(name)
                            .ok_or_else(|| anyhow::anyhow!("Unknown BACnet property '{name}'"))?;
                        let value = bacnet_client::read_property(
                            addr,
                            object_type,
                            node.config.object_instance,
                            property,
                            std::time::Duration::from_millis(timeout as u64),
                            retries,
                        )
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("{e} ({}:{} {name})", node.config.object_type, node.config.object_instance)
                        })?;
                        values.insert(name.clone(), value);
                    }
                    let payload = if requested.len() == 1 {
                        values.remove(&requested[0]).unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Object(values)
                    };

                    let info = serde_json::json!({
                        "objectType": node.config.object_type,
                        "objectInstance": node.config.object_instance,
                        "property": node.config.property,
                        "properties": requested,
                        "endpoint": target,
                        "deviceId": device_id,
                    });

                    let mut guard = msg.write().await;
                    guard.set("payload".to_string(), Variant::from(payload));
                    guard.set("bacnet".to_string(), Variant::from(info));
                    drop(guard);
                    node.fan_out_one(Envelope { port: 0, msg: msg.clone() }, send_cancel).await?;
                    Ok(())
                }
            })
            .await;
        }
    }
}
