//! Data-only view contract. No package URLs, expressions or executable content.
use super::*;
pub fn is(m: &Value) -> bool {
    m["renderer"] == "declarative"
}
fn keys(v: &Value, allowed: &[&str]) -> Result<()> {
    if v.as_object()
        .is_none_or(|o| o.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        return Err("Unsupported declarative field".into());
    }
    Ok(())
}
fn binding(v: &Value, m: &Value, item: bool) -> Result<()> {
    if v.as_str().is_some_and(|s| s.len() <= 256) {
        return Ok(());
    }
    keys(v, &["setting", "item"])?;
    if v.as_object().unwrap().len() != 1 {
        return Err("Use one binding source".into());
    }
    if let Some(key) = v["setting"].as_str() {
        if m["settingsSchema"]["properties"].get(key).is_some() {
            return Ok(());
        }
    }
    if item
        && v["item"]
            .as_str()
            .is_some_and(|s| ["label", "zone"].contains(&s))
    {
        return Ok(());
    }
    Err("Invalid declarative binding".into())
}
fn node(v: &Value, m: &Value, depth: usize, count: &mut usize, item: bool) -> Result<()> {
    *count += 1;
    if depth > 6 || *count > 64 {
        return Err("View exceeds depth 6 / 64 nodes".into());
    }
    match v["type"].as_str().unwrap_or("") {
        "row" | "column" => {
            keys(v, &["type", "children"])?;
            let children = v["children"]
                .as_array()
                .filter(|a| !a.is_empty() && a.len() <= 12)
                .ok_or("Layout needs 1–12 children")?;
            for child in children {
                node(child, m, depth + 1, count, item)?;
            }
        }
        "repeat" => {
            keys(v, &["type", "items", "child"])?;
            if item {
                return Err("Nested repeats are not supported".into());
            }
            let key = v["items"]["setting"]
                .as_str()
                .ok_or("Repeat needs a settings array")?;
            binding(&v["items"], m, false)?;
            let schema = &m["settingsSchema"]["properties"][key];
            if schema["type"] != "array" || schema["maxItems"].as_u64().is_none_or(|n| n > 12) {
                return Err("Repeat arrays require maxItems <= 12".into());
            }
            node(&v["child"], m, depth + 1, count, true)?;
        }
        "text" => {
            keys(v, &["type", "value", "style"])?;
            binding(&v["value"], m, item)?;
            if v.get("style")
                .is_some_and(|s| !["caption", "body", "heading"].iter().any(|x| s == x))
            {
                return Err("Unknown text style".into());
            }
        }
        "clock" => {
            keys(v, &["type", "timezone", "mode"])?;
            binding(&v["timezone"], m, item)?;
            binding(&v["mode"], m, item)?;
        }
        _ => return Err("Unknown declarative component".into()),
    }
    Ok(())
}
pub fn contract(m: &Value) -> Result<()> {
    if m["coreApi"] != 3
        || !m["requires"]
            .as_array()
            .is_some_and(|a| a.contains(&json!("declarative-v1")))
    {
        return Err("Declarative widgets require API 3 and declarative-v1".into());
    }
    for key in [
        "entryPoint",
        "settingsEntryPoint",
        "dependencies",
        "capabilities",
        "refresh",
    ] {
        if m.get(key).is_some() {
            return Err(format!("Declarative v1 does not accept {key}"));
        }
    }
    if m["settingsSchema"]["type"] != "object" {
        return Err("Declarative settings require a schema".into());
    }
    node(&m["view"], m, 0, &mut 0, false)?;
    let fields = m["settingsUi"]
        .as_array()
        .filter(|a| a.len() <= 16)
        .ok_or("Declare up to 16 settings controls")?;
    let mut seen = std::collections::BTreeSet::new();
    for field in fields {
        keys(field, &["key", "label", "type"])?;
        let key = field["key"].as_str().ok_or("Settings key required")?;
        if !seen.insert(key)
            || !field["label"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && s.len() <= 100)
        {
            return Err("Invalid settings control".into());
        }
        let schema = &m["settingsSchema"]["properties"][key];
        match field["type"].as_str().unwrap_or("") {
            "text" if schema["type"] == "string" => (),
            "choice"
                if schema["type"] == "string"
                    && schema["enum"].as_array().is_some_and(|a| {
                        !a.is_empty() && a.len() <= 8 && a.iter().all(Value::is_string)
                    }) =>
            {
                ()
            }
            "timezone-list"
                if schema["type"] == "array"
                    && schema["maxItems"].as_u64().is_some_and(|n| n <= 12)
                    && schema["items"]["type"] == "object"
                    && schema["items"]["properties"]["label"]["type"] == "string"
                    && schema["items"]["properties"]["zone"]["type"] == "string" =>
            {
                ()
            }
            _ => return Err("Unsupported generated settings control".into()),
        }
    }
    Ok(())
}
pub fn validate_settings(m: &Value, value: &Value) -> Result<()> {
    if !is(m) {
        return Ok(());
    }
    for field in m["settingsUi"].as_array().into_iter().flatten() {
        if field["type"] == "timezone-list" {
            let key = field["key"].as_str().unwrap_or("");
            for city in value[key].as_array().into_iter().flatten() {
                if !city["zone"].as_str().is_some_and(zone_ok) {
                    return Err("Choose an installed IANA timezone, such as Europe/London".into());
                }
            }
        }
    }
    Ok(())
}
// Linux libc performs timezone/DST conversion using the host tzdata. This is a
// short-lived, single-threaded CLI command, not a renderer-global TZ mutation.
// The mutex also serialises unit tests calling this function in one process.
static CLOCK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[repr(C)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    day: i32,
    month: i32,
    year: i32,
    weekday: i32,
    yearday: i32,
    dst: i32,
    offset: std::ffi::c_long,
    zone: *const std::ffi::c_char,
}
unsafe extern "C" {
    fn tzset();
    fn localtime_r(time: *const std::ffi::c_long, out: *mut Tm) -> *mut Tm;
}
fn zone_ok(zone: &str) -> bool {
    if zone.len() > 128
        || !safe_relative(zone)
        || !zone
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_+-".contains(&b))
    {
        return false;
    }
    let base = Path::new("/usr/share/zoneinfo");
    base.join(zone).canonicalize().is_ok_and(|p| {
        let mut magic = [0; 4];
        p.starts_with(base)
            && File::open(p)
                .and_then(|mut f| f.read_exact(&mut magic))
                .is_ok()
            && &magic == b"TZif"
    })
}
fn clock_at(zones: &Value, seconds: i64) -> Result<Value> {
    let zones = zones
        .as_array()
        .filter(|a| a.len() <= 128)
        .ok_or("At most 128 clock zones")?;
    let _guard = CLOCK_LOCK.lock().map_err(err)?;
    let original = env::var_os("TZ");
    let mut rows = serde_json::Map::new();
    for zone in zones {
        let Some(zone) = zone.as_str().filter(|z| zone_ok(z)) else {
            continue;
        };
        env::set_var("TZ", format!(":/usr/share/zoneinfo/{zone}"));
        let mut value = std::mem::MaybeUninit::<Tm>::uninit();
        let time = seconds as std::ffi::c_long;
        unsafe {
            tzset();
            if !localtime_r(&time, value.as_mut_ptr()).is_null() {
                let value = value.assume_init();
                rows.insert(zone.into(),json!({"hour":value.hour,"minute":value.min,"time":format!("{:02}:{:02}",value.hour,value.min),"date":format!("{:04}-{:02}-{:02}",value.year+1900,value.month+1,value.day)}));
            }
        }
    }
    match original {
        Some(value) => env::set_var("TZ", value),
        None => env::remove_var("TZ"),
    }
    unsafe {
        tzset();
    }
    Ok(json!({"zones":rows,"sampledAt":seconds}))
}
pub fn clock_times(input: &str) -> Result<Value> {
    if input.len() > 32768 {
        return Err("Clock request too large".into());
    }
    clock_at(
        &serde_json::from_str(input).map_err(err)?,
        (reveal::now() / 1000) as i64,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_only_contract() {
        let m: Value =
            serde_json::from_str(include_str!("../examples/declarative-clock/widget.json"))
                .unwrap();
        assert!(contract(&m).is_ok());
        for key in [
            "entryPoint",
            "settingsEntryPoint",
            "dependencies",
            "capabilities",
        ] {
            let mut bad = m.clone();
            bad[key] = json!("evil.qml");
            assert!(contract(&bad).is_err());
        }
        for view in [
            json!({"type":"qml","source":"file:///tmp/evil.qml"}),
            json!({"type":"text","value":{"expression":"Qt.quit()"}}),
        ] {
            let mut bad = m.clone();
            bad["view"] = view;
            assert!(contract(&bad).is_err());
        }
        let mut bad = m.clone();
        bad["settingsSchema"]["properties"]["cities"]["maxItems"] = json!(1000);
        assert!(contract(&bad).is_err());
    }
    #[test]
    fn timezone_dst_and_paths() {
        assert!(!zone_ok("../../etc/passwd"));
        assert!(!zone_ok("/etc/passwd"));
        let zones = json!(["UTC", "Asia/Tokyo", "Europe/London"]);
        let winter = clock_at(&zones, 1704067200).unwrap();
        assert_eq!(winter["zones"]["UTC"]["time"], "00:00");
        assert_eq!(winter["zones"]["Asia/Tokyo"]["time"], "09:00");
        let summer = clock_at(&zones, 1719792000).unwrap();
        assert_eq!(summer["zones"]["Europe/London"]["time"], "01:00");
    }
}
