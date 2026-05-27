//! XML node compatibility tests.
//!
//! Verifies that the XML node parses XML strings to objects and
/// converts objects back to XML strings, matching Node-RED behavior.
use serde_json::json;

use super::harness::TestHarness;

/// XML: parse a simple XML string to an object.
///
/// Input: `<root><item>val</item></root>`
/// Output: object `{root: {item: ["val"]}}`
#[cfg(feature = "nodes_xml")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn xml_parse_to_object() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "xml", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<root><item>val</item></root>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let root_obj = payload.as_object().expect("Payload should be an object");

    // Default Xml2jsOptions has explicit_root=true, explicit_array=true
    let root = root_obj.get("root").expect("Should have 'root' key");
    let root_inner = root.as_object().expect("'root' should be an object");
    let item = root_inner.get("item").expect("Should have 'item' key");
    let item_arr = item.as_array().expect("'item' should be an array");
    assert_eq!(item_arr[0].as_str().unwrap(), "val");
}

/// XML: convert object to XML string.
///
/// Input: object `{root: {item: ["val"]}}`
/// Output: XML string containing `<root>` and `<item>val</item>`.
#[cfg(feature = "nodes_xml")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn xml_stringify() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "xml", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": {"root": {"item": ["val"]}}}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let xml_str = payload.as_str().expect("Payload should be a string");

    assert!(xml_str.contains("<root>"), "XML output should contain '<root>', got: {xml_str}");
    assert!(xml_str.contains("<item>val</item>"), "XML output should contain '<item>val</item>', got: {xml_str}");
}

/// XML: parse XML with attributes.
///
/// Input: `<root><item attr="hello">val</item></root>`
/// Output: object with attribute key "$" containing attrs (merge_attrs=true by default
/// flattens attributes into the element object).
#[cfg(feature = "nodes_xml")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn xml_with_attributes() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "xml", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect("1", json!({"payload": "<root><item attr=\"hello\">val</item></root>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let root_obj = payload.as_object().expect("Payload should be an object");
    let root = root_obj.get("root").expect("Should have 'root' key").as_object().unwrap();
    let item = root.get("item").expect("Should have 'item' key").as_array().unwrap();
    let item_obj = item[0].as_object().expect("Item should be an object");

    // With merge_attrs=true (default), attributes are flattened into the element
    assert!(item_obj.get("attr").is_some(), "Item should contain attribute 'attr'");
    assert_eq!(item_obj.get("attr").unwrap().as_str().unwrap(), "hello");
}
