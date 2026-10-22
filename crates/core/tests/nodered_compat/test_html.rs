//! HTML node compatibility tests.
//!
//! Verifies that the HTML node extracts elements from HTML documents
//! using CSS selectors, matching Node-RED behavior.

use serde_json::json;

use super::harness::TestHarness;

/// HTML: extract text content from `<p>` elements.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_extract_text() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "p", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<html><body><p>Hello</p></body></html>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].as_str().unwrap(), "Hello");
}

/// HTML: extract HTML markup (default ret="html").
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_extract_html_markup() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "p", "ret": "html", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<html><body><p>Hello</p></body></html>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 1);
    let html_str = arr[0].as_str().unwrap();
    // `ret: "html"` is the inner html of the element, as in Node-RED
    assert_eq!(html_str, "Hello");
}

/// HTML: extract multiple matching elements returns array.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_extract_multiple() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "li", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect("1", json!({"payload": "<ul><li>One</li><li>Two</li><li>Three</li></ul>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str().unwrap(), "One");
    assert_eq!(arr[1].as_str().unwrap(), "Two");
    assert_eq!(arr[2].as_str().unwrap(), "Three");
}

/// HTML: selector matches nothing returns empty array.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_no_match() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "h2", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs =
        harness.inject_and_collect("1", json!({"payload": "<html><body><p>No h2 here</p></body></html>"}), 1).await;

    assert_eq!(msgs.len(), 1);
    let payload = msgs[0].get("payload").expect("Missing payload");
    let arr = payload.as_array().expect("Payload should be an array");
    assert!(arr.is_empty(), "Expected empty array when no elements match");
}

/// HTML: multi-output mode sends separate messages per element with parts.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_multi_output() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "li", "ret": "text", "as": "multi", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": "<ul><li>A</li><li>B</li></ul>"}), 2).await;

    assert_eq!(msgs.len(), 2, "Expected 2 separate messages in multi mode");
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), "A");
    assert_eq!(msgs[1].get("payload").unwrap().as_str().unwrap(), "B");
}

const PAGE: &str = "<html><body><h1>Title</h1><p>Text</p><ol><li>Blue</li><li>Red</li></ol>\
<span><img src=\"foo.png\"></span></body></html>";

/// HTML: `ret: "html"` returns the inner html of a list, not the list element itself.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_inner_html_of_list() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "ol", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": PAGE}), 1).await;

    let payload = msgs[0].get("payload").unwrap().as_array().unwrap();
    assert_eq!(payload[0].as_str().unwrap(), "<li>Blue</li><li>Red</li>");
}

/// HTML: `ret: "attr"` returns an object of the attributes of each element.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_return_attributes() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "span img", "ret": "attr", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": PAGE}), 1).await;

    let payload = msgs[0].get("payload").unwrap().as_array().unwrap();
    assert_eq!(payload.len(), 1);
    assert_eq!(payload[0].as_object().unwrap().get("src").unwrap().as_str().unwrap(), "foo.png");
}

/// HTML: `ret: "compl"` returns the complete element.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_return_complete_element() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "tag": "p", "ret": "compl", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": PAGE}), 1).await;

    let payload = msgs[0].get("payload").unwrap().as_array().unwrap();
    assert_eq!(payload[0].as_str().unwrap(), "<p>Text</p>");
}

/// HTML: the result goes to the input property when `property` is set.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_alternative_property() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "property": "foo", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"foo": PAGE, "select": "h1"}), 1).await;

    let foo = msgs[0].get("foo").unwrap().as_array().expect("result must replace msg.foo");
    assert_eq!(foo[0].as_str().unwrap(), "Title");
    assert!(msgs[0].get("payload").is_none(), "msg.payload must not be created");
}

/// HTML: `outproperty` receives the result and the input is left alone.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_alternative_in_and_out_properties() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "property": "foo", "outproperty": "bar", "tag": "h1",
         "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"foo": PAGE}), 1).await;

    let bar = msgs[0].get("bar").unwrap().as_array().unwrap();
    assert_eq!(bar[0].as_str().unwrap(), "Title");
    assert_eq!(msgs[0].get("foo").unwrap().as_str().unwrap(), PAGE);
}

/// HTML: `property` and `outproperty` may be nested paths.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_nested_properties() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "property": "data.page", "outproperty": "result.items",
         "tag": "li", "ret": "text", "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"data": {"page": PAGE}}), 1).await;

    let items = msgs[0].get("result").unwrap().as_object().unwrap().get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].as_str().unwrap(), "Red");
}

/// HTML: multi output puts each match in `outproperty`.
#[cfg(feature = "nodes_html")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_multi_output_alternative_out_property() {
    let flow = json!([
        {"id": "100", "type": "tab"},
        {"id": "1", "z": "100", "type": "html", "outproperty": "item", "tag": "li", "ret": "text", "as": "multi",
         "wires": [["99"]]},
        {"id": "99", "z": "100", "type": "test-once"}
    ]);
    let harness = TestHarness::from_flow_json(flow);
    let msgs = harness.inject_and_collect("1", json!({"payload": PAGE}), 2).await;

    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].get("item").unwrap().as_str().unwrap(), "Blue");
    assert_eq!(msgs[1].get("item").unwrap().as_str().unwrap(), "Red");
    assert_eq!(msgs[0].get("payload").unwrap().as_str().unwrap(), PAGE);
}
