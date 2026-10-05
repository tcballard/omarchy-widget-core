//! Public weather projection and generation-bound permission.
use super::*;
use std::sync::{Arc, Mutex, OnceLock};
static CACHE: OnceLock<Arc<Mutex<public_data::Cache>>> = OnceLock::new();
pub fn coordinates(lat: &str, lon: &str) -> Result<(String, String)> {
    fn coordinate(s: &str, max: f64) -> Result<String> {
        let n = s.parse::<f64>().map_err(err)?;
        if !n.is_finite() || n.abs() > max {
            return Err("Invalid weather coordinates".into());
        }
        Ok(format!("{:.2}", (n * 100.0).round() / 100.0 + 0.0))
    }
    Ok((coordinate(lat, 90.0)?, coordinate(lon, 180.0)?))
}
pub(crate) fn project(v: &Value) -> Result<Value> {
    let c = &v["current"];
    let temperature = c["temperature_2m"]
        .as_f64()
        .filter(|n| n.is_finite() && (-150.0..=100.0).contains(n))
        .ok_or("Invalid weather temperature")?;
    let code = c["weather_code"]
        .as_u64()
        .filter(|n| *n <= 99)
        .ok_or("Invalid weather code")?;
    let time = c["time"].as_u64().ok_or("Missing weather timestamp")?;
    Ok(
        json!({"temperatureC":temperature,"weatherCode":code,"observedAt":time,"attribution":"Open-Meteo"}),
    )
}
pub fn request(package: &str, lat: &str, lon: &str, active: bool) -> Result<Value> {
    let (lat, lon) = coordinates(lat, lon)?;
    public_data::request(&CACHE, package, format!("{lat},{lon}"), active, move || {
        let bytes = public_data::fetch(public_data::Endpoint::Weather(&lat, &lon))?;
        project(&serde_json::from_slice::<Value>(&bytes).map_err(err)?)
    })
}
pub fn declared(m: &Value) -> bool {
    m["capabilities"]
        .as_array()
        .is_some_and(|a| a.contains(&json!("weather")))
}
pub fn authorised(r: &Registry, l: &Value, package: &str) -> bool {
    r.source(l, package)
        .ok()
        .is_some_and(|p| l["packages"][package]["weatherGrant"] == json!(p.to_string_lossy()))
}
pub fn grant(r: &Registry, id: &str, choice: &str) -> Result<Value> {
    if !["allow", "deny"].contains(&choice) {
        return Err("Use allow or deny".into());
    }
    let _lock = r.lock()?;
    let mut l = r.layout()?;
    let source = r.source(&l, id)?;
    if !declared(&manifest(&source)?) {
        return Err("Package does not declare weather".into());
    }
    if !l["packages"][id].is_object() {
        return Err("Reinstall legacy package before granting weather".into());
    }
    l["packages"][id]["weatherGrant"] = if choice == "allow" {
        json!(source.to_string_lossy())
    } else {
        Value::Null
    };
    r.commit(&mut l)?;
    Ok(json!(true))
}
