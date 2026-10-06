//! Narrow experimental renderer: Core installs the executable; packages are data only.
use super::*;
pub fn is(m: &Value) -> bool {
    m["renderer"] == "mygo-github-experimental"
}
pub fn settings(m: &Value) -> bool {
    declarative::is(m) || is(m)
}
pub fn contract(m: &Value) -> Result<()> {
    if m["coreApi"] != 3 || m["capabilities"] != json!(["github"]) {
        return Err("MyGo GitHub requires API 3 and only the GitHub capability".into());
    }
    for key in [
        "entryPoint",
        "settingsEntryPoint",
        "dependencies",
        "refresh",
        "view",
    ] {
        if m.get(key).is_some() {
            return Err(format!("MyGo GitHub does not accept {key}"));
        }
    }
    // Reuse the bounded data-only settings contract, never load package QML.
    let mut generated = m.clone();
    generated["renderer"] = json!("declarative");
    generated["requires"] = json!(["declarative-v1"]);
    generated.as_object_mut().unwrap().remove("capabilities");
    generated["view"] = json!({"type":"text","value":"GitHub"});
    declarative::contract(&generated)?;
    if m["settingsSchema"]["properties"]["username"]["type"] != "string"
        || m["settingsSchema"]["properties"]["username"]["maxLength"] != 39
        || m["settingsSchema"]["properties"]["palette"]["enum"]
            != json!(["GitHub green", "Theme accent"])
    {
        return Err("MyGo GitHub requires username and palette settings".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_contract_is_data_only() {
        let m: Value =
            serde_json::from_str(include_str!("../examples/mygo-github/widget.json")).unwrap();
        contract(&m).unwrap();
        for key in [
            "entryPoint",
            "settingsEntryPoint",
            "dependencies",
            "refresh",
            "view",
        ] {
            let mut bad = m.clone();
            bad[key] = json!("untrusted.qml");
            assert!(contract(&bad).is_err());
        }
        assert!(settings(&m));
        assert!(!declarative::is(&m));
    }
}
