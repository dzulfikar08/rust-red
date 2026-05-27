//! Example echo plugin for Rust-Red.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec;
use rust_red_wasm_sdk::*;

struct EchoNode;
impl Default for EchoNode {
    fn default() -> Self {
        EchoNode
    }
}

impl WasmNodeHandler for EchoNode {
    fn info() -> WasmNodeInfo {
        WasmNodeInfo {
            node_type: String::from("example/echo"),
            red_name: String::from("echo"),
            module: String::from("example"),
            version: String::from("1.0.0"),
            inputs: 1,
            outputs: 1,
            color: Some(String::from("#3FADB5")),
            icon: None,
            label: Some(String::from("echo")),
            label_style: None,
            palette_label: None,
            align: None,
            editor_template: None,
            capabilities: vec![],
        }
    }

    fn on_start(_config: WasmMessage) {
        log("info", "EchoNode started");
    }

    fn process(msg: WasmMessage) -> ProcessResult {
        log("debug", "EchoNode: processing message");
        ProcessResult::single(msg)
    }

    fn on_stop() {
        log("info", "EchoNode stopped");
    }
}

export_node!(EchoNode);
