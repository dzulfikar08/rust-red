use std::collections::HashMap;

use crate::runtime::model::*;
use crate::*;

pub fn evaluate(expression: &str, data: &serde_json::Value) -> crate::Result<serde_json::Value> {
    let arena = bumpalo::Bump::new();
    let jsonata =
        jsonata_rs::JsonAta::new(expression, &arena).map_err(|e| anyhow::anyhow!("JSONata parse error: {e}"))?;

    let input = serde_json::to_string(data)?;
    let result = jsonata.evaluate(Some(&input), None).map_err(|e| anyhow::anyhow!("JSONata evaluation error: {e}"))?;

    if result.is_undefined() {
        return Ok(serde_json::Value::Null);
    }

    let output = result.serialize(false);
    let value: serde_json::Value = serde_json::from_str(&output)?;
    Ok(value)
}

pub fn evaluate_variant(expression: &str, data: &Variant) -> crate::Result<Variant> {
    let json_data =
        serde_json::to_value(data).map_err(|e| anyhow::anyhow!("Failed to serialize Variant to JSON: {e}"))?;
    let result = evaluate(expression, &json_data)?;
    Ok(Variant::from(result))
}

pub fn evaluate_with_env(expression: &str, env_vars: &HashMap<String, Variant>) -> crate::Result<Variant> {
    let json_data: serde_json::Value = serde_json::to_value(env_vars)?;
    let result = evaluate(expression, &json_data)?;
    Ok(Variant::from(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_path_expression() {
        let data = json!({"name": "world"});
        let result = evaluate("name", &data).unwrap();
        assert_eq!(result, json!("world"));
    }

    #[test]
    fn test_nested_path() {
        let data = json!({"address": {"city": "Berlin"}});
        let result = evaluate("address.city", &data).unwrap();
        assert_eq!(result, json!("Berlin"));
    }

    #[test]
    fn test_array_index() {
        let data = json!({"items": [10, 20, 30]});
        let result = evaluate("items[1]", &data).unwrap();
        assert_eq!(result, json!(20));
    }

    #[test]
    fn test_sum_function() {
        let data = json!({"prices": [10, 20, 30]});
        let result = evaluate("$sum(prices)", &data).unwrap();
        assert_eq!(result, json!(60));
    }

    #[test]
    fn test_count_function() {
        let data = json!({"items": [1, 2, 3, 4, 5]});
        let result = evaluate("$count(items)", &data).unwrap();
        assert_eq!(result, json!(5));
    }

    #[test]
    fn test_string_function() {
        let data = json!(42);
        let result = evaluate("$string()", &data).unwrap();
        assert_eq!(result, json!("42"));
    }

    #[test]
    fn test_filter_expression() {
        let data = json!({"items": [
            {"name": "a", "price": 10},
            {"name": "b", "price": 20},
            {"name": "c", "price": 30}
        ]});
        let result = evaluate("items[price > 15]", &data).unwrap();
        assert_eq!(result, json!([{"name": "b", "price": 20}, {"name": "c", "price": 30}]));
    }

    #[test]
    fn test_undefined_path_returns_null() {
        let data = json!({"name": "world"});
        let result = evaluate("nonexistent", &data).unwrap();
        assert_eq!(result, json!(null));
    }

    #[test]
    fn test_evaluate_variant_wrapper() {
        let data = Variant::from(json!({"value": 42}));
        let result = evaluate_variant("value", &data).unwrap();
        assert_eq!(result, Variant::from(42));
    }

    #[test]
    fn test_evaluate_with_env() {
        let mut env = HashMap::new();
        env.insert("FOO".to_string(), Variant::String("hello".to_string()));
        let result = evaluate_with_env("FOO", &env).unwrap();
        assert_eq!(result, Variant::String("hello".to_string()));
    }

    #[test]
    fn test_parse_error() {
        let data = json!(null);
        let result = evaluate("][invalid", &data);
        assert!(result.is_err());
    }
}
