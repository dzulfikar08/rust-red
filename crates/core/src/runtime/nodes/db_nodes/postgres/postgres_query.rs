//! PostgreSQL query node.
//!
//! Registered as `postgres-query`, and also answers to `postgresql`, the query node of
//! `node-red-contrib-postgresql`. For a `postgresql` node the behaviour follows that package:
//!
//! - the `query` property is rendered with Mustache against `{ msg }` (`{{{msg.payload}}}`),
//!   unless `msg.query` is set, which is used as it is
//! - parameters come from `msg.params` (an array for `$1, $2, ...`) or from `msg.queryParameters`
//!   (an object for `$name` placeholders)
//! - on success `msg.payload` holds the rows and `msg.pgsql` holds `{ command, rowCount }`
//! - with `split` on, the rows are sent in chunks of `rowsPerMsg`, with `msg.parts`
//! - on failure the error goes to `catch` nodes and no message is sent on
//!
//! A `postgres-query` node keeps its own behaviour: the query is used as written, parameters can
//! also come from `msg.queryParams`, `msg.rowCount` is set, and a failed query sets `msg.error`
//! and passes the message on.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio_postgres::SimpleQueryMessage;
use tokio_postgres::types::ToSql;
use tokio_util::sync::CancellationToken;

use crate::runtime::flow::Flow;
use crate::runtime::model::json::deser::{lenient_bool, lenient_opt_u64};
use crate::runtime::model::*;
use crate::runtime::nodes::*;
use rust_red_macro::*;

use super::pg_codec::{PgParam, convert_named, rows_to_variant};
use super::postgres_config::PostgresConfigNode;
use super::template;

#[derive(Deserialize, Debug, Clone)]
struct PostgresQueryConfig {
    #[serde(default, alias = "configNode", alias = "postgreSQLConfig")]
    config_node: String,
    #[serde(default)]
    query: String,
    #[serde(default, alias = "timeoutMs", deserialize_with = "lenient_opt_u64")]
    timeout_ms: Option<u64>,
    /// Send the rows in chunks (`node-red-contrib-postgresql`).
    #[serde(default, deserialize_with = "lenient_bool")]
    split: bool,
    #[serde(default, rename = "rowsPerMsg", deserialize_with = "lenient_opt_u64")]
    rows_per_msg: Option<u64>,
}

const DEFAULT_TIMEOUT_MS: u64 = 30000;

#[derive(Debug)]
#[flow_node("postgres-query", red_name = "postgres-query", module = "rust-red")]
struct PostgresQueryNode {
    base: BaseFlowNodeState,
    config: PostgresQueryConfig,
    /// Built from a `postgresql` node of `node-red-contrib-postgresql`.
    contrib: bool,
}

/// What a statement returned.
struct QueryOutcome {
    rows: Vec<Variant>,
    row_count: u64,
    command: String,
    /// One entry per statement, when a query with several statements ran.
    statements: Option<Vec<Variant>>,
}

fn command_of(sql: &str) -> String {
    sql.trim_start().split(|c: char| c.is_whitespace() || c == '(' || c == ';').next().unwrap_or("").to_uppercase()
}

fn is_multi_statement_error(error: &tokio_postgres::Error) -> bool {
    error.as_db_error().is_some_and(|db| db.message().contains("multiple commands"))
}

async fn run_simple(client: &deadpool_postgres::Object, query: &str) -> crate::Result<QueryOutcome> {
    let mut rows = Vec::new();
    let mut statements = Vec::new();
    let mut total = 0;
    for message in client.simple_query(query).await? {
        match message {
            SimpleQueryMessage::Row(row) => {
                let mut object = std::collections::BTreeMap::new();
                for (idx, column) in row.columns().iter().enumerate() {
                    let value = row.get(idx).map_or(Variant::Null, |text| Variant::String(text.to_string()));
                    object.insert(column.name().to_string(), value);
                }
                rows.push(Variant::Object(object));
            }
            SimpleQueryMessage::CommandComplete(count) => {
                total += count;
                statements.push(Variant::from([("rowCount", Variant::from(count as i64))]));
            }
            _ => {}
        }
    }
    Ok(QueryOutcome { rows, row_count: total, command: command_of(query), statements: Some(statements) })
}

async fn run_query(client: &deadpool_postgres::Object, query: &str, params: Vec<Value>) -> crate::Result<QueryOutcome> {
    let statement = match client.prepare(query).await {
        Ok(statement) => statement,
        // Several statements in one query only work in the simple protocol, which takes no parameters.
        Err(e) if params.is_empty() && is_multi_statement_error(&e) => return run_simple(client, query).await,
        Err(e) => return Err(e.into()),
    };

    let bound: Vec<PgParam> = params.into_iter().map(PgParam).collect();
    let stream = client.query_raw(&statement, bound.iter().map(|p| p as &(dyn ToSql + Sync))).await?;
    tokio::pin!(stream);
    let mut raw_rows = Vec::new();
    while let Some(row) = stream.try_next().await? {
        raw_rows.push(row);
    }
    let row_count = stream.rows_affected().unwrap_or(raw_rows.len() as u64);
    let Variant::Array(rows) = rows_to_variant(&raw_rows) else { unreachable!("rows_to_variant returns an array") };
    Ok(QueryOutcome { rows, row_count, command: command_of(query), statements: None })
}

impl PostgresQueryNode {
    fn build(
        _flow: &Flow,
        base_node: BaseFlowNodeState,
        config: &RedFlowNodeConfig,
        _options: Option<&config::Config>,
    ) -> crate::Result<Box<dyn FlowNodeBehavior>> {
        let query_config = PostgresQueryConfig::deserialize(&config.rest)?;
        let contrib = config.type_name == "postgresql";
        Ok(Box::new(PostgresQueryNode { base: base_node, config: query_config, contrib }))
    }

    async fn resolve_config_node(&self) -> crate::Result<Arc<dyn GlobalNodeBehavior>> {
        let engine = self.flow().and_then(|f| f.engine()).ok_or_else(|| anyhow::anyhow!("No engine available"))?;
        engine
            .find_global_node_by_ref(&self.config.config_node)
            .ok_or_else(|| anyhow::anyhow!("Config node '{}' not found, or it failed to load", self.config.config_node))
    }

    /// Work out the SQL text and the parameter values for one message.
    fn prepare(&self, msg: &Msg) -> crate::Result<(String, Vec<Value>)> {
        let to_json = |variant: &Variant| serde_json::to_value(variant);

        let query_override = msg.get("query").and_then(|v| v.as_str()).filter(|q| !q.is_empty()).map(str::to_string);
        let query = match query_override {
            Some(query) => query,
            None if self.contrib => {
                let context = serde_json::json!({ "msg": serde_json::to_value(msg)? });
                template::render(&self.config.query, &context)?
            }
            None => self.config.query.clone(),
        };

        // `msg.params`, then the RustRED name `msg.queryParams`: positional values for `$1, $2, ...`.
        for key in ["params", "queryParams"] {
            if let Some(Variant::Array(items)) = msg.get(key)
                && !items.is_empty()
            {
                let values = items.iter().map(to_json).collect::<Result<Vec<_>, _>>()?;
                return Ok((query, values));
            }
        }

        // `msg.queryParameters`: named values for `$name` placeholders.
        if let Some(named @ Variant::Object(_)) = msg.get("queryParameters")
            && let Value::Object(map) = to_json(named)?
        {
            let (query, values) = convert_named(&query, &map)?;
            return Ok((query, values));
        }

        Ok((query, Vec::new()))
    }

    async fn execute(&self, config_node: &PostgresConfigNode, msg: &Msg) -> crate::Result<(String, QueryOutcome)> {
        let (query, params) = self.prepare(msg)?;
        let client = config_node.get_pool().await?;
        let timeout = Duration::from_millis(self.config.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
        let outcome = tokio::time::timeout(timeout, run_query(&client, &query, params))
            .await
            .map_err(|_| anyhow::anyhow!("Query timed out after {}ms", timeout.as_millis()))??;
        Ok((query, outcome))
    }

    fn pgsql_info(outcome: &QueryOutcome) -> Variant {
        let single = Variant::from([
            ("command", Variant::String(outcome.command.clone())),
            ("rowCount", Variant::from(outcome.row_count as i64)),
        ]);
        match &outcome.statements {
            Some(statements) => Variant::Array(statements.clone()),
            None => single,
        }
    }

    async fn status(&self, fill: StatusFill, shape: StatusShape, text: String, cancel: CancellationToken) {
        self.report_status(StatusObject { fill: Some(fill), shape: Some(shape), text: Some(text) }, cancel).await;
    }

    /// `postgresql` node: send the whole result, or the rows in chunks when `split` is on.
    async fn send_contrib(
        &self,
        msg: MsgHandle,
        outcome: QueryOutcome,
        cancel: CancellationToken,
    ) -> crate::Result<()> {
        let info = Self::pgsql_info(&outcome);

        if !self.config.split {
            {
                let mut guard = msg.write().await;
                guard.set("payload".to_string(), Variant::Array(outcome.rows));
                guard.set("pgsql".to_string(), info);
            }
            return self.fan_out_one(Envelope { port: 0, msg }, cancel).await;
        }

        let per_msg = self.config.rows_per_msg.unwrap_or(1).max(1) as usize;
        let parts_id = format!("{:016x}", rand::random::<u64>());
        let chunks: Vec<Vec<Variant>> = if outcome.rows.is_empty() {
            vec![Vec::new()]
        } else {
            outcome.rows.chunks(per_msg).map(<[Variant]>::to_vec).collect()
        };
        let count = chunks.len();
        for (index, chunk) in chunks.into_iter().enumerate() {
            let out = msg.deep_clone(true).await;
            {
                let mut guard = out.write().await;
                guard.remove("complete");
                match (per_msg, chunk.first()) {
                    (1, Some(row)) => guard.set("payload".to_string(), row.clone()),
                    (1, None) => {
                        guard.remove("payload");
                    }
                    _ => guard.set("payload".to_string(), Variant::Array(chunk)),
                }
                guard.set("pgsql".to_string(), info.clone());
                let mut parts = std::collections::BTreeMap::from([
                    ("id".to_string(), Variant::String(parts_id.clone())),
                    ("type".to_string(), Variant::String("array".to_string())),
                    ("index".to_string(), Variant::from(index as i64)),
                ]);
                if index + 1 == count {
                    parts.insert("count".to_string(), Variant::from(count as i64));
                    guard.set("complete".to_string(), Variant::Bool(true));
                }
                guard.set("parts".to_string(), Variant::Object(parts));
            }
            self.fan_out_one(Envelope { port: 0, msg: out }, cancel.clone()).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl FlowNodeBehavior for PostgresQueryNode {
    fn get_base(&self) -> &BaseFlowNodeState {
        &self.base
    }

    async fn run(self: Arc<Self>, stop_token: CancellationToken) {
        let config_node: Arc<dyn GlobalNodeBehavior> = match self.resolve_config_node().await {
            Ok(n) => n,
            Err(e) => {
                log::error!("[postgres-query:{}] {}", self.name(), e);
                self.status(StatusFill::Red, StatusShape::Ring, e.to_string(), stop_token.clone()).await;
                stop_token.cancelled().await;
                return;
            }
        };

        while !stop_token.is_cancelled() {
            let cancel = stop_token.child_token();
            let this = self.clone();
            let cfg = config_node.clone();
            with_uow(this.as_ref(), cancel.child_token(), |node, msg| {
                let cfg = cfg.clone();
                async move {
                    let Some(cfg_inner) = cfg.as_any().downcast_ref::<PostgresConfigNode>() else {
                        let error =
                            anyhow::anyhow!("Config node '{}' is not a PostgreSQL config", node.config.config_node);
                        log::error!("[postgres-query:{}] {error}", node.name());
                        return Err(error);
                    };

                    let result = {
                        let guard = msg.read().await;
                        node.execute(cfg_inner, &guard).await
                    };

                    match result {
                        Ok((_, outcome)) => {
                            node.status(
                                StatusFill::Green,
                                StatusShape::Dot,
                                format!("{} rows", outcome.row_count),
                                cancel.child_token(),
                            )
                            .await;
                            if node.contrib {
                                node.send_contrib(msg, outcome, CancellationToken::new()).await?;
                            } else {
                                {
                                    let mut guard = msg.write().await;
                                    guard.set("rowCount".to_string(), Variant::from(outcome.rows.len() as i64));
                                    guard.set("payload".to_string(), Variant::Array(outcome.rows));
                                }
                                node.fan_out_one(Envelope { port: 0, msg }, CancellationToken::new()).await?;
                            }
                            Ok(())
                        }
                        Err(e) => {
                            log::warn!("[postgres-query:{}] Query error: {e}", node.name());
                            node.status(StatusFill::Red, StatusShape::Ring, e.to_string(), cancel.child_token()).await;
                            if node.contrib {
                                // Like the contrib node: the error goes to `catch` nodes, nothing is sent on.
                                let query = msg.read().await.get("query").and_then(|q| q.as_str().map(str::to_string));
                                let text = match query {
                                    Some(query) => format!("{e} {query}"),
                                    None => e.to_string(),
                                };
                                return Err(anyhow::anyhow!(text));
                            }
                            msg.write().await.set("error".to_string(), Variant::String(e.to_string()));
                            node.fan_out_one(Envelope { port: 0, msg }, CancellationToken::new()).await?;
                            Ok(())
                        }
                    }
                }
            })
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_the_first_keyword() {
        assert_eq!(command_of("  insert into t values (1)"), "INSERT");
        assert_eq!(command_of("SELECT 1;"), "SELECT");
        assert_eq!(command_of("select(1)"), "SELECT");
    }

    #[test]
    fn contrib_config_properties_are_read() {
        let config: PostgresQueryConfig = serde_json::from_value(serde_json::json!({
            "name": "insert", "query": "INSERT INTO t VALUES ({{{msg.payload}}})", "postgreSQLConfig": "p1",
            "split": false, "rowsPerMsg": 1, "outputs": 1
        }))
        .unwrap();
        assert_eq!(config.config_node, "p1");
        assert!(!config.split);
        assert_eq!(config.rows_per_msg, Some(1));
    }

    #[test]
    fn native_config_properties_are_read() {
        let config: PostgresQueryConfig = serde_json::from_value(serde_json::json!({
            "configNode": "pg", "query": "SELECT 1", "timeout_ms": 500
        }))
        .unwrap();
        assert_eq!((config.config_node.as_str(), config.timeout_ms), ("pg", Some(500)));
    }
}
