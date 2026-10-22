//! A small Mustache renderer for the `query` property of `node-red-contrib-postgresql`.
//!
//! That node renders its query with Mustache.js against `{ msg }`, so `{{{msg.payload}}}` and
//! `{{msg.topic}}` work. Values are turned into text the way JavaScript does it, which is what
//! makes `INSERT INTO t VALUES ({{{msg.payload}}})` work for an array payload (`[1,2]` becomes `1,2`).
//!
//! Supported: variables (`{{x}}`, `{{{x}}}`, `{{&x}}`), dotted names, comments, sections (`{{#x}}`,
//! `{{^x}}`, `{{/x}}`) over booleans, objects and arrays, and the implicit iterator `{{.}}`.
//! Not supported: partials and delimiter changes. They give an error instead of wrong output.

use serde_json::Value;

#[derive(Debug, PartialEq)]
enum Node {
    Text(String),
    Var { path: Vec<String>, escape: bool },
    Section { path: Vec<String>, inverted: bool, children: Vec<Node> },
}

fn split_path(name: &str) -> Vec<String> {
    let name = name.trim();
    if name == "." { Vec::new() } else { name.split('.').map(|p| p.trim().to_string()).collect() }
}

fn parse(template: &str) -> crate::Result<Vec<Node>> {
    let mut stack: Vec<(Vec<String>, bool, Vec<Node>)> = Vec::new();
    let mut current: Vec<Node> = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        if start > 0 {
            current.push(Node::Text(rest[..start].to_string()));
        }
        let after = &rest[start + 2..];
        let (tag, consumed, triple) = if let Some(inner) = after.strip_prefix('{') {
            let end = inner.find("}}}").ok_or_else(|| anyhow::anyhow!("Unclosed '{{{{{{' tag in query template"))?;
            (&inner[..end], 1 + end + 3, true)
        } else {
            let end = after.find("}}").ok_or_else(|| anyhow::anyhow!("Unclosed '{{{{' tag in query template"))?;
            (&after[..end], end + 2, false)
        };
        rest = &after[consumed..];

        if triple {
            current.push(Node::Var { path: split_path(tag), escape: false });
            continue;
        }
        let tag = tag.trim();
        match tag.chars().next() {
            Some('!') => {}
            Some('&') => current.push(Node::Var { path: split_path(&tag[1..]), escape: false }),
            Some(c @ ('#' | '^')) => {
                stack.push((split_path(&tag[1..]), c == '^', std::mem::take(&mut current)));
            }
            Some('/') => {
                let (path, inverted, parent) =
                    stack.pop().ok_or_else(|| anyhow::anyhow!("Unopened section '{}' in query template", &tag[1..]))?;
                if path != split_path(&tag[1..]) {
                    return Err(anyhow::anyhow!(
                        "Closing '{}' does not match the open section in query template",
                        &tag[1..]
                    ));
                }
                let children = std::mem::replace(&mut current, parent);
                current.push(Node::Section { path, inverted, children });
            }
            Some('>') => return Err(anyhow::anyhow!("Partials are not supported in query templates")),
            Some('=') => return Err(anyhow::anyhow!("Delimiter changes are not supported in query templates")),
            _ => current.push(Node::Var { path: split_path(tag), escape: true }),
        }
    }
    if !rest.is_empty() {
        current.push(Node::Text(rest.to_string()));
    }
    if let Some((path, _, _)) = stack.pop() {
        return Err(anyhow::anyhow!("Unclosed section '{}' in query template", path.join(".")));
    }
    Ok(current)
}

fn lookup<'a>(contexts: &[&'a Value], path: &[String]) -> Option<&'a Value> {
    if path.is_empty() {
        return contexts.last().copied();
    }
    // The first segment may live in any enclosing context, the rest must follow from it.
    let first = contexts.iter().rev().find_map(|ctx| ctx.get(path[0].as_str()))?;
    path[1..].iter().try_fold(first, |value, key| value.get(key.as_str()))
}

fn number_to_js(n: &serde_json::Number) -> String {
    match n.as_f64() {
        Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 1e21 => format!("{f:.0}"),
        _ => n.to_string(),
    }
}

/// Text of a value as `String(value)` gives it in JavaScript. `null` is empty, as Mustache prints it.
pub(super) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_js(n),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn escape_html(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '/' => out.push_str("&#x2F;"),
            '`' => out.push_str("&#x60;"),
            '=' => out.push_str("&#x3D;"),
            other => out.push(other),
        }
    }
}

fn is_falsy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => true,
        Some(Value::String(s)) => s.is_empty(),
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::Array(items)) => items.is_empty(),
        Some(_) => false,
    }
}

fn render_nodes<'a>(nodes: &'a [Node], contexts: &mut Vec<&'a Value>, out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Var { path, escape } => {
                if let Some(value) = lookup(contexts, path) {
                    let text = js_string(value);
                    if *escape {
                        escape_html(&text, out);
                    } else {
                        out.push_str(&text);
                    }
                }
            }
            Node::Section { path, inverted, children } => {
                let value = lookup(contexts, path);
                if *inverted {
                    if is_falsy(value) {
                        render_nodes(children, contexts, out);
                    }
                } else if let Some(value) = value.filter(|v| !is_falsy(Some(v))) {
                    match value {
                        Value::Array(items) => {
                            for item in items {
                                contexts.push(item);
                                render_nodes(children, contexts, out);
                                contexts.pop();
                            }
                        }
                        other => {
                            contexts.push(other);
                            render_nodes(children, contexts, out);
                            contexts.pop();
                        }
                    }
                }
            }
        }
    }
}

/// Render `template` against `context`. Unknown names render as nothing.
pub(super) fn render(template: &str, context: &Value) -> crate::Result<String> {
    if !template.contains("{{") {
        return Ok(template.to_string());
    }
    let nodes = parse(template)?;
    let mut out = String::with_capacity(template.len());
    render_nodes(&nodes, &mut vec![context], &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn triple_braces_print_array_payload_like_javascript() {
        let ctx = json!({"msg": {"payload": [12.5, 3, "x"], "topic": "t"}});
        assert_eq!(render("VALUES ({{{msg.payload}}})", &ctx).unwrap(), "VALUES (12.5,3,x)");
    }

    #[test]
    fn double_braces_escape_html_like_mustache_js() {
        let ctx = json!({"msg": {"topic": "a<b & 'c'"}});
        assert_eq!(render("{{msg.topic}}", &ctx).unwrap(), "a&lt;b &amp; &#39;c&#39;");
        assert_eq!(render("{{{msg.topic}}}", &ctx).unwrap(), "a<b & 'c'");
        assert_eq!(render("{{&msg.topic}}", &ctx).unwrap(), "a<b & 'c'");
    }

    #[test]
    fn scalars_and_missing_values() {
        let ctx = json!({"msg": {"n": 7.0, "b": true, "z": null}});
        assert_eq!(render("{{{msg.n}}}|{{{msg.b}}}|{{{msg.z}}}|{{{msg.nope.deeper}}}", &ctx).unwrap(), "7|true||");
    }

    #[test]
    fn sections_iterate_and_invert() {
        let ctx = json!({"msg": {"rows": [{"id": 1}, {"id": 2}], "empty": [], "flag": true}});
        assert_eq!(render("{{#msg.rows}}[{{{id}}}]{{/msg.rows}}", &ctx).unwrap(), "[1][2]");
        assert_eq!(render("{{^msg.empty}}none{{/msg.empty}}{{#msg.flag}}yes{{/msg.flag}}", &ctx).unwrap(), "noneyes");
        assert_eq!(render("{{#msg.rows}}{{#id}}.{{/id}}{{/msg.rows}}", &ctx).unwrap(), "..");
    }

    #[test]
    fn implicit_iterator_and_comments() {
        let ctx = json!({"msg": {"ids": [1, 2, 3]}});
        assert_eq!(render("{{! note }}{{#msg.ids}}{{{.}}},{{/msg.ids}}", &ctx).unwrap(), "1,2,3,");
    }

    #[test]
    fn plain_sql_is_returned_untouched() {
        assert_eq!(render("SELECT 1", &json!({})).unwrap(), "SELECT 1");
    }

    #[test]
    fn malformed_templates_are_errors() {
        assert!(render("{{#a}}x", &json!({})).is_err());
        assert!(render("{{a", &json!({})).is_err());
        assert!(render("{{> p}}", &json!({})).is_err());
        assert!(render("{{/a}}", &json!({})).is_err());
    }
}
