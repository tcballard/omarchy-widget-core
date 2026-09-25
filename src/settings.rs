//! Deliberately small declarative schema/migration language; never executes package code.
use super::*;
pub fn version(m: &Value) -> u64 {
    m["settingsVersion"].as_u64().unwrap_or(1)
}
fn schema(s: &Value, depth: usize) -> Result<()> {
    if depth > 8 {
        return Err("Settings schema nesting exceeds 8".into());
    }
    let object = s.as_object().ok_or("Settings schema must be an object")?;
    if object.keys().any(|k| {
        ![
            "type",
            "properties",
            "required",
            "additionalProperties",
            "items",
            "maxItems",
            "maxLength",
            "minimum",
            "maximum",
            "enum",
        ]
        .contains(&k.as_str())
    }) {
        return Err("Unsupported settings schema keyword".into());
    }
    if !["object", "array", "string", "number", "integer", "boolean"]
        .contains(&s["type"].as_str().unwrap_or(""))
    {
        return Err("Unsupported settings schema type".into());
    }
    if let Some(props) = s.get("properties") {
        let props = props
            .as_object()
            .filter(|p| p.len() <= 64)
            .ok_or("Invalid schema properties")?;
        for (key, value) in props {
            if key.is_empty() || key.len() > 100 {
                return Err("Invalid settings key".into());
            }
            schema(value, depth + 1)?;
        }
    }
    if let Some(items) = s.get("items") {
        schema(items, depth + 1)?;
    }
    if let Some(required) = s.get("required") {
        let values = required
            .as_array()
            .filter(|v| v.len() <= 64)
            .ok_or("Invalid required fields")?;
        if values
            .iter()
            .any(|v| v.as_str().is_none_or(|k| s["properties"].get(k).is_none()))
        {
            return Err("Required fields must have property schemas".into());
        }
    }
    for key in ["maxLength", "maxItems"] {
        if s.get(key)
            .is_some_and(|v| v.as_u64().is_none_or(|n| n > 8192))
        {
            return Err(format!("Invalid {key}"));
        }
    }
    for key in ["minimum", "maximum"] {
        if s.get(key).is_some_and(|v| v.as_f64().is_none()) {
            return Err(format!("Invalid {key}"));
        }
    }
    if s.get("additionalProperties")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err("additionalProperties must be boolean".into());
    }
    if s.get("enum")
        .is_some_and(|v| v.as_array().is_none_or(|a| a.is_empty() || a.len() > 64))
    {
        return Err("Invalid settings enum".into());
    }
    Ok(())
}
fn check(s: &Value, v: &Value, path: &str) -> Result<()> {
    let valid = match s["type"].as_str().unwrap_or("") {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.as_i64().is_some() || v.as_u64().is_some(),
        "boolean" => v.is_boolean(),
        _ => false,
    };
    if !valid {
        return Err(format!("{path}: expected {}", s["type"]));
    }
    if let Some(allowed) = s["enum"].as_array() {
        if !allowed.contains(v) {
            return Err(format!("{path}: choose an allowed value"));
        }
    }
    if let Some(n) = v.as_f64() {
        if s["minimum"].as_f64().is_some_and(|min| n < min)
            || s["maximum"].as_f64().is_some_and(|max| n > max)
        {
            return Err(format!("{path}: value outside allowed range"));
        }
    }
    if let Some(text) = v.as_str() {
        if text.chars().count() > s["maxLength"].as_u64().unwrap_or(8192) as usize {
            return Err(format!("{path}: text is too long"));
        }
    }
    if let Some(items) = v.as_array() {
        if items.len() > s["maxItems"].as_u64().unwrap_or(8192) as usize {
            return Err(format!("{path}: too many items"));
        }
        if !s["items"].is_null() {
            for (i, item) in items.iter().enumerate() {
                check(&s["items"], item, &format!("{path}[{i}]"))?;
            }
        }
    }
    if let Some(object) = v.as_object() {
        if let Some(required) = s["required"].as_array() {
            for field in required {
                if !object.contains_key(field.as_str().unwrap()) {
                    return Err(format!("{path}: missing {}", field));
                }
            }
        }
        for (key, value) in object {
            if let Some(rule) = s["properties"].get(key) {
                check(rule, value, &format!("{path}.{key}"))?;
            } else if s["additionalProperties"] == false {
                return Err(format!("{path}: unknown field {key}"));
            }
        }
    }
    Ok(())
}
pub fn validate(m: &Value, v: &Value) -> Result<()> {
    if !v.is_object() || serde_json::to_vec(v).map_err(err)?.len() > 8192 {
        return Err("Settings must be an object up to 8 KiB".into());
    }
    if !m["settingsSchema"].is_null() {
        check(&m["settingsSchema"], v, "Settings")?;
        declarative::validate_settings(m, v)?;
    }
    Ok(())
}
pub fn contract(m: &Value) -> Result<()> {
    if m.get("settingsVersion")
        .is_some_and(|v| v.as_u64().is_none_or(|n| n == 0 || n > 1000))
    {
        return Err("settingsVersion must be 1–1000".into());
    }
    if !m["settingsSchema"].is_null() {
        schema(&m["settingsSchema"], 0)?;
        if m["settingsSchema"]["type"] != "object" {
            return Err("Settings root must be object".into());
        }
    }
    if let Some(steps) = m.get("migrations") {
        let steps = steps
            .as_array()
            .filter(|v| v.len() <= 32)
            .ok_or("Invalid migrations")?;
        let mut seen = std::collections::BTreeSet::new();
        for step in steps {
            let from = step["from"]
                .as_u64()
                .filter(|n| *n > 0 && *n < version(m))
                .ok_or("Invalid migration source")?;
            if step["to"] != from + 1 || !seen.insert(from) {
                return Err("Migrations must be unique consecutive version steps".into());
            }
            if step.as_object().is_none_or(|s| {
                s.keys()
                    .any(|k| !["from", "to", "rename", "defaults", "remove"].contains(&k.as_str()))
            }) {
                return Err("Unsupported migration operation".into());
            }
            for key in ["rename", "defaults"] {
                if step.get(key).is_some_and(|v| !v.is_object()) {
                    return Err("Migration maps must be objects".into());
                }
            }
            if let Some(rename) = step["rename"].as_object() {
                if rename.values().any(|v| !v.is_string()) {
                    return Err("Rename targets must be strings".into());
                }
            }
            if step.get("remove").is_some_and(|v| {
                v.as_array()
                    .is_none_or(|a| a.iter().any(|v| !v.is_string()))
            }) {
                return Err("Remove must be an array of keys".into());
            }
        }
    }
    validate(m, &m["defaults"])
}
pub fn migrate(m: &Value, from: u64, settings: &Value) -> Result<Value> {
    if from > version(m) {
        return Err("Settings downgrade requires rollback or a compatible package".into());
    }
    let mut result = settings.clone();
    for n in from..version(m) {
        let step = m["migrations"]
            .as_array()
            .and_then(|a| a.iter().find(|s| s["from"] == n))
            .ok_or("Missing settings migration step")?;
        let object = result.as_object_mut().ok_or("Invalid settings")?;
        if let Some(rename) = step["rename"].as_object() {
            for (old, new) in rename {
                let new = new.as_str().unwrap();
                if object.contains_key(old) {
                    if object.contains_key(new) {
                        return Err("Migration rename would overwrite a field".into());
                    }
                    let value = object.remove(old).unwrap();
                    object.insert(new.into(), value);
                }
            }
        }
        if let Some(defaults) = step["defaults"].as_object() {
            for (key, value) in defaults {
                object.entry(key).or_insert(value.clone());
            }
        }
        if let Some(remove) = step["remove"].as_array() {
            for key in remove {
                object.remove(key.as_str().unwrap());
            }
        }
    }
    validate(m, &result)?;
    Ok(result)
}
pub fn checkpoint(placements: &Value, id: &str) -> Value {
    Value::Object(placements.as_object().unwrap().iter().filter(|(_,p)|p["packageId"]==id).map(|(key,p)|(key.clone(),json!({"settings":p["settings"],"settingsVersion":p["settingsVersion"].as_u64().unwrap_or(1),"revision":p["revision"]}))).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarative_schema_and_migration_reject_data_loss() {
        let m = json!({"settingsVersion":2,"defaults":{"label":"London"},"settingsSchema":{"type":"object","properties":{"label":{"type":"string","maxLength":80}},"required":["label"],"additionalProperties":false},"migrations":[{"from":1,"to":2,"rename":{"city":"label"}}]});
        contract(&m).unwrap();
        assert_eq!(
            migrate(&m, 1, &json!({"city":"Paris"})).unwrap(),
            json!({"label":"Paris"})
        );
        assert!(migrate(&m, 1, &json!({"city":"Paris","label":"London"})).is_err());
        assert!(validate(&m, &json!({"label":9})).is_err());
        assert!(validate(&m, &json!({})).is_err());
        assert!(migrate(&m, 3, &json!({"label":"London"})).is_err());
        let mut bad = m.clone();
        bad["settingsSchema"]["$ref"] = json!("https://example.com");
        assert!(contract(&bad).is_err());
    }
}
