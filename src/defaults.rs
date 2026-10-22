use crate::Result;

/// Creates a default flows.json with an inject->debug flow that sends "Hello, World" every 5 seconds
pub fn create_default_flows_json() -> serde_json::Value {
    serde_json::json!([
    {
        "id": "adf5c374d9ac0466",
        "type": "tab",
        "label": "Flow 1"
    },
    {
        "id": "ded1a8c84fec2323",
        "type": "inject",
        "z": "adf5c374d9ac0466",
        "name": "Inject Hello",
        "props": [
            {
                "p": "payload"
            },
            {
                "p": "topic",
                "vt": "str"
            }
        ],
        "repeat": "5",
        "crontab": "",
        "once": false,
        "onceDelay": 0.1,
        "topic": "",
        "payload": "Hello, Rust-Red!",
        "payloadType": "date",
        "x": 410,
        "y": 280,
        "wires": [
            [
                "dc18e4d63818b44b"
            ]
        ]
    },
    {
        "id": "dc18e4d63818b44b",
        "type": "debug",
        "z": "adf5c374d9ac0466",
        "name": "debug 1",
        "active": true,
        "tosidebar": true,
        "console": true,
        "tostatus": false,
        "complete": "payload",
        "targetType": "msg",
        "statusVal": "",
        "statusType": "auto",
        "x": 670,
        "y": 280,
        "wires": []
    }
    ])
}

/// Creates a default rust-red.toml configuration file
pub fn create_default_config_file(config_dir: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    let config_path = Path::new(config_dir).join("rust-red.toml");

    // If config file already exists, nothing to do
    if config_path.exists() {
        return Ok(());
    }

    // Create directory if it doesn't exist
    if !Path::new(config_dir).exists() {
        fs::create_dir_all(config_dir)?;
        log::info!("Created config directory: {config_dir}");
    }

    // Create default config file content
    let default_config = r#"[runtime]

[runtime.engine]

[runtime.context]
default = "memory"

[runtime.context.stores]
memory = { provider = "memory" }

[runtime.flow]
node_msg_queue_capacity = 16
# Max messages buffered across incomplete message groups by sort (Node-RED nodeMessageBufferMaxLength); 0 = unlimited
node_message_buffer_max_length = 0

[ui-host]
host = "127.0.0.1"
port = 1888

# Security settings (secure by default)
# See docs/SECURITY.md for details
[security]
require_auth = true       # Require authentication (currently advisory)
max_flow_size = 10485760  # Max flow JSON payload size in bytes (10 MB)
rate_limit_rpm = 300      # Requests per minute per IP for admin API
cors_origins = []         # Empty = same-origin only; ["*"] = allow all (insecure)
security_headers = true   # Enable security response headers

# Audit logging settings
[audit]
enabled = true            # Enable audit logging
path = "./audit.log"      # Path to audit log file (JSONL format)
max_file_size_mb = 100    # Rotate log file when it exceeds this size

# OpenTelemetry observability (requires the `otel` feature flag at compile time)
[telemetry]
enabled = false           # Set to true to enable OTel export
endpoint = "http://localhost:4317"  # OTLP gRPC endpoint
service_name = "rust-red" # Logical service name in traces/metrics
trace_ratio = 1.0         # Sampling ratio (0.0–1.0)
"#;

    fs::write(&config_path, default_config)?;
    log::info!("Created default config file at: {}", config_path.display());

    Ok(())
}
