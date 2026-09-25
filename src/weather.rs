//! Fixed public provider; never an arbitrary URL proxy. Cache lives in the supervisor.
use super::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
const TTL: u64 = 900_000;
#[derive(Default)]
struct Entry {
    data: Value,
    updated: u64,
    wall_updated: u64,
    touched: u64,
    owner: String,
    retry: u64,
    pending: bool,
    error: String,
}
#[derive(Default)]
struct Cache {
    entries: BTreeMap<String, Entry>,
    next_request: u64,
    package_retry: BTreeMap<String, u64>,
}
static CACHE: OnceLock<Arc<Mutex<Cache>>> = OnceLock::new();
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
fn project(v: &Value) -> Result<Value> {
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
fn public(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_documentation()
        && a != 0
        && a < 224
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 198 && (18..=19).contains(&b))
        && !(a == 192 && b == 0 && c == 0)
        && !(a == 192 && b == 88 && c == 99)
}
fn resolve() -> Result<String> {
    let output = std::process::Command::new("/usr/bin/timeout")
        .env_clear()
        .args([
            "--kill-after=1",
            "2",
            "/usr/bin/getent",
            "ahostsv4",
            "api.open-meteo.com",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(err)?;
    if !output.status.success() || output.stdout.len() > 16384 {
        return Err("Weather DNS unavailable".into());
    }
    let text = String::from_utf8(output.stdout).map_err(err)?;
    let addresses = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.parse::<std::net::Ipv4Addr>().map_err(err))
        .collect::<Result<Vec<_>>>()?;
    if addresses.is_empty() || addresses.iter().any(|ip| !public(*ip)) {
        return Err("Weather destination is not public".into());
    }
    Ok(format!("api.open-meteo.com:443:{}", addresses[0]))
}
fn fetch(lat: &str, lon: &str) -> Result<Value> {
    // No curlrc, proxy, netrc, cookies, credentials, redirects, user headers or URL input.
    // curl's byte/time caps are reinforced by a Core monotonic timeout and capped pipe reads.
    use std::process::{Command, Stdio};
    let resolved = resolve()?;
    let url=format!("https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,weather_code&timeformat=unixtime&forecast_days=1");
    let mut child = Command::new("/usr/bin/curl")
        .env_clear()
        .args([
            "--disable",
            "--silent",
            "--fail",
            "--noproxy",
            "*",
            "--proto",
            "=https",
            "--connect-timeout",
            "3",
            "--max-time",
            "5",
            "--max-filesize",
            "65536",
            "--resolve",
            &resolved,
            "--url",
            &url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(err)?;
    let mut stdout = child.stdout.take().ok_or("Missing weather output")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take(65537)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(6);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "Weather reader failed")?
        .map_err(err)?;
    if !success || bytes.len() > 65536 {
        return Err("Weather service unavailable; retrying later".into());
    }
    project(&serde_json::from_slice::<Value>(&bytes).map_err(err)?)
}
impl Cache {
    fn request(
        &mut self,
        package: &str,
        key: &str,
        now: u64,
        active: bool,
    ) -> Result<(Value, bool)> {
        let missing = !self.entries.contains_key(key);
        let eligible = active
            && now >= self.next_request
            && now >= self.package_retry.get(package).copied().unwrap_or(0);
        // Hidden misses and throttled misses consume neither memory nor another
        // user's cached data. Existing public results can still be read/shared.
        if missing && !eligible {
            return Ok((
                json!({"state":"loading","data":null,"updatedAt":0,"refreshing":false,"error":""}),
                false,
            ));
        }
        if missing {
            let owned = self.entries.values().filter(|e| e.owner == package).count();
            if owned >= 16 || self.entries.len() >= 128 {
                let victim = self
                    .entries
                    .iter()
                    .filter(|(_, e)| !e.pending && (owned < 16 || e.owner == package))
                    .min_by_key(|(_, e)| e.touched)
                    .map(|(key, _)| key.clone());
                if let Some(victim) = victim {
                    self.entries.remove(&victim);
                } else {
                    return Err("Weather is busy; retry shortly".into());
                }
            }
            self.entries.insert(
                key.into(),
                Entry {
                    owner: package.into(),
                    ..Entry::default()
                },
            );
        }
        let e = self.entries.get_mut(key).unwrap();
        if active {
            e.touched = now;
        }
        let stale = e.data.is_null() || now.saturating_sub(e.updated) >= TTL;
        let start = eligible && stale && !e.pending && now >= e.retry;
        if start {
            e.pending = true;
            self.next_request = now.saturating_add(10_000);
            if !self.package_retry.contains_key(package) && self.package_retry.len() >= MAX_PACKAGES
            {
                let oldest = self
                    .package_retry
                    .iter()
                    .min_by_key(|(_, v)| *v)
                    .map(|(k, _)| k.clone())
                    .unwrap();
                self.package_retry.remove(&oldest);
            }
            self.package_retry
                .insert(package.into(), now.saturating_add(60_000));
        }
        let state = if e.data.is_null() {
            if e.error.is_empty() {
                "loading"
            } else {
                "unavailable"
            }
        } else if stale {
            "stale"
        } else {
            "ready"
        };
        Ok((
            json!({"state":state,"data":e.data,"updatedAt":e.wall_updated,"refreshing":e.pending,"error":e.error}),
            start,
        ))
    }
    fn complete(&mut self, key: &str, now: u64, wall: u64, result: Result<Value>) {
        if let Some(e) = self.entries.get_mut(key) {
            e.pending = false;
            match result {
                Ok(data) => {
                    e.data = data;
                    e.updated = now;
                    e.wall_updated = wall;
                    e.error.clear();
                    e.retry = now.saturating_add(TTL);
                }
                Err(_) => {
                    e.error = "Weather service unavailable; retrying later".into();
                    e.retry = now.saturating_add(60_000);
                }
            }
        }
    }
}
// Linux CLOCK_BOOTTIME is monotonic and includes time spent suspended.
// Wall time is used only for the externally displayed updatedAt timestamp.
fn elapsed_ms() -> Result<u64> {
    #[repr(C)]
    struct Timespec {
        seconds: std::os::raw::c_long,
        nanos: std::os::raw::c_long,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }
    let mut time = Timespec {
        seconds: 0,
        nanos: 0,
    };
    // SAFETY: valid writable timespec, correct Linux C ABI; no retained pointer.
    if unsafe { clock_gettime(7, &mut time) } != 0 {
        return Err("Monotonic clock unavailable".into());
    }
    Ok(time.seconds as u64 * 1000 + time.nanos as u64 / 1_000_000)
}
pub fn request(package: &str, lat: &str, lon: &str, active: bool) -> Result<Value> {
    let (lat, lon) = coordinates(lat, lon)?;
    let key = format!("{lat},{lon}");
    let cache = CACHE
        .get_or_init(|| Arc::new(Mutex::new(Cache::default())))
        .clone();
    let (result, start) =
        cache
            .lock()
            .map_err(err)?
            .request(package, &key, elapsed_ms()?, active)?;
    if start {
        std::thread::spawn(move || {
            let result = fetch(&lat, &lon);
            if let Ok(mut c) = cache.lock() {
                if let Ok(now) = elapsed_ms() {
                    c.complete(&key, now, reveal::now(), result);
                }
            }
        });
    }
    Ok(result)
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_cannot_supply_urls_or_queries() {
        for lat in ["NaN", "inf", "91", "https://localhost", "0&url=x"] {
            assert!(coordinates(lat, "0").is_err());
        }
        assert_eq!(
            coordinates("51.501", "-0.121").unwrap(),
            ("51.50".into(), "-0.12".into())
        );
    }
    #[test]
    fn deduplication_stale_retry_and_hidden_budget() {
        let mut c = Cache::default();
        let key = "51.50,-0.12";
        assert!(!c.request("a", key, 100, false).unwrap().1);
        assert!(c.request("a", key, 100, true).unwrap().1);
        assert!(!c.request("a", key, 200, true).unwrap().1);
        c.complete(key, 300, 86400000, Ok(json!({"temperatureC":21})));
        assert_eq!(c.request("a", key, 400, true).unwrap().0["state"], "ready");
        assert!(c.request("a", key, TTL + 400, true).unwrap().1);
        c.complete(key, TTL + 500, 1000, Err("offline".into()));
        let (r, start) = c.request("a", key, TTL + 600, true).unwrap();
        assert!(!start);
        assert_eq!(r["state"], "stale");
        assert_eq!(r["data"]["temperatureC"], 21);
        assert!(!c.request("a", key, TTL + 100_000, false).unwrap().1);
        assert!(c.request("a", key, TTL + 100_000, true).unwrap().1);
    }
    #[test]
    fn rejects_malformed_response_and_bounds_cache() {
        assert!(project(&json!({})).is_err());
        assert_eq!(
            project(&json!({"current":{"temperature_2m":20,"weather_code":3,"time":1700000000}}))
                .unwrap()["temperatureC"],
            20.0
        );
        let mut c = Cache::default();
        for i in 0..1024 {
            c.request("a", &i.to_string(), 100, false).unwrap();
        }
        assert!(c.entries.is_empty());
        assert!(c.request("b", "extra", 100, true).unwrap().1);
        c.complete("extra", 101, 86400000, Ok(json!({"temperatureC":20})));
        // Wall time jumps backwards, while BOOTTIME advances through sleep.
        assert!(c.request("b", "extra", TTL + 102, true).unwrap().1);
        c.complete("extra", TTL + 103, 1000, Err("offline".into()));
        assert_eq!(
            c.request("b", "extra", TTL + 104, true).unwrap().0["state"],
            "stale"
        );
        assert!(c.request("b", "extra", TTL + 60104, true).unwrap().1);
        assert!(elapsed_ms().unwrap() > 0);
    }
    #[test]
    fn coordinate_churn_is_bounded_and_other_packages_can_refresh() {
        let mut c = Cache::default();
        for i in 0..300 {
            let now = 100 + i * 60000;
            let key = i.to_string();
            assert!(c.request("churn", &key, now, true).unwrap().1);
            c.complete(&key, now + 1, 1000, Ok(json!({"temperatureC":21})));
            assert!(c.entries.values().filter(|e| e.owner == "churn").count() <= 16);
            // One package cannot take the next global admission slot.
            assert!(!c.request("churn", "another", now + 10000, true).unwrap().1);
            assert!(c.request("neighbour", "shared", now + 10000, true).is_ok());
        }
        assert!(c.entries.len() <= 17);
    }
    #[test]
    fn private_and_reserved_destinations_denied() {
        for ip in [
            "0.1.2.3",
            "10.1.2.3",
            "127.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "192.168.0.1",
            "192.0.0.1",
            "198.18.0.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(!public(ip.parse().unwrap()));
        }
        assert!(public("1.1.1.1".parse().unwrap()));
    }
}
