use super::*;
pub fn scaffold(path: &Path, id: &str, name: &str) -> Result<Value> {
    if !id_ok(id) || name.is_empty() || name.len() > 100 {
        return Err("Use a valid widget ID and a name up to 100 bytes".into());
    }
    // create_dir is exclusive: existing work is never replaced, even if empty.
    fs::create_dir(path).map_err(err)?;
    let result = (|| {
        let mut m: Value =
            serde_json::from_str(include_str!("../sdk/starter/widget.json")).map_err(err)?;
        m["id"] = json!(id);
        m["name"] = json!(name);
        atomic_json(&path.join("widget.json"), &m)?;
        for (file, content) in [
            ("View.qml", include_str!("../sdk/starter/View.qml")),
            ("Settings.qml", include_str!("../sdk/starter/Settings.qml")),
            ("README.md", include_str!("../sdk/starter/README.md")),
        ] {
            let mut out = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path.join(file))
                .map_err(err)?;
            out.write_all(content.as_bytes()).map_err(err)?;
            out.sync_all().map_err(err)?;
        }
        validate(path)?;
        Ok(json!({"created":path,"id":id,"coreApi":2}))
    })();
    // Preserve a partial scaffold on I/O failure; never recursively delete a user path.
    result
}
pub fn dependencies(m: &Value) -> Result<()> {
    if let Some(d) = m.get("dependencies") {
        let object = d.as_object().ok_or("Dependencies must be an object")?;
        if object.keys().any(|k| k != "commands") {
            return Err("Unsupported dependency kind".into());
        }
        if let Some(commands) = d.get("commands") {
            let commands = commands
                .as_array()
                .filter(|a| a.len() <= 16)
                .ok_or("Declare at most 16 commands")?;
            for value in commands {
                let name = value
                    .as_str()
                    .filter(|s| {
                        !s.is_empty()
                            && s.len() <= 80
                            && s.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
                    })
                    .ok_or("Command dependency must be a basename")?;
                let path = Path::new("/usr/bin").join(name);
                if !fs::metadata(path)
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                {
                    return Err(format!("Missing command dependency: {name}"));
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dependencies_are_names_and_never_executed() {
        assert!(dependencies(&json!({"dependencies":{"commands":["sh"]}})).is_ok());
        for command in ["../sh", "/bin/sh", "$(id)", "missing-widget-dependency-123"] {
            assert!(dependencies(&json!({"dependencies":{"commands":[command]}})).is_err());
        }
        assert!(
            dependencies(&json!({"dependencies":{"origins":["https://example.com"]}})).is_err()
        );
    }
}
