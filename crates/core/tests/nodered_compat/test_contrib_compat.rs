//! Flows exported from stock Node-RED and from `node-red-contrib-modbus` and
//! `node-red-contrib-postgresql` must load and run as they are.
//!
//! The fixtures in `fixtures/` have the shape of a real Node-RED export: every property the
//! editor saves is there, including the ones we do not use, and numbers and booleans are the
//! strings the editor writes.

use std::time::Duration;

use serde_json::{Value, json};

use super::harness::TestHarness;
use rust_red_core::runtime::model::json::deser::parse_red_id_str;

const MQTT_STOCK: &str = include_str!("fixtures/node_red_mqtt_stock.json");
const MODBUS_POSTGRES: &str = include_str!("fixtures/node_red_modbus_postgres.json");

fn fixture(text: &str) -> Vec<Value> {
    serde_json::from_str(text).expect("fixture is valid JSON")
}

fn node_mut<'a>(flow: &'a mut [Value], id: &str) -> &'a mut Value {
    flow.iter_mut().find(|n| n["id"] == id).unwrap_or_else(|| panic!("fixture has no node '{id}'"))
}

fn to_json(value: Option<&rust_red_core::runtime::model::Variant>) -> Value {
    serde_json::to_value(value.expect("property is present")).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Stock mqtt-broker config node
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stock_mqtt_broker_export_loads_without_url() {
    let harness = TestHarness::from_flow_json_str(MQTT_STOCK);
    let engine = harness.engine();

    let broker = engine.find_global_node_by_ref("a4c91d6e7b2f8035").expect("mqtt-broker config node is loaded");
    let settings = broker.mqtt_settings().expect("it is an MQTT broker config");
    assert_eq!((settings.host.as_str(), settings.port), ("localhost", 1883));
    assert_eq!(settings.keepalive, 60);
    assert!(settings.clean && settings.auto_connect && !settings.tls);
    assert_eq!(settings.protocol_version, 4);
    assert!(settings.will_topic.is_none(), "an empty willTopic means no will message");

    for id in ["3e8d2a71f9c04b56", "c05e7a39d1b84f62"] {
        let node = engine.find_flow_node_by_id(&parse_red_id_str(id).unwrap()).expect("mqtt node is loaded");
        assert_eq!(node.type_str(), if id.starts_with("3e") { "mqtt in" } else { "mqtt out" });
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stock_mqtt_broker_with_tls_and_credentials() {
    let mut flow = fixture(MQTT_STOCK);
    let broker = node_mut(&mut flow, "a4c91d6e7b2f8035");
    broker["broker"] = json!("broker.example.com");
    broker["port"] = json!("8883");
    broker["usetls"] = json!(true);
    broker["clientid"] = json!("plc-gateway");
    broker["protocolVersion"] = json!("5");
    broker["keepalive"] = json!("15");
    broker["cleansession"] = json!(false);
    broker["credentials"] = json!({"user": "alice", "password": "s3cret"});

    let harness = TestHarness::from_flow_json(Value::Array(flow));
    let settings = harness.engine().find_global_node_by_ref("a4c91d6e7b2f8035").unwrap().mqtt_settings().unwrap();
    assert_eq!((settings.host.as_str(), settings.port, settings.tls), ("broker.example.com", 8883, true));
    assert_eq!(settings.client_id.as_deref(), Some("plc-gateway"));
    assert_eq!((settings.username.as_deref(), settings.password.as_deref()), (Some("alice"), Some("s3cret")));
    assert_eq!((settings.keepalive, settings.clean, settings.protocol_version), (15, false, 5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mqtt_broker_with_a_non_hex_id_is_found_by_mqtt_nodes() {
    let text = MQTT_STOCK.replace("a4c91d6e7b2f8035", "broker-local");
    let harness = TestHarness::from_flow_json_str(&text);
    let broker = harness.engine().find_global_node_by_ref("broker-local").expect("config node found by its id");
    assert_eq!(broker.type_str(), "mqtt-broker");
}

// ---------------------------------------------------------------------------
// A bad config node disables only itself
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bad_config_node_is_disabled_and_the_rest_runs() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "badbroker", "type": "mqtt-broker", "broker": "localhost", "port": "not-a-port"},
        {"id": "badpg", "type": "postgreSQLConfig", "host": "h", "port": "x", "portFieldType": "num"},
        {"id": "badplc", "type": "modbus-client", "clienttype": "udp"},
        {"id": "1", "z": "100", "type": "inject", "once": true, "onceDelay": 0,
         "props": [{"p": "payload", "v": "still works", "vt": "str"}],
         "payload": "still works", "payloadType": "str", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    for id in ["badbroker", "badpg", "badplc"] {
        assert!(harness.engine().find_global_node_by_ref(id).is_none(), "'{id}' is disabled");
    }

    let msgs = harness.run(1).await;
    assert_eq!(msgs.len(), 1, "the flow around the broken config nodes still runs");
}

// ---------------------------------------------------------------------------
// 4. Non-hex node ids
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn non_hex_config_node_ids_resolve() {
    let harness = TestHarness::from_flow_json_str(MODBUS_POSTGRES);
    let engine = harness.engine();

    let pg = engine.find_global_node_by_ref("p1").expect("'p1' resolves");
    assert_eq!(pg.type_str(), "postgres-config");
    let plc = engine.find_global_node_by_ref("plc1").expect("'plc1' resolves");
    assert_eq!(plc.type_str(), "modbus-config");
    assert!(engine.find_global_node_by_ref("nope").is_none());
}

// ---------------------------------------------------------------------------
// 2 and 3. Contrib node types are served by our nodes
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contrib_node_types_map_to_our_nodes() {
    let harness = TestHarness::from_flow_json_str(MODBUS_POSTGRES);
    let engine = harness.engine();
    let type_of = |id: &str| engine.find_flow_node_by_id(&parse_red_id_str(id).unwrap()).unwrap().type_str();

    assert_eq!(type_of("e19a4c70b5d32f88"), "modbus read", "modbus-read");
    assert_eq!(type_of("f4a81b6e3d905c27"), "postgres-query", "postgresql");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn all_contrib_modbus_node_types_load() {
    let mut flow = vec![json!({"id": "100", "type": "tab"})];
    flow.extend(fixture(MODBUS_POSTGRES).into_iter().filter(|n| n["id"] == "plc1"));
    let common = json!({"z": "100", "server": "plc1", "name": "", "wires": [[], []]});
    for (id, kind, extra) in [
        (
            "m1",
            "modbus-read",
            json!({"dataType": "Coil", "adr": "0", "quantity": "8", "rate": "500", "rateUnit": "ms"}),
        ),
        ("m2", "modbus-getter", json!({"dataType": "InputRegister", "adr": "5", "quantity": "2"})),
        ("m3", "modbus-write", json!({"dataType": "MHoldingRegisters", "adr": "0", "quantity": "2"})),
        ("m4", "modbus-flex-getter", json!({})),
        ("m5", "modbus-flex-write", json!({})),
    ] {
        let mut node = common.clone();
        node["id"] = json!(id);
        node["type"] = json!(kind);
        node.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        flow.push(node);
    }

    let harness = TestHarness::from_flow_json(Value::Array(flow));
    for (id, expected) in [
        ("m1", "modbus read"),
        ("m2", "modbus read"),
        ("m3", "modbus write"),
        ("m4", "modbus-flex-getter"),
        ("m5", "modbus-flex-writer"),
    ] {
        let node =
            harness.engine().find_flow_node_by_id(&parse_red_id_str(id).unwrap()).unwrap_or_else(|| panic!("{id}"));
        assert_eq!(node.type_str(), expected);
    }
}

/// A `modbus-client`, a `modbus-flex-write`, a `modbus-write` and two `modbus-getter` nodes
/// as exported by node-red-contrib-modbus, against the modbus-server node as a simulated PLC.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contrib_modbus_nodes_read_and_write_through_a_server() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();

    let inject = |id: &str, payload: &str, wires: Value| {
        json!({
            "id": id, "type": "inject", "z": "100", "name": "", "once": true, "onceDelay": 1,
            "props": [{"p": "payload", "v": payload, "vt": "json"}],
            "payload": payload, "payloadType": "json", "repeat": "", "crontab": "", "topic": "", "wires": wires
        })
    };
    let getter = |id: &str, adr: &str, quantity: &str, wires: Value| {
        json!({
            "id": id, "type": "modbus-getter", "z": "100", "name": "", "topic": "", "showStatusActivities": false,
            "logIOActivities": false, "showErrors": false, "showWarnings": true, "unitid": "", "dataType": "HoldingRegister",
            "adr": adr, "quantity": quantity, "server": "plc1", "useIOFile": false, "ioFile": "", "useIOForPayload": false,
            "emptyMsgOnFail": false, "keepMsgProperties": false, "delayOnStart": false, "enableDeformedMessages": false,
            "startDelayTime": "", "wires": wires
        })
    };

    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "srv", "type": "modbus-server", "z": "100", "host": "127.0.0.1", "port": port, "wires": []},
        {"id": "plc1", "type": "modbus-client", "name": "PLC", "clienttype": "tcp", "bufferCommands": true,
         "tcpHost": "127.0.0.1", "tcpPort": port.to_string(), "tcpType": "DEFAULT", "unit_id": 1,
         "commandDelay": "1", "clientTimeout": "2000", "reconnectOnTimeout": true, "reconnectTimeout": "500",
         "parallelUnitIdsAllowed": true},
        inject("i1", r#"{"fc":16,"unitid":1,"address":0,"quantity":3,"value":[11,22,33]}"#, json!([["w1"]])),
        {"id": "w1", "type": "modbus-flex-write", "z": "100", "name": "", "showStatusActivities": false,
         "showErrors": false, "showWarnings": true, "server": "plc1", "emptyMsgOnFail": false,
         "keepMsgProperties": false, "delayOnStart": false, "startDelayTime": "", "wires": [["g1"], []]},
        getter("g1", "0", "3", json!([["99"], ["98"]])),
        inject("i2", "[5,6]", json!([["w2"]])),
        {"id": "w2", "type": "modbus-write", "z": "100", "name": "", "showStatusActivities": false,
         "showErrors": false, "showWarnings": true, "unitid": "", "dataType": "MHoldingRegisters", "adr": "10",
         "quantity": "2", "server": "plc1", "emptyMsgOnFail": false, "keepMsgProperties": false,
         "delayOnStart": false, "startDelayTime": "", "wires": [["g2"], []]},
        getter("g2", "10", "2", json!([["97"], ["96"]])),
        {"id": "99", "z": "100", "type": "test-once"},
        {"id": "98", "z": "100", "type": "test-once"},
        {"id": "97", "z": "100", "type": "test-once"},
        {"id": "96", "z": "100", "type": "test-once"}
    ]);

    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.run_with_timeout(4, Duration::from_secs(20)).await;
    assert_eq!(msgs.len(), 4);

    // Output 1 of a getter: the values in payload, the raw response in responseBuffer.
    let first: Vec<_> = msgs.iter().filter(|m| m.get("responseBuffer").is_some()).collect();
    assert_eq!(first.len(), 2, "one output-1 message per getter");
    let payloads: Vec<Value> = first.iter().map(|m| to_json(m.get("payload"))).collect();
    assert!(payloads.contains(&json!([11, 22, 33])), "flex-write then getter: {payloads:?}");
    assert!(payloads.contains(&json!([5, 6])), "modbus-write then getter: {payloads:?}");

    let reading = first.iter().find(|m| to_json(m.get("payload")) == json!([11, 22, 33])).unwrap();
    let response = to_json(reading.get("responseBuffer"));
    assert_eq!(response["data"], json!([11, 22, 33]));
    assert_eq!(to_json(reading.get("modbusRequest"))["fc"], 3);

    // Output 2: the raw response in payload, the values next to it.
    let second: Vec<_> = msgs.iter().filter(|m| m.get("values").is_some()).collect();
    assert_eq!(second.len(), 2, "one output-2 message per getter");
    let raw = second.iter().find(|m| to_json(m.get("values")) == json!([11, 22, 33])).unwrap();
    assert_eq!(to_json(raw.get("payload"))["data"], json!([11, 22, 33]));
    assert!(raw.get("responseBuffer").is_none());
}

// ---------------------------------------------------------------------------
// 3. postgresql against a real server (needs Docker, see the module docs of test_postgres)
// ---------------------------------------------------------------------------

/// Connection of the throwaway server of `scripts/smoke`, or `RUST_RED_TEST_PG_*`.
fn pg_flow(query: &str, extra: Value) -> Value {
    let env = |key: &str, default: &str| std::env::var(key).unwrap_or_else(|_| default.to_string());
    let mut node = json!({
        "id": "q1", "type": "postgresql", "z": "100", "name": "", "query": query, "postgreSQLConfig": "p1",
        "split": false, "rowsPerMsg": 1, "outputs": 1, "wires": [["99"]]
    });
    node.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
    json!([
        {"id": "100", "type": "tab"},
        {"id": "p1", "type": "postgreSQLConfig", "name": "", "host": env("RUST_RED_TEST_PG_HOST", "127.0.0.1"),
         "hostFieldType": "str", "port": env("RUST_RED_TEST_PG_PORT", "5433"), "portFieldType": "num",
         "database": env("RUST_RED_TEST_PG_DB", "postgres"), "databaseFieldType": "str",
         "ssl": env("RUST_RED_TEST_PG_SSL", "false"), "sslFieldType": "json", "max": "4", "maxFieldType": "num",
         "credentials": {"user": env("RUST_RED_TEST_PG_USER", "postgres"), "password": env("RUST_RED_TEST_PG_PASSWORD", "rustred")}},
        node,
        {"id": "i1", "type": "inject", "z": "100", "once": true, "onceDelay": 0.2, "props": [], "wires": [["q1"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ])
}

async fn run_pg(query: &str, msg_props: Value) -> rust_red_core::runtime::model::Msg {
    // The inject node cannot set several typed properties from here, so a change node sets them.
    let mut flow = pg_flow(query, json!({}));
    let nodes = flow.as_array_mut().unwrap();
    let setter = json!({
        "id": "s1", "type": "function", "z": "100", "name": "", "outputs": 1,
        "func": format!("Object.assign(msg, {}); return msg;", msg_props),
        "wires": [["q1"]]
    });
    nodes.retain(|n| n["id"] != "i1");
    nodes.push(json!({"id": "i1", "type": "inject", "z": "100", "once": true, "onceDelay": 0.2, "props": [], "wires": [["s1"]]}));
    nodes.push(setter);
    let harness = TestHarness::from_flow_json(flow);
    harness.run_with_timeout(1, Duration::from_secs(20)).await.remove(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "Requires a PostgreSQL server, see scripts/smoke"]
async fn postgresql_contrib_node_round_trips_typed_params() {
    let setup = run_pg(
        "DROP TABLE IF EXISTS rr_contrib; CREATE TABLE rr_contrib (id serial primary key, a int4, b numeric(10,2), c text, d boolean, e timestamptz, f jsonb, g float8, h bigint);",
        json!({}),
    )
    .await;
    assert!(setup.get("error").is_none());

    // Positional parameters from msg.params, bound by column type.
    let insert = run_pg(
        "INSERT INTO rr_contrib (a, b, c, d, e, f, g, h) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id;",
        json!({"params": [42, 12.5, "hello", true, "2024-05-01T10:00:00Z", {"k": [1, 2]}, 3.25, 9007199254740994_i64]}),
    )
    .await;
    assert_eq!(to_json(insert.get("payload")), json!([{"id": 1}]));
    assert_eq!(to_json(insert.get("pgsql"))["command"], "INSERT");
    assert_eq!(to_json(insert.get("pgsql"))["rowCount"], 1);

    // Mustache in the query, an array payload printed like JavaScript does it.
    let mustache = run_pg(
        "INSERT INTO rr_contrib (a, b, c) VALUES ({{{msg.payload}}}, '{{msg.topic}}');",
        json!({"payload": [7, 0.5], "topic": "from mustache"}),
    )
    .await;
    assert_eq!(to_json(mustache.get("pgsql"))["rowCount"], 1);

    // Named parameters.
    let named = run_pg(
        "SELECT a FROM rr_contrib WHERE c = $name AND a = $num;",
        json!({"queryParameters": {"name": "hello", "num": 42}}),
    )
    .await;
    assert_eq!(to_json(named.get("payload")), json!([{"a": 42}]));

    // Types come back the way node-postgres hands them over.
    let select = run_pg("SELECT a, b, c, d, e, f, g, h FROM rr_contrib WHERE id = 1;", json!({})).await;
    let row = &to_json(select.get("payload"))[0];
    assert_eq!(row["a"], 42);
    assert_eq!(row["b"], "12.50", "numeric is text, like node-postgres");
    assert_eq!(row["c"], "hello");
    assert_eq!(row["d"], true);
    assert_eq!(row["e"], "2024-05-01T10:00:00.000Z");
    assert_eq!(row["f"], json!({"k": [1, 2]}));
    assert_eq!(row["g"], 3.25);
    assert_eq!(row["h"], "9007199254740994", "bigint beyond 2^53 is text");

    // Several statements in one query.
    let multi =
        run_pg("INSERT INTO rr_contrib (a) VALUES (1); INSERT INTO rr_contrib (a) VALUES (2);", json!({})).await;
    assert_eq!(to_json(multi.get("pgsql")).as_array().unwrap().len(), 2);

    run_pg("DROP TABLE rr_contrib;", json!({})).await;
}
