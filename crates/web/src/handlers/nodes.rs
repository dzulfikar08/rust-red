use crate::handlers::WebState;
use std::sync::Arc;
// use crate::handlers::utils::get_static_dir;
use axum::{
    Extension,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
};
use rust_red_core::runtime::paths;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct NodeInfo {
    name: String,
    module: String,
    version: String,
    local: bool,
    user: bool,
    types: Vec<String>,
}
type GroupedNodes = HashMap<String, NodeInfo>;

// nodes/plugins related handler
// ...existing code...

/// Get all nodes
pub async fn get_nodes(
    Extension(state): Extension<Arc<WebState>>,
    headers: HeaderMap,
) -> Result<axum::response::Response, StatusCode> {
    // Check Accept header to determine response format
    let accept_header = headers.get("accept").and_then(|h| h.to_str().ok()).unwrap_or("application/json");

    if accept_header.contains("text/html") {
        // Return HTML config for all nodes, including custom Rust nodes from registry
        let registry_guard = state.registry.read().await;
        let html_content = generate_nodes_html_with_registry(registry_guard.as_ref()).await;
        Ok(Html(html_content).into_response())
    } else {
        // Return node list in JSON format - based on actual node registry
        let registry_guard = state.registry.read().await;
        if let Some(registry) = registry_guard.as_ref() {
            // Use actual node registry
            let mut grouped_nodes: GroupedNodes = GroupedNodes::new();

            for (_, meta_node) in registry.all().iter() {
                let entry = grouped_nodes.entry(meta_node.red_id.to_string()).or_insert_with(|| NodeInfo {
                    name: meta_node.red_name.to_string(),
                    module: meta_node.module.to_string(),
                    version: meta_node.version.to_string(),
                    local: meta_node.local,
                    user: meta_node.user,
                    types: Vec::new(),
                });
                entry.types.push(meta_node.type_.to_string());
            }

            let flat_nodes: Vec<_> = grouped_nodes
                .into_iter()
                .map(|(red_id, node_info)| {
                    serde_json::json!({
                        "id": red_id,
                        "name": node_info.name,
                        "types": node_info.types,
                        "enabled": true,
                        "local": node_info.local,
                        "user": node_info.user,
                        "module": node_info.module,
                        "version": node_info.version
                    })
                })
                .collect();

            Ok(Json(serde_json::Value::Array(flat_nodes)).into_response())
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Generate HTML config for all nodes
pub async fn generate_nodes_html_with_registry(
    registry: Option<&rust_red_core::runtime::registry::RegistryHandle>,
) -> String {
    // Dynamically generate node HTML at runtime - read and merge all HTML files under Node-RED node directory
    let node_red_nodes_dir = paths::ui_static_dir().join("nodes");

    let mut html_content = String::new();

    if node_red_nodes_dir.exists() {
        // Handle core nodes
        let core_dir = node_red_nodes_dir.join("core");
        if core_dir.exists() {
            process_node_directory_runtime(&core_dir, &mut html_content).await;
        }

        // Handle example nodes (if any)
        let examples_dir = node_red_nodes_dir.join("examples");
        if examples_dir.exists() {
            process_node_directory_runtime(&examples_dir, &mut html_content).await;
        }
    }

    // Generate editor definitions for custom Rust nodes not covered by static HTML
    if let Some(reg) = registry {
        html_content.push_str(&generate_custom_nodes_html(reg));
    }

    if html_content.is_empty() {
        return get_fallback_nodes_html();
    }

    html_content
}

/// Custom node types that lack static HTML files and need auto-generated templates
const CUSTOM_NODE_TYPES: &[&str] = &[
    "modbus-config",
    "modbus read",
    "modbus write",
    "modbus-flex-getter",
    "modbus-flex-writer",
    "modbus-server",
    "opcua-config",
    "opcua read",
    "opcua write",
    "bacnet-config",
    "bacnet read",
    "bacnet write",
    "mqtt broker embedded",
];

fn is_custom_node(module: &str, type_: &str) -> bool {
    if module != "node-red" {
        return true;
    }
    CUSTOM_NODE_TYPES.contains(&type_)
}

/// Generate HTML templates and RED.nodes.registerType() calls for custom nodes from the Rust registry
fn generate_custom_nodes_html(registry: &rust_red_core::runtime::registry::RegistryHandle) -> String {
    use rust_red_core::runtime::nodes::NodeKind;

    let mut output = String::new();

    for (_, meta) in registry.all().iter() {
        if !is_custom_node(meta.module, meta.type_) {
            continue;
        }

        let node_type = meta.type_;
        let is_global = matches!(meta.kind, NodeKind::Global);

        // Generate HTML edit form template
        let template_html = get_node_template_html(node_type, is_global);
        output.push_str(&format!(
            "\n<script type=\"text/html\" data-template-name=\"{node_type}\">\n{template_html}</script>\n"
        ));

        // Generate help text
        let help_html = get_node_help_html(node_type);
        output.push_str(&format!(
            "\n<script type=\"text/x-red\" data-help-name=\"{node_type}\">\n{help_html}</script>\n"
        ));
    }

    // Generate JS registration block
    output.push_str("\n<script type=\"text/javascript\">\n(function() {\n");

    for (_, meta) in registry.all().iter() {
        if !is_custom_node(meta.module, meta.type_) {
            continue;
        }

        let node_type = meta.type_;
        let red_name = meta.red_name;
        let is_global = matches!(meta.kind, NodeKind::Global);
        let palette_label = get_palette_label(node_type);

        let (category, color, inputs, outputs, icon, defaults, align) = get_node_editor_config(node_type, is_global);
        let oneditprepare = get_oneditprepare(node_type, is_global);
        let oneditsave = get_oneditsave(node_type, is_global);

        output.push_str(&format!(
            "    RED.nodes.registerType('{node_type}', {{\n\
             \x20       category: '{category}',\n\
             \x20       color: '{color}',\n\
             \x20       defaults: {{\n{defaults}\
             \x20       }},\n\
             \x20       inputs: {inputs},\n\
             \x20       outputs: {outputs},\n\
             \x20       icon: \"{icon}\",\n\
             \x20       paletteLabel: \"{palette_label}\",\n\
             \x20       align: \"{align}\",\n\
             \x20       label: function() {{ return this.name || \"{red_name}\"; }},\n\
             \x20       oneditprepare: function() {{{oneditprepare}}},\n\
             \x20       oneditsave: function() {{{oneditsave}}}\n\
             \x20   }});\n\n",
        ));
    }

    output.push_str("})();\n</script>\n");
    output
}

/// Short label for the palette (displayed in the node palette sidebar)
fn get_palette_label(type_name: &str) -> String {
    match type_name {
        "modbus read" => "read".to_string(),
        "modbus write" => "write".to_string(),
        "modbus-flex-getter" => "flex getter".to_string(),
        "modbus-flex-writer" => "flex writer".to_string(),
        "modbus-config" => "Modbus Client".to_string(),
        "modbus-server" => "Modbus Server".to_string(),
        "mqtt broker embedded" => "MQTT Broker".to_string(),
        _ => type_name.to_string(),
    }
}

fn get_oneditprepare(node_type: &str, _is_global: bool) -> String {
    let mut js = String::new();

    // Helper closure to add tab click handler
    let add_tab_handler = |js: &mut String, prefix: &str| {
        js.push_str(&format!(
            r##"
                $( "#{prefix}-tabs li" ).on( "click", function() {{
                    $( "#{prefix}-tabs li" ).css( "border-bottom", "" );
                    $(this).css( "border-bottom", "2px solid #d9400d" );
                    $( ".{prefix}-tab-pane" ).hide();
                    $( "#"+$(this).data( "tab" ) ).show();
                }});
            "##
        ));
    };

    match node_type {
        "modbus-config" => {
            add_tab_handler(&mut js, "mc");
            js.push_str(r##"
                $( "#node-config-input-transport" ).on( "change", function() { if($(this).val()==="rtu") { $( "#mc-serial-section" ).show(); } else { $( "#mc-serial-section" ).hide(); } });
                $( "#node-config-input-transport" ).trigger( "change" );
            "##);
        }
        "modbus read" => {
            add_tab_handler(&mut js, "mr");
            js.push_str(
                r##"
                if(this.pollRateUnit) { $( "#node-input-pollRateUnit" ).val(this.pollRateUnit); }
            "##,
            );
        }
        "modbus write" => {
            add_tab_handler(&mut js, "mw");
        }
        "modbus-server" => {
            add_tab_handler(&mut js, "ms");
        }
        "mqtt broker embedded" => {
            add_tab_handler(&mut js, "mbe");
            js.push_str(r##"
                $( "#node-input-wsEnabled" ).on( "change", function() { if($(this).is( ":checked" )) { $( "#mbe-ws-section" ).show(); } else { $( "#mbe-ws-section" ).hide(); } });
                $( "#node-input-wsEnabled" ).trigger( "change" );
            "##);
        }
        "postgres-config" | "timescaledb-config" => {
            add_tab_handler(&mut js, "pc");
            js.push_str("                $( \"#node-config-input-host\" ).typedInput({types:[\"str\",\"env\"],typeField:\"#node-config-input-hostType\"});\n");
            js.push_str("                $( \"#node-config-input-port\" ).typedInput({types:[\"num\",\"env\"],typeField:\"#node-config-input-portType\"});\n");
            js.push_str("                $( \"#node-config-input-dbname\" ).typedInput({types:[\"str\",\"env\"],typeField:\"#node-config-input-dbnameType\"});\n");
        }
        "opcua-config" => {
            add_tab_handler(&mut js, "oc");
            js.push_str(r##"
                $( "#node-config-input-authMethod" ).on( "change", function() { var v=$(this).val(); $( "#oc-credentials-section" ).toggle(v==="credentials"); $( "#oc-certificate-section" ).toggle(v==="certificate"); });
                $( "#node-config-input-authMethod" ).trigger( "change" );
            "##);
        }
        "opcua read" => {
            add_tab_handler(&mut js, "or");
            js.push_str(r##"
                var action=this.action||"read"; $( "#node-input-action" ).val(action);
                function toggleOpcuaFields() { var a=$( "#node-input-action" ).val(); $( "#or-interval-section" ).toggle(a==="subscribe"||a==="monitor"); $( "#or-deadband-section" ).toggle(a==="monitor"); }
                $( "#node-input-action" ).on( "change", toggleOpcuaFields); toggleOpcuaFields();
            "##);
        }
        _ => {}
    }

    js
}

fn get_oneditsave(node_type: &str, _is_global: bool) -> String {
    match node_type {
        "modbus read" => r##" this.pollRateUnit = $( "#node-input-pollRateUnit" ).val(); "##.to_string(),
        _ => String::new(),
    }
}

/// Generate help text for a node type (shown in the info panel)
fn get_node_help_html(type_name: &str) -> String {
    match type_name {
        "modbus-config" => r#"<p>Modbus client connection configuration. Defines how to connect to a Modbus TCP/UDP device.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Host <span class="property-type">string</span></dt><dd>IP address or hostname of the Modbus device</dd>
    <dt>Port <span class="property-type">number</span></dt><dd>TCP port (default: 502)</dd>
    <dt>Transport <span class="property-type">string</span></dt><dd>Connection type: TCP, UDP, or Serial RTU</dd>
    <dt>Unit ID <span class="property-type">number</span></dt><dd>Modbus slave unit ID (default: 1)</dd>
    <dt>Timeout <span class="property-type">number</span></dt><dd>Connection timeout in ms (default: 5000)</dd>
</dl>
<h3>Details</h3>
<p>This is a configuration node shared by all Modbus flow nodes. It manages the connection lifecycle and auto-reconnects on failure.</p>"#.to_string(),

        "modbus-server" => r#"<p>In-process Modbus TCP server/simulator for testing and development.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Host <span class="property-type">string</span></dt><dd>Bind address (default: 127.0.0.1)</dd>
    <dt>Port <span class="property-type">number</span></dt><dd>Listen port (default: 5020)</dd>
    <dt>Coil Count <span class="property-type">number</span></dt><dd>Number of simulated coils (default: 100)</dd>
    <dt>Register Count <span class="property-type">number</span></dt><dd>Number of simulated holding registers (default: 100)</dd>
</dl>
<h3>Inputs</h3>
<dl class="message-properties">
    <dt>payload <span class="property-type">object</span></dt><dd>Write to simulator state: <code>{address: 0, value: true}</code> writes a coil, <code>{address: 0, value: 42}</code> writes a register</dd>
</dl>
<h3>Outputs</h3>
<dl class="message-properties">
    <dt>topic: modbus/connect</dt><dd>Emitted when a client connects. Payload contains remote address.</dd>
    <dt>topic: modbus/request</dt><dd>Emitted for each Modbus request. Payload contains functionCode, address, quantity/value.</dd>
    <dt>topic: modbus/disconnect</dt><dd>Emitted when a client disconnects.</dd>
</dl>
<h3>Details</h3>
<p>Supports FC1-6, FC15-16 (read/write coils and registers). Connect a debug node to the output to monitor all server activity. Use with <code>modbus read</code>/<code>modbus write</code> nodes for end-to-end testing without physical hardware.</p>"#.to_string(),

        "mqtt broker embedded" => r#"<p>Embedded MQTT 3.1.1 broker. Runs a full MQTT server as part of your flow.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Host <span class="property-type">string</span></dt><dd>Bind address (default: 127.0.0.1)</dd>
    <dt>Port <span class="property-type">number</span></dt><dd>Listen port (default: 1883)</dd>
    <dt>Max Connections <span class="property-type">number</span></dt><dd>Maximum concurrent client connections (default: 100)</dd>
</dl>
<h3>Outputs</h3>
<dl class="message-properties">
    <dt>topic: broker/start</dt><dd>Emitted when broker starts. Payload contains host and port.</dd>
    <dt>topic: broker/metrics</dt><dd>Emitted every 30 seconds with connection and message counters.</dd>
</dl>
<h3>Details</h3>
<p>Supports QoS 0/1/2, retained messages, wildcards (+/#), and will messages. Connect <code>mqtt in</code> and <code>mqtt out</code> nodes to this broker by setting their broker URL to <code>localhost:&lt;port&gt;</code>. Ideal for local testing, edge scenarios, and decoupled microservice communication without an external broker.</p>"#.to_string(),

        "modbus read" => r#"<p>Reads data from a Modbus device. Supports polling and trigger-based reads.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Server <span class="property-type">modbus-config</span></dt><dd>Modbus client connection</dd>
    <dt>Function Code <span class="property-type">string</span></dt><dd>Modbus read function</dd>
    <dt>Address <span class="property-type">number</span></dt><dd>Starting register/coil address (0-based)</dd>
    <dt>Quantity <span class="property-type">number</span></dt><dd>Number of values to read</dd>
    <dt>Data Type <span class="property-type">string</span></dt><dd>Register interpretation: uint16, int16, uint32, int32, float, double, uint64, int64</dd>
    <dt>Poll Interval <span class="property-type">number</span></dt><dd>Polling interval in ms (0 = trigger-only mode)</dd>
</dl>
<h3>Function Codes</h3>
<table>
    <tr><th>Code</th><th>Name</th></tr>
    <tr><td>FC1</td><td>Read Coils</td></tr>
    <tr><td>FC2</td><td>Read Discrete Inputs</td></tr>
    <tr><td>FC3</td><td>Read Holding Registers</td></tr>
    <tr><td>FC4</td><td>Read Input Registers</td></tr>
</table>
<h3>Output</h3>
<p>Sets <code>msg.payload</code> to the read values (array or single value depending on quantity). Sets <code>msg.modbus</code> with functionCode, address, and quantity metadata.</p>"#.to_string(),

        "modbus write" => r#"<p>Writes data to a Modbus device.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Server <span class="property-type">modbus-config</span></dt><dd>Modbus client connection</dd>
    <dt>Function Code <span class="property-type">string</span></dt><dd>Modbus write function</dd>
    <dt>Address <span class="property-type">number</span></dt><dd>Target register/coil address (0-based)</dd>
    <dt>Data Type <span class="property-type">string</span></dt><dd>Register encoding: uint16, int16, uint32, int32, float, double, uint64, int64</dd>
</dl>
<h3>Function Codes</h3>
<table>
    <tr><th>Code</th><th>Name</th><th>Payload</th></tr>
    <tr><td>FC5</td><td>Write Single Coil</td><td>boolean</td></tr>
    <tr><td>FC6</td><td>Write Single Register</td><td>number</td></tr>
    <tr><td>FC15</td><td>Write Multiple Coils</td><td>boolean array</td></tr>
    <tr><td>FC16</td><td>Write Multiple Registers</td><td>number or array</td></tr>
</table>
<h3>Input</h3>
<p><code>msg.payload</code> — value(s) to write. For coils use boolean. For registers use number(s) converted via Data Type.</p>
<h3>Output</h3>
<p>Sets <code>msg.payload</code> to the written value or count. Sets <code>msg.modbus</code> with functionCode and address.</p>"#.to_string(),

        "modbus-flex-getter" => r#"<p>Dynamic Modbus reader — address, function code, and quantity are set at runtime via the incoming message.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Server <span class="property-type">modbus-config</span></dt><dd>Modbus client connection</dd>
    <dt>Data Type <span class="property-type">string</span></dt><dd>Register interpretation for FC3/FC4 results</dd>
</dl>
<h3>Input Message</h3>
<dl class="message-properties">
    <dt>msg.address <span class="property-type">number</span></dt><dd>Starting register/coil address (default: 0)</dd>
    <dt>msg.quantity <span class="property-type">number</span></dt><dd>Number of values to read (default: 1)</dd>
    <dt>msg.functionCode <span class="property-type">string</span></dt><dd>readCoils, readDiscreteInputs, readHoldingRegisters, readInputRegisters (default: readHoldingRegisters)</dd>
</dl>
<h3>Output</h3>
<p>Sets <code>msg.payload</code> to the read values. Sets <code>msg.modbus</code> with functionCode, address, and quantity.</p>"#.to_string(),

        "modbus-flex-writer" => r#"<p>Dynamic Modbus writer — address, function code, and payload are set at runtime via the incoming message.</p>
<h3>Properties</h3>
<dl class="message-properties">
    <dt>Server <span class="property-type">modbus-config</span></dt><dd>Modbus client connection</dd>
    <dt>Data Type <span class="property-type">string</span></dt><dd>Register encoding for write operations</dd>
</dl>
<h3>Input Message</h3>
<dl class="message-properties">
    <dt>msg.address <span class="property-type">number</span></dt><dd>Target register/coil address (default: 0)</dd>
    <dt>msg.functionCode <span class="property-type">string</span></dt><dd>writeSingleCoil, writeSingleRegister, writeMultipleCoils, writeMultipleRegisters (default: writeSingleRegister)</dd>
    <dt>msg.payload <span class="property-type">any</span></dt><dd>Value(s) to write</dd>
</dl>
<h3>Output</h3>
<p>Sets <code>msg.payload</code> to the written value or count. Sets <code>msg.modbus</code> with functionCode and address.</p>"#.to_string(),

        _ => format!("<p>{type_name} node</p>"),
    }
}

/// Generate the <script type="text/html" data-template-name> form HTML for a node type
fn get_node_template_html(type_name: &str, is_global: bool) -> String {
    if is_global { get_global_node_template_html(type_name) } else { get_flow_node_template_html(type_name) }
}

fn form_row(icon: &str, label: &str, input_id: &str, placeholder: &str) -> String {
    format!(
        "    <div class=\"form-row\">\n\
         \x20       <label for=\"{input_id}\"><i class=\"fa fa-{icon}\"></i> {label}</label>\n\
         \x20       <input type=\"text\" id=\"{input_id}\" placeholder=\"{placeholder}\">\n\
         \x20   </div>\n"
    )
}

fn form_row_password(icon: &str, label: &str, input_id: &str) -> String {
    format!(
        "    <div class=\"form-row\">\n\
         \x20       <label for=\"{input_id}\"><i class=\"fa fa-{icon}\"></i> {label}</label>\n\
         \x20       <input type=\"password\" id=\"{input_id}\" placeholder=\"\">\n\
         \x20   </div>\n"
    )
}

fn form_row_number(icon: &str, label: &str, input_id: &str, placeholder: &str) -> String {
    format!(
        "    <div class=\"form-row\">\n\
         \x20       <label for=\"{input_id}\"><i class=\"fa fa-{icon}\"></i> {label}</label>\n\
         \x20       <input type=\"number\" id=\"{input_id}\" placeholder=\"{placeholder}\" style=\"width:100px;\">\n\
         \x20   </div>\n"
    )
}

fn form_row_textarea(icon: &str, label: &str, input_id: &str, placeholder: &str) -> String {
    format!(
        "    <div class=\"form-row\">\n\
         \x20       <label for=\"{input_id}\"><i class=\"fa fa-{icon}\"></i> {label}</label>\n\
         \x20       <textarea id=\"{input_id}\" rows=\"6\" placeholder=\"{placeholder}\" style=\"width:100%;\"></textarea>\n\
         \x20   </div>\n"
    )
}

fn form_row_select(icon: &str, label: &str, input_id: &str, options: &[(&str, &str)]) -> String {
    let opts: String = options.iter().map(|(val, text)| format!("<option value=\"{val}\">{text}</option>")).collect();
    format!(
        "    <div class=\"form-row\">\n\
         \x20       <label for=\"{input_id}\"><i class=\"fa fa-{icon}\"></i> {label}</label>\n\
         \x20       <select id=\"{input_id}\" style=\"width:70%\">{opts}</select>\n\
         \x20   </div>\n"
    )
}

fn form_row_config_node(config_type: &str, label: &str) -> String {
    // Must match the defaults key: camelCase for modbus/bacnet/opcua, snake_case for DB nodes
    let key = match config_type {
        "modbus read" | "modbus write" | "modbus-flex-getter" | "modbus-flex-writer" | "bacnet read"
        | "bacnet write" | "opcua read" | "opcua write" => "configNode",
        _ => "config_node",
    };
    format!(
        "    <div class=\"form-row\">\n\
         \x20       <label for=\"node-input-{key}\"><i class=\"fa fa-server\"></i> {label}</label>\n\
         \x20       <input type=\"text\" id=\"node-input-{key}\" style=\"width:60%\">\n\
         \x20   </div>\n"
    )
}

fn name_row() -> String {
    form_row("tag", "Name", "node-input-name", "")
}

// Config node (global node) variants — Node-RED uses node-config-input- prefix
fn cfg_name_row() -> String {
    form_row("tag", "Name", "node-config-input-name", "")
}

fn cfg_form_row(icon: &str, label: &str, key: &str, placeholder: &str) -> String {
    form_row(icon, label, &format!("node-config-input-{key}"), placeholder)
}

fn cfg_form_row_number(icon: &str, label: &str, key: &str, placeholder: &str) -> String {
    form_row_number(icon, label, &format!("node-config-input-{key}"), placeholder)
}

fn cfg_form_row_password(icon: &str, label: &str, key: &str) -> String {
    form_row_password(icon, label, &format!("node-config-input-{key}"))
}

fn cfg_form_row_select(icon: &str, label: &str, key: &str, options: &[(&str, &str)]) -> String {
    form_row_select(icon, label, &format!("node-config-input-{key}"), options)
}

// --- Tabbed UI helpers ---

fn tab_bar(prefix: &str, tabs: &[(&str, &str)]) -> String {
    let mut s = String::new();
    s.push_str("<div style=\"margin-bottom:10px\">");
    s.push_str(&format!("<ul id=\"{prefix}-tabs\" style=\"list-style:none;margin:0;padding:0;border-bottom:1px solid #ccc;display:flex;gap:0\">"));
    for (i, (id, label)) in tabs.iter().enumerate() {
        let border_bottom = if i == 0 { "border-bottom:2px solid #d9400d;" } else { "" };
        s.push_str(&format!(
            "<li data-tab=\"{prefix}-{id}\" class=\"{prefix}-tab-item\" style=\"padding:6px 14px;cursor:pointer;{border_bottom}\"><a href=\"#\" onclick=\"return false\" style=\"text-decoration:none;color:inherit;font-size:12px\">{label}</a></li>"
        ));
    }
    s.push_str("</ul>");
    s.push_str(&format!("<div id=\"{prefix}-tab-content\" style=\"min-height:120px;padding-top:8px\">"));
    s
}

fn tab_content_start(prefix: &str, id: &str, active: bool) -> String {
    let style = if active { "" } else { "display:none;" };
    format!("<div id=\"{prefix}-{id}\" class=\"{prefix}-tab-pane\" style=\"{style}\">")
}

fn tab_content_end() -> &'static str {
    "</div>"
}

fn tab_bar_close() -> &'static str {
    "</div></div>"
}

fn form_row_checkbox(icon: &str, label: &str, input_id: &str) -> String {
    format!(
        "<div class=\"form-row\"><label>&nbsp;</label><input type=\"checkbox\" id=\"{input_id}\" style=\"display:inline-block;width:auto;vertical-align:middle\"> <i class=\"fa fa-{icon}\"></i> <span>{label}</span></div>"
    )
}

fn cfg_form_row_checkbox(icon: &str, label: &str, key: &str) -> String {
    form_row_checkbox(icon, label, &format!("node-config-input-{key}"))
}

fn poll_rate_row(input_id: &str, unit_id: &str) -> String {
    format!(
        "<div class=\"form-row\"><label><i class=\"fa fa-clock\"></i> Poll Rate</label>\
         <input type=\"number\" id=\"{input_id}\" style=\"width:80px;display:inline-block\" min=\"0\">\
         <select id=\"{unit_id}\" style=\"width:70px;display:inline-block;margin-left:4px\">\
         <option value=\"ms\">ms</option><option value=\"s\">s</option>\
         <option value=\"m\">min</option><option value=\"h\">hr</option></select></div>"
    )
}

fn section_divider(title: &str) -> String {
    format!(
        "<hr style=\"border:0;border-top:1px solid #ccc;margin:12px 0 8px\"><b style=\"font-size:11px\">{title}</b>"
    )
}

fn conditional_section_start(id: &str) -> String {
    format!("<div id=\"{id}\" style=\"display:none\">")
}

fn conditional_section_end() -> &'static str {
    "</div>"
}

fn typed_input_row(icon: &str, label: &str, input_id: &str, type_field_id: &str) -> String {
    format!(
        "<div class=\"form-row\"><label for=\"{input_id}\"><i class=\"fa fa-{icon}\"></i> {label}</label>\
         <input type=\"text\" id=\"{input_id}\">\
         <input type=\"hidden\" id=\"{type_field_id}\"></div>"
    )
}

fn get_flow_node_template_html(type_name: &str) -> String {
    let mut html = String::new();

    match type_name {
        "postgres-query" | "sqlite-query" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&section_divider("Query"));
            html.push_str(&form_row_textarea("file-code-o", "Query", "node-input-query", "SELECT * FROM table"));
            html.push_str(&section_divider("Output"));
            html.push_str(&form_row_number("clock-o", "Timeout (ms)", "node-input-timeout_ms", "30000"));
            html.push_str(&form_row("cog", "Output Mode", "node-input-output_mode", "rows"));
        }
        "timescaledb-query" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_textarea("file-code-o", "Query", "node-input-query", "SELECT * FROM table"));
            html.push_str(&form_row_number("clock-o", "Timeout (ms)", "node-input-timeout_ms", "30000"));
        }
        "mssql-query" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_textarea("file-code-o", "Query", "node-input-query", "SELECT * FROM table"));
            html.push_str(&form_row_number("clock-o", "Timeout (ms)", "node-input-timeout_ms", "30000"));
        }
        "influxdb-in" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row("edit", "Measurement", "node-input-measurement", "measurement"));
            html.push_str(&form_row("tags", "Tags (JSON)", "node-input-tag_columns", "{\"host\":\"server1\"}"));
            html.push_str(&form_row("list", "Fields (JSON)", "node-input-field_columns", "{\"value\":42}"));
            html.push_str(&form_row("clock-o", "Timestamp column", "node-input-timestamp_column", "time"));
        }
        "influxdb-out" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_textarea("file-code-o", "Query", "node-input-query", "from(bucket: \"...\")"));
            html.push_str(&form_row_number("clock-o", "Timeout (ms)", "node-input-timeout_ms", "30000"));
        }
        "modbus read" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&tab_bar("mr", &[("settings", "Settings"), ("options", "Options")]));
            html.push_str(&tab_content_start("mr", "settings", true));
            html.push_str(&form_row_select(
                "cog",
                "Function Code",
                "node-input-functionCode",
                &[
                    ("readCoils", "Read Coils (FC1)"),
                    ("readDiscreteInputs", "Read Discrete Inputs (FC2)"),
                    ("readHoldingRegisters", "Read Holding Registers (FC3)"),
                    ("readInputRegisters", "Read Input Registers (FC4)"),
                ],
            ));
            html.push_str(&form_row_number("map-marker", "Address", "node-input-address", "0"));
            html.push_str(&form_row_number("bars", "Quantity", "node-input-quantity", "1"));
            html.push_str(&poll_rate_row("node-input-pollRate", "node-input-pollRateUnit"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("mr", "options", false));
            html.push_str(&form_row_select(
                "cog",
                "Data Type",
                "node-input-dataType",
                &[
                    ("uint16", "UInt16"),
                    ("int16", "Int16"),
                    ("uint32", "UInt32"),
                    ("int32", "Int32"),
                    ("float", "Float"),
                    ("double", "Double"),
                    ("uint64", "UInt64"),
                    ("int64", "Int64"),
                ],
            ));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "modbus write" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&tab_bar("mw", &[("settings", "Settings"), ("options", "Options")]));
            html.push_str(&tab_content_start("mw", "settings", true));
            html.push_str(&form_row_select(
                "cog",
                "Function Code",
                "node-input-functionCode",
                &[
                    ("writeSingleCoil", "Write Single Coil (FC5)"),
                    ("writeSingleRegister", "Write Single Register (FC6)"),
                    ("writeMultipleCoils", "Write Multiple Coils (FC15)"),
                    ("writeMultipleRegisters", "Write Multiple Registers (FC16)"),
                ],
            ));
            html.push_str(&form_row_number("map-marker", "Address", "node-input-address", "0"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("mw", "options", false));
            html.push_str(&form_row_select(
                "cog",
                "Data Type",
                "node-input-dataType",
                &[
                    ("uint16", "UInt16"),
                    ("int16", "Int16"),
                    ("uint32", "UInt32"),
                    ("int32", "Int32"),
                    ("float", "Float"),
                    ("double", "Double"),
                    ("uint64", "UInt64"),
                    ("int64", "Int64"),
                ],
            ));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "modbus-flex-getter" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_select(
                "cog",
                "Data Type",
                "node-input-dataType",
                &[
                    ("uint16", "UInt16"),
                    ("int16", "Int16"),
                    ("uint32", "UInt32"),
                    ("int32", "Int32"),
                    ("float", "Float"),
                    ("double", "Double"),
                    ("uint64", "UInt64"),
                    ("int64", "Int64"),
                ],
            ));
        }
        "modbus-flex-writer" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_select(
                "cog",
                "Data Type",
                "node-input-dataType",
                &[
                    ("uint16", "UInt16"),
                    ("int16", "Int16"),
                    ("uint32", "UInt32"),
                    ("int32", "Int32"),
                    ("float", "Float"),
                    ("double", "Double"),
                    ("uint64", "UInt64"),
                    ("int64", "Int64"),
                ],
            ));
        }
        "modbus-server" => {
            html.push_str(&name_row());
            html.push_str(&tab_bar("ms", &[("settings", "Settings"), ("options", "Options")]));
            html.push_str(&tab_content_start("ms", "settings", true));
            html.push_str(&form_row("server", "Host", "node-input-host", "127.0.0.1"));
            html.push_str(&form_row_number("cog", "Port", "node-input-port", "5020"));
            html.push_str(&form_row_number("bars", "Coil Count", "node-input-coilCount", "100"));
            html.push_str(&form_row_number("bars", "Register Count", "node-input-registerCount", "100"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("ms", "options", false));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "mqtt broker embedded" => {
            html.push_str(&name_row());
            html.push_str(&tab_bar(
                "mbe",
                &[("connection", "Connection"), ("persistence", "Persistence"), ("security", "Security")],
            ));
            html.push_str(&tab_content_start("mbe", "connection", true));
            html.push_str(&form_row("server", "Host", "node-input-host", "127.0.0.1"));
            html.push_str(&form_row_number("cog", "Port", "node-input-port", "1883"));
            html.push_str(&form_row_number("users", "Max Connections", "node-input-max_connections", "100"));
            html.push_str(&form_row_checkbox("fa-link", "Enable WebSocket", "node-input-wsEnabled"));
            html.push_str(&conditional_section_start("mbe-ws-section"));
            html.push_str(&form_row("sign-in", "WebSocket Path", "node-input-wsPath", "/mqtt"));
            html.push_str(conditional_section_end());
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("mbe", "persistence", false));
            html.push_str(&form_row_select("database", "Type", "node-input-persistence", &[("memory", "In-Memory")]));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("mbe", "security", false));
            html.push_str(&form_row("user", "Username", "node-input-username", ""));
            html.push_str(&form_row_password("lock", "Password", "node-input-password"));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "opcua read" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&tab_bar("or", &[("settings", "Settings"), ("options", "Options")]));
            html.push_str(&tab_content_start("or", "settings", true));
            html.push_str(&form_row_select(
                "cog",
                "Action",
                "node-input-action",
                &[("read", "Read"), ("subscribe", "Subscribe"), ("monitor", "Monitor")],
            ));
            html.push_str(&form_row("crosshairs", "Node ID", "node-input-nodeId", "ns=2;s=Temperature"));
            html.push_str(&conditional_section_start("or-interval-section"));
            html.push_str(&form_row_number("clock", "Interval (ms)", "node-input-intervalMs", "1000"));
            html.push_str(conditional_section_end());
            html.push_str(&conditional_section_start("or-deadband-section"));
            html.push_str(&form_row_number("sliders", "Deadband", "node-input-deadband", "0"));
            html.push_str(conditional_section_end());
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("or", "options", false));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "opcua write" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row("crosshairs", "Node ID", "node-input-nodeId", "ns=2;s=Setpoint"));
        }
        "bacnet read" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_number("map-marker", "Address", "node-input-address", "0"));
            html.push_str(&form_row_number("bars", "Quantity", "node-input-quantity", "1"));
        }
        "bacnet write" => {
            html.push_str(&name_row());
            html.push_str(&form_row_config_node(type_name, "Server"));
            html.push_str(&form_row_number("map-marker", "Address", "node-input-address", "0"));
        }
        _ => {
            // Generic template with just name
            html.push_str(&name_row());
        }
    }

    html
}

fn get_global_node_template_html(type_name: &str) -> String {
    let mut html = String::new();

    match type_name {
        "postgres-config" | "timescaledb-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&tab_bar("pc", &[("connection", "Connection"), ("security", "Security"), ("pool", "Pool")]));
            html.push_str(&tab_content_start("pc", "connection", true));
            html.push_str(&typed_input_row("server", "Host", "node-config-input-host", "node-config-input-hostType"));
            html.push_str(&typed_input_row("cog", "Port", "node-config-input-port", "node-config-input-portType"));
            html.push_str(&typed_input_row(
                "database",
                "Database",
                "node-config-input-dbname",
                "node-config-input-dbnameType",
            ));
            html.push_str(&cfg_form_row_checkbox("lock", "SSL", "ssl"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("pc", "security", false));
            html.push_str(&cfg_form_row("user", "User", "user", "postgres"));
            html.push_str(&cfg_form_row_password("lock", "Password", "password"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("pc", "pool", false));
            html.push_str(&cfg_form_row_number("cog", "Pool Size", "poolMaxSize", "10"));
            html.push_str(&cfg_form_row_number("clock", "Connect Timeout (ms)", "connectTimeoutMs", "5000"));
            html.push_str(&cfg_form_row_number("clock", "Idle Timeout (ms)", "idleTimeoutMs", "30000"));
            html.push_str(&cfg_form_row("cog", "Application Name", "applicationName", "rust-red"));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "sqlite-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&cfg_form_row("file", "Database path", "path", "data.db"));
        }
        "mssql-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&cfg_form_row("server", "Host", "host", "localhost"));
            html.push_str(&cfg_form_row_number("cog", "Port", "port", "1433"));
            html.push_str(&cfg_form_row("database", "Database", "database", "mydb"));
            html.push_str(&cfg_form_row("user", "User", "user", "sa"));
            html.push_str(&cfg_form_row_password("lock", "Password", "password"));
        }
        "influxdb-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&cfg_form_row("globe", "URL", "url", "http://localhost:8086"));
            html.push_str(&cfg_form_row_password("key", "Token", "token"));
            html.push_str(&cfg_form_row("cog", "Organization", "org", "my-org"));
            html.push_str(&cfg_form_row("database", "Bucket", "bucket", "my-bucket"));
        }
        "modbus-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&tab_bar("mc", &[("settings", "Settings"), ("queue", "Queue"), ("options", "Options")]));
            html.push_str(&tab_content_start("mc", "settings", true));
            html.push_str(&cfg_form_row_select(
                "exchange",
                "Transport",
                "transport",
                &[("tcp", "TCP"), ("udp", "UDP"), ("rtu", "Serial RTU")],
            ));
            html.push_str(&cfg_form_row("server", "Host", "host", "localhost"));
            html.push_str(&cfg_form_row_number("cog", "Port", "port", "502"));
            html.push_str(&conditional_section_start("mc-serial-section"));
            html.push_str(&cfg_form_row("serial", "Serial Port", "serialPort", "/dev/ttyUSB0"));
            html.push_str(&cfg_form_row_number("cog", "Baud Rate", "baudRate", "9600"));
            html.push_str(&cfg_form_row_select("cog", "Data Bits", "dataBits", &[("7", "7"), ("8", "8")]));
            html.push_str(&cfg_form_row_select("cog", "Stop Bits", "stopBits", &[("1", "1"), ("2", "2")]));
            html.push_str(&cfg_form_row_select(
                "cog",
                "Parity",
                "parity",
                &[("none", "None"), ("even", "Even"), ("odd", "Odd")],
            ));
            html.push_str(conditional_section_end());
            html.push_str(&cfg_form_row_number("cog", "Unit ID", "unitId", "1"));
            html.push_str(&cfg_form_row_number("clock", "Timeout (ms)", "timeoutMs", "5000"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("mc", "queue", false));
            html.push_str(&cfg_form_row_checkbox("list", "Parallel Unit IDs", "parallelUnitIds"));
            html.push_str(&cfg_form_row_checkbox("file-text", "Queue Log", "queueLogEnabled"));
            html.push_str(&cfg_form_row_checkbox("database", "Buffer Commands", "bufferCommands"));
            html.push_str(&cfg_form_row_number("clock", "Command Delay (ms)", "commandDelay", "0"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("mc", "options", false));
            html.push_str(&cfg_form_row_checkbox("heartbeat", "Keep Alive", "keepAlive"));
            html.push_str(&cfg_form_row_number("clock", "Reconnect Timeout (ms)", "reconnectTimeout", "5000"));
            html.push_str(&cfg_form_row_checkbox("plug", "Auto Connect", "autoConnect"));
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "opcua-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&tab_bar("oc", &[("connection", "Connection"), ("security", "Security")]));
            html.push_str(&tab_content_start("oc", "connection", true));
            html.push_str(&cfg_form_row("globe", "Endpoint", "endpoint", "opc.tcp://localhost:4840"));
            html.push_str(tab_content_end());
            html.push_str(&tab_content_start("oc", "security", false));
            html.push_str(&cfg_form_row_select(
                "shield",
                "Security Policy",
                "securityPolicy",
                &[
                    ("None", "None"),
                    ("Basic128Rsa15", "Basic128Rsa15"),
                    ("Basic256", "Basic256"),
                    ("Basic256Sha256", "Basic256Sha256"),
                ],
            ));
            html.push_str(&cfg_form_row_select(
                "lock",
                "Security Mode",
                "securityMode",
                &[("None", "None"), ("Sign", "Sign"), ("SignAndEncrypt", "SignAndEncrypt")],
            ));
            html.push_str(&cfg_form_row_select(
                "user",
                "Auth Method",
                "authMethod",
                &[("anonymous", "Anonymous"), ("credentials", "Credentials"), ("certificate", "Certificate")],
            ));
            html.push_str(&conditional_section_start("oc-credentials-section"));
            html.push_str(&cfg_form_row("user", "Username", "username", ""));
            html.push_str(&cfg_form_row_password("lock", "Password", "password"));
            html.push_str(conditional_section_end());
            html.push_str(&conditional_section_start("oc-certificate-section"));
            html.push_str(&cfg_form_row("file", "Certificate Path", "certPath", ""));
            html.push_str(&cfg_form_row("key", "Private Key Path", "keyPath", ""));
            html.push_str(conditional_section_end());
            html.push_str(tab_content_end());
            html.push_str(tab_bar_close());
        }
        "bacnet-config" => {
            html.push_str(&cfg_name_row());
            html.push_str(&cfg_form_row_number("cog", "Device ID", "device_id", "0"));
            html.push_str(&cfg_form_row("server", "Target Host", "target_host", ""));
            html.push_str(&cfg_form_row_number("cog", "Target Port", "target_port", "47808"));
        }
        _ => {
            // Generic config template
            html.push_str(&cfg_name_row());
        }
    }

    html
}

/// Get full editor config for a node type: (category, color, inputs, outputs, icon, defaults_js, align)
fn get_node_editor_config(
    type_name: &str,
    is_global: bool,
) -> (&'static str, &'static str, usize, usize, &'static str, String, &'static str) {
    if is_global {
        let defaults = get_global_node_defaults(type_name);
        return ("config", "#C0DEED", 0, 0, "cog.svg", defaults, "");
    }

    let (cat, color, icon) = categorize_node_v2(type_name);
    let defaults = get_flow_node_defaults(type_name);
    (cat, color, 1, 1, icon, defaults, "left")
}

fn categorize_node_v2(type_name: &str) -> (&'static str, &'static str, &'static str) {
    match type_name {
        t if t.contains("mqtt") => ("network", "#c1975b", "bridge.svg"),
        t if t.contains("postgres") || t.contains("timescale") => ("storage", "#e2d96e", "db.svg"),
        t if t.contains("mssql") || t.contains("sqlite") => ("storage", "#e2d96e", "db.svg"),
        t if t.contains("influxdb") => ("storage", "#dbc08e", "db.svg"),
        t if t.contains("modbus") => ("modbus", "#E9967A", "modbus.svg"),
        t if t.contains("opcua") => ("opcua", "#3FADB5", "serial.svg"),
        t if t.contains("bacnet") => ("storage", "#c1975b", "serial.svg"),
        _ => ("function", "#a6bbcf", "function.svg"),
    }
}

fn get_flow_node_defaults(type_name: &str) -> String {
    let mut d = String::from("            name: {value:\"\"},\n");

    // Map each flow node type to its config node type for the dropdown picker
    let config_type = match type_name {
        "postgres-query" => Some("postgres-config"),
        "sqlite-query" => Some("sqlite-config"),
        "mssql-query" => Some("mssql-config"),
        "timescaledb-query" => Some("timescaledb-config"),
        "influxdb-in" | "influxdb-out" => Some("influxdb-config"),
        "modbus read" | "modbus write" | "modbus-flex-getter" | "modbus-flex-writer" => Some("modbus-config"),
        "opcua read" | "opcua write" => Some("opcua-config"),
        "bacnet read" | "bacnet write" => Some("bacnet-config"),
        _ => None,
    };

    if let Some(ct) = config_type {
        // Modbus/bacnet/opcua flow JSON uses camelCase configNode;
        // DB nodes use snake_case config_node in their existing flow data.
        let key = match type_name {
            "modbus read" | "modbus write" | "modbus-flex-getter" | "modbus-flex-writer" | "bacnet read"
            | "bacnet write" | "opcua read" | "opcua write" => "configNode",
            _ => "config_node",
        };
        d.push_str(&format!("            {key}: {{value:\"\", type:\"{ct}\", required: true}},\n"));
    }

    match type_name {
        "postgres-query" | "mssql-query" | "sqlite-query" => {
            d.push_str("            query: {value:\"\", required: true},\n");
            d.push_str("            timeout_ms: {value:30000},\n");
            d.push_str("            output_mode: {value:\"rows\"},\n");
        }
        "timescaledb-query" => {
            d.push_str("            query: {value:\"\", required: true},\n");
            d.push_str("            timeout_ms: {value:30000},\n");
        }
        "influxdb-in" => {
            d.push_str("            measurement: {value:\"\"},\n");
            d.push_str("            tag_columns: {value:[]},\n");
            d.push_str("            field_columns: {value:[]},\n");
            d.push_str("            timestamp_column: {value:\"\"},\n");
        }
        "influxdb-out" => {
            d.push_str("            query: {value:\"\", required: true},\n");
            d.push_str("            timeout_ms: {value:30000},\n");
        }
        "modbus read" => {
            d.push_str("            functionCode: {value:\"readHoldingRegisters\"},\n");
            d.push_str("            address: {value:0},\n");
            d.push_str("            quantity: {value:1},\n");
            d.push_str("            dataType: {value:\"uint16\"},\n");
            d.push_str("            pollRate: {value:5000},\n");
            d.push_str("            pollRateUnit: {value:\"ms\"},\n");
        }
        "bacnet read" => {
            d.push_str("            address: {value:0},\n");
            d.push_str("            quantity: {value:1},\n");
        }
        "modbus write" => {
            d.push_str("            functionCode: {value:\"writeSingleRegister\"},\n");
            d.push_str("            address: {value:0},\n");
            d.push_str("            dataType: {value:\"uint16\"},\n");
        }
        "modbus-flex-getter" | "modbus-flex-writer" => {
            d.push_str("            dataType: {value:\"uint16\"},\n");
        }
        "modbus-server" => {
            d.push_str("            host: {value:\"127.0.0.1\"},\n");
            d.push_str("            port: {value:5020},\n");
            d.push_str("            coilCount: {value:100},\n");
            d.push_str("            registerCount: {value:100},\n");
        }
        "mqtt broker embedded" => {
            d.push_str("            host: {value:\"127.0.0.1\"},\n");
            d.push_str("            port: {value:1883},\n");
            d.push_str("            max_connections: {value:100},\n");
            d.push_str("            wsEnabled: {value:false},\n");
            d.push_str("            wsPath: {value:\"/mqtt\"},\n");
            d.push_str("            persistence: {value:\"memory\"},\n");
            d.push_str("            username: {value:\"\"},\n");
            d.push_str("            password: {value:\"\"},\n");
        }
        "bacnet write" => {
            d.push_str("            address: {value:0},\n");
        }
        "opcua read" => {
            d.push_str("            nodeId: {value:\"\", required: true},\n");
            d.push_str("            action: {value:\"read\"},\n");
            d.push_str("            intervalMs: {value:1000},\n");
            d.push_str("            deadband: {value:0},\n");
        }
        "opcua write" => {
            d.push_str("            nodeId: {value:\"\", required: true},\n");
        }
        _ => {}
    }

    d
}

fn get_global_node_defaults(type_name: &str) -> String {
    let mut d = String::from("            name: {value:\"\"},\n");

    match type_name {
        "postgres-config" | "timescaledb-config" => {
            d.push_str("            host: {value:\"localhost\"},\n");
            d.push_str("            hostType: {value:\"str\"},\n");
            d.push_str("            port: {value:5432},\n");
            d.push_str("            portType: {value:\"num\"},\n");
            d.push_str("            dbname: {value:\"\"},\n");
            d.push_str("            dbnameType: {value:\"str\"},\n");
            d.push_str("            user: {value:\"\"},\n");
            d.push_str("            password: {value:\"\"},\n");
            d.push_str("            ssl: {value:false},\n");
            d.push_str("            poolMaxSize: {value:10},\n");
            d.push_str("            connectTimeoutMs: {value:5000},\n");
            d.push_str("            idleTimeoutMs: {value:30000},\n");
            d.push_str("            applicationName: {value:\"rust-red\"},\n");
        }
        "sqlite-config" => {
            d.push_str("            path: {value:\"data.db\"},\n");
        }
        "mssql-config" => {
            d.push_str("            host: {value:\"localhost\"},\n");
            d.push_str("            port: {value:1433},\n");
            d.push_str("            database: {value:\"\"},\n");
            d.push_str("            user: {value:\"\"},\n");
            d.push_str("            password: {value:\"\"},\n");
        }
        "influxdb-config" => {
            d.push_str("            url: {value:\"http://localhost:8086\"},\n");
            d.push_str("            token: {value:\"\"},\n");
            d.push_str("            org: {value:\"\"},\n");
            d.push_str("            bucket: {value:\"\"},\n");
        }
        "modbus-config" => {
            d.push_str("            host: {value:\"localhost\"},\n");
            d.push_str("            port: {value:502},\n");
            d.push_str("            transport: {value:\"tcp\"},\n");
            d.push_str("            unitId: {value:1},\n");
            d.push_str("            timeoutMs: {value:5000},\n");
            d.push_str("            serialPort: {value:\"/dev/ttyUSB0\"},\n");
            d.push_str("            baudRate: {value:9600},\n");
            d.push_str("            dataBits: {value:\"8\"},\n");
            d.push_str("            stopBits: {value:\"1\"},\n");
            d.push_str("            parity: {value:\"none\"},\n");
            d.push_str("            parallelUnitIds: {value:false},\n");
            d.push_str("            queueLogEnabled: {value:false},\n");
            d.push_str("            bufferCommands: {value:false},\n");
            d.push_str("            commandDelay: {value:0},\n");
            d.push_str("            keepAlive: {value:true},\n");
            d.push_str("            reconnectTimeout: {value:5000},\n");
            d.push_str("            autoConnect: {value:true},\n");
        }
        "opcua-config" => {
            d.push_str("            endpoint: {value:\"opc.tcp://localhost:4840\"},\n");
            d.push_str("            securityPolicy: {value:\"None\"},\n");
            d.push_str("            securityMode: {value:\"None\"},\n");
            d.push_str("            authMethod: {value:\"anonymous\"},\n");
            d.push_str("            username: {value:\"\"},\n");
            d.push_str("            password: {value:\"\"},\n");
            d.push_str("            certPath: {value:\"\"},\n");
            d.push_str("            keyPath: {value:\"\"},\n");
        }
        "bacnet-config" => {
            d.push_str("            device_id: {value:0},\n");
            d.push_str("            target_host: {value:\"\"},\n");
            d.push_str("            target_port: {value:47808},\n");
        }
        _ => {}
    }

    d
}

/// Recursively process node directory at runtime
async fn process_node_directory_runtime(dir: &std::path::Path, html_content: &mut String) {
    use std::future::Future;
    use std::pin::Pin;

    fn process_dir_recursive<'a>(
        dir: &'a std::path::Path,
        html_content: &'a mut String,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            if let Ok(entries) = tokio::fs::read_dir(dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();

                    if path.is_dir() {
                        // Skip lib directory - they contain files for dynamic services
                        if path.file_name().and_then(|s| s.to_str()) == Some("lib") {
                            continue;
                        }

                        // Recursively process subdirectories
                        process_dir_recursive(&path, html_content).await;
                    } else if path.extension().and_then(|s| s.to_str()) == Some("html") {
                        // Handle HTML files
                        if let Ok(file_content) = tokio::fs::read_to_string(&path).await {
                            extract_node_html_content_runtime(&file_content, &path, html_content);
                        }
                    }
                }
            }
        })
    }

    process_dir_recursive(dir, html_content).await;
}

/// Extract node HTML content at runtime
fn extract_node_html_content_runtime(file_content: &str, file_path: &std::path::Path, output: &mut String) {
    // Extract module name from file path
    let module_name = extract_module_name_runtime(file_path);

    // Add red-module separator
    output.push_str(&format!("<!-- --- [red-module:{module_name}] --- -->\n"));

    // Add original file content
    output.push_str(file_content);

    // Ensure content ends with a newline
    if !file_content.ends_with('\n') {
        output.push('\n');
    }
}

/// Extract module name from file path at runtime
fn extract_module_name_runtime(file_path: &std::path::Path) -> String {
    if let Some(file_name) = file_path.file_name().and_then(|f| f.to_str()) {
        // Remove .html extension
        let name_without_ext = file_name.trim_end_matches(".html");

        // For all Node-RED core nodes, use "node-red/nodename" pattern
        // Extract node name part (remove numeric prefix)
        let node_name = if let Some(pos) = name_without_ext.find('-') {
            // Remove numeric prefix, e.g. "20-inject" -> "inject"
            &name_without_ext[pos + 1..]
        } else {
            // No prefix, use directly (e.g. "view", "rbe")
            name_without_ext
        };

        // Always use node-red/ prefix for core nodes
        format!("node-red/{node_name}")
    } else {
        "unknown".to_string()
    }
}

/// Get fallback node HTML config
fn get_fallback_nodes_html() -> String {
    r#"<script type="text/javascript">
// Node-RED node configurations (fallback)
(function() {
    // Inject node
    RED.nodes.registerType('inject',{
        category: 'common',
        color: '#a6bbcf',
        defaults: {
            name: {value:""},
            topic: {value:""},
            payload: {value:"", type:"msg"},
            payloadType: {value:"date"},
            repeat: {value:""},
            crontab: {value:""},
            once: {value:false}
        },
        inputs:0,
        outputs:1,
        icon: "inject.svg",
        label: function() {
            return this.name||this.topic||"inject";
        }
    });

    // Debug node
    RED.nodes.registerType('debug',{
        category: 'common',
        color: '#87a980',
        defaults: {
            name: {value:""},
            active: {value:true},
            console: {value:"false"},
            complete: {value:"false", required:true}
        },
        inputs:1,
        outputs:0,
        icon: "debug.svg",
        label: function() {
            return this.name||"debug";
        }
    });

    // Function node
    RED.nodes.registerType('function',{
        category: 'function',
        color: '#fdd0a2',
        defaults: {
            name: {value:""},
            func: {value:"return msg;"},
            outputs: {value:1},
            noerr: {value:0,required:true}
        },
        inputs:1,
        outputs:1,
        icon: "function.svg",
        label: function() {
            return this.name||"function";
        }
    });
})();
</script>"#
        .to_string()
}

/// Get node module info
pub async fn get_node_module(
    Extension(state): Extension<Arc<WebState>>,
    Path(module_name): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let registry_guard = state.registry.read().await;
    if let Some(registry) = registry_guard.as_ref() {
        // Lookup module info from registry
        for (_, meta_node) in registry.all().iter() {
            if meta_node.module == module_name {
                let module_info = serde_json::json!({
                    "name": meta_node.module,
                    "version": meta_node.version,
                    "enabled": true,
                    "local": meta_node.local,
                    "user": meta_node.user
                });
                return Ok(Json(module_info));
            }
        }
    }

    Err(StatusCode::NOT_FOUND)
}

/// Install node module
pub async fn install_node_module(
    Extension(_state): Extension<Arc<WebState>>,
    Json(_payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Node module installation is now managed by registry, just return unimplemented status here
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Enable/disable node module
pub async fn toggle_node_module(
    Extension(_state): Extension<Arc<WebState>>,
    Path(_module_name): Path<String>,
    Json(_payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Node module enable/disable is now managed by registry, just return unimplemented status here
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Uninstall node module
pub async fn uninstall_node_module(
    Extension(_state): Extension<Arc<WebState>>,
    Path(_module_name): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Node module uninstall is now managed by registry, just return unimplemented status here
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Get node message directory
pub async fn get_node_messages(
    Extension(state): Extension<Arc<WebState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let lang = params.get("lng").unwrap_or(&"en-US".to_string()).clone();

    log::info!("Getting node messages for language: {lang}");

    // Use static_dir from WebState
    let static_dir = &state.static_dir;

    // Try to load the locale file from the new structure
    let locale_path = static_dir.join("locales").join(&lang).join("messages.json");

    match tokio::fs::read_to_string(&locale_path).await {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(json) => Ok(Json(json)),
            Err(_) => {
                log::warn!("Invalid JSON in locale file: {}", locale_path.display());
                get_fallback_node_messages(&state, &lang).await
            }
        },
        Err(_) => {
            log::warn!("Locale file not found: {}", locale_path.display());
            // If the specific locale isn't found, try fallback strategies
            get_fallback_node_messages(&state, &lang).await
        }
    }
}

/// Get node set info
pub async fn get_node_set(
    Extension(state): Extension<Arc<WebState>>,
    Path((module_name, set_name)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let registry_guard = state.registry.read().await;
    if let Some(registry) = registry_guard.as_ref() {
        // Lookup node set info from registry
        for (_, meta_node) in registry.all().iter() {
            if meta_node.module == module_name {
                let node_set = serde_json::json!({
                    "id": format!("{}/{}", module_name, set_name),
                    "module": module_name,
                    "set": set_name,
                    "enabled": true,
                    "nodes": [meta_node.type_]
                });
                return Ok(Json(node_set));
            }
        }
    }

    Err(StatusCode::NOT_FOUND)
}

/// Enable/disable node set
pub async fn toggle_node_set(
    Extension(_state): Extension<Arc<WebState>>,
    Path((_module_name, _set_name)): Path<(String, String)>,
    Json(_payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Node set enable/disable is now managed by registry, just return unimplemented status here
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Get node set messages
pub async fn get_node_set_messages(
    Extension(state): Extension<Arc<WebState>>,
    Path((module_name, set_name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let lang = params.get("lng").unwrap_or(&"en-US".to_string()).clone();

    log::info!("Getting node set messages for {module_name}/{set_name} in language: {lang}");

    // Use static_dir from WebState
    let static_dir = &state.static_dir;

    // Try to load the locale file from the new structure
    let locale_path = static_dir.join("locales").join(&lang).join("messages.json");

    match tokio::fs::read_to_string(&locale_path).await {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(full_locale) => {
                // Look for the specific namespace in the locale data
                // Try different namespace formats: module_name, set_name, or the combination
                let formatted_slash = format!("{module_name}/{set_name}");
                let formatted_underscore = format!("{module_name}_{set_name}");
                let possible_keys = vec![&module_name, &set_name, &formatted_slash, &formatted_underscore];

                for key in possible_keys {
                    if let Some(namespace_data) = full_locale.get(key) {
                        return Ok(Json(namespace_data.clone()));
                    }
                }

                // If no specific namespace found, return empty object
                Ok(Json(serde_json::json!({})))
            }
            Err(_) => {
                log::warn!("Invalid JSON in locale file: {}", locale_path.display());
                get_fallback_node_set_messages(&state, &module_name, &set_name, &lang).await
            }
        },
        Err(_) => {
            log::warn!("Locale file not found: {}", locale_path.display());
            // If the specific locale isn't found, try fallback strategies
            get_fallback_node_set_messages(&state, &module_name, &set_name, &lang).await
        }
    }
}

/// Get fallback node messages with fallback strategies
async fn get_fallback_node_messages(state: &WebState, requested_lang: &str) -> Result<Json<Value>, StatusCode> {
    let static_dir = &state.static_dir;

    // Strategy 1: Try primary language (e.g., 'en' for 'en-US')
    if requested_lang.contains('-') {
        let primary_lang = requested_lang.split('-').next().unwrap();
        let primary_path = static_dir.join("locales").join(primary_lang).join("messages.json");

        if let Ok(content) = tokio::fs::read_to_string(&primary_path).await
            && let Ok(json) = serde_json::from_str::<Value>(&content)
        {
            return Ok(Json(json));
        }
    }

    // Strategy 2: Try en-US as ultimate fallback
    if requested_lang != "en-US" {
        let en_us_path = static_dir.join("locales/en-US/messages.json");
        if let Ok(content) = tokio::fs::read_to_string(&en_us_path).await
            && let Ok(json) = serde_json::from_str::<Value>(&content)
        {
            return Ok(Json(json));
        }
    }

    // Strategy 3: Return hardcoded fallback
    Ok(Json(get_hardcoded_fallback_node_messages()))
}

/// Get fallback node set messages with fallback strategies  
async fn get_fallback_node_set_messages(
    state: &WebState,
    module_name: &str,
    set_name: &str,
    requested_lang: &str,
) -> Result<Json<Value>, StatusCode> {
    let static_dir = &state.static_dir;

    // Strategy 1: Try primary language (e.g., 'en' for 'en-US')
    if requested_lang.contains('-') {
        let primary_lang = requested_lang.split('-').next().unwrap();
        let primary_path = static_dir.join("locales").join(primary_lang).join("messages.json");

        if let Ok(content) = tokio::fs::read_to_string(&primary_path).await
            && let Ok(full_locale) = serde_json::from_str::<Value>(&content)
        {
            // Look for the specific namespace in the locale data
            let formatted_slash = format!("{module_name}/{set_name}");
            let formatted_underscore = format!("{module_name}_{set_name}");
            let possible_keys = vec![module_name, set_name, &formatted_slash, &formatted_underscore];

            for key in possible_keys {
                if let Some(namespace_data) = full_locale.get(key) {
                    return Ok(Json(namespace_data.clone()));
                }
            }
        }
    }

    // Strategy 2: Try en-US as ultimate fallback
    if requested_lang != "en-US" {
        let en_us_path = static_dir.join("locales/en-US/messages.json");
        if let Ok(content) = tokio::fs::read_to_string(&en_us_path).await
            && let Ok(full_locale) = serde_json::from_str::<Value>(&content)
        {
            let formatted_slash = format!("{module_name}/{set_name}");
            let formatted_underscore = format!("{module_name}_{set_name}");
            let possible_keys = vec![module_name, set_name, &formatted_slash, &formatted_underscore];

            for key in possible_keys {
                if let Some(namespace_data) = full_locale.get(key) {
                    return Ok(Json(namespace_data.clone()));
                }
            }
        }
    }

    // Strategy 3: Return hardcoded fallback
    Ok(Json(get_hardcoded_fallback_node_set_messages(module_name, set_name)))
}

/// Get hardcoded fallback node messages for when files aren't available
fn get_hardcoded_fallback_node_messages() -> Value {
    serde_json::json!({
        "node-red": {
            "common": {
                "label": {
                    "name": "Name",
                    "input": "Input",
                    "output": "Output",
                    "payload": "Payload",
                    "topic": "Topic"
                },
                "status": {
                    "connected": "connected",
                    "disconnected": "disconnected"
                }
            },
            "inject": {
                "inject": "inject",
                "label": {
                    "repeat": "repeat",
                    "payload": "payload",
                    "topic": "topic"
                }
            },
            "debug": {
                "output": "output",
                "label": {
                    "name": "name"
                }
            }
        }
    })
}

/// Get hardcoded fallback node set messages
fn get_hardcoded_fallback_node_set_messages(module_name: &str, set_name: &str) -> Value {
    serde_json::json!({
        format!("{}/{}", module_name, set_name): {
            "help": "Help text for this node set",
            "label": "Node Set Label",
            "description": "Node set description"
        }
    })
}
