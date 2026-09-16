//! Trusted, read-only Hyprland monitor snapshot. Never expose the control socket to a runner.
use super::*;
use std::os::unix::net::UnixStream;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
type Cached = Option<(PathBuf, Instant, Result<Value>)>;
static CACHE: OnceLock<Mutex<Cached>> = OnceLock::new();

pub fn valid(value: &Value) -> bool {
    value.is_null() || value.as_u64().is_some_and(|n| (1..=9999).contains(&n))
}
fn parse(bytes: &[u8]) -> Result<Value> {
    let raw: Value = serde_json::from_slice(bytes).map_err(err)?;
    let monitors = raw
        .as_array()
        .filter(|a| a.len() <= 64)
        .ok_or("Invalid monitor list")?;
    let mut result = serde_json::Map::new();
    for monitor in monitors {
        if monitor["disabled"] == true {
            continue;
        }
        let name = monitor["name"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 120)
            .ok_or("Invalid monitor name")?;
        if result.contains_key(name) {
            return Err("Duplicate monitor name".into());
        }
        let workspace = &monitor["activeWorkspace"];
        // Current Hyprland uses an address; older builds use an integer id.
        let id = if let Some(id) = workspace["id"].as_i64() {
            id
        } else if let Some(address) = workspace["address"].as_str() {
            address.parse::<i64>().unwrap_or(0)
        } else {
            return Err("Missing active workspace".into());
        };
        // Ignore special-workspace overlays: assignment means the numbered workspace underneath.
        result.insert(name.into(), json!(id));
    }
    Ok(Value::Object(result))
}
fn request(path: &Path, command: &[u8]) -> Result<Value> {
    let mut socket = UnixStream::connect(path).map_err(err)?;
    let budget = Duration::from_millis(100);
    socket.set_write_timeout(Some(budget)).map_err(err)?;
    socket.write_all(command).map_err(err)?;
    let deadline = Instant::now() + budget;
    let mut bytes = Vec::new();
    loop {
        socket
            .set_read_timeout(Some(
                deadline
                    .checked_duration_since(Instant::now())
                    .ok_or("Workspace read deadline")?,
            ))
            .map_err(err)?;
        let mut buf = [0; 4096];
        let count = socket.read(&mut buf).map_err(err)?;
        if count == 0 {
            break;
        }
        if bytes.len() + count > 65536 {
            return Err("Monitor response too large".into());
        }
        bytes.extend_from_slice(&buf[..count]);
    }
    serde_json::from_slice(&bytes).map_err(err)
}
fn query(path: &Path) -> Result<Value> {
    project(|command| request(path, command.as_bytes()))
}
fn project(mut request: impl FnMut(&str) -> Result<Value>) -> Result<Value> {
    let raw = request("j/monitors")?;
    let monitors = parse(&serde_json::to_vec(&raw).map_err(err)?)?;
    let mut option = |name: &str| -> Result<Value> {
        let v = request(&format!("j/getoption {name}"))?;
        for key in ["custom", "css", "int", "float"] {
            if !v[key].is_null() {
                return Ok(v[key].clone());
            }
        }
        Err(format!("Missing desktop option {name}"))
    };
    let inside = grid::edges(&option("general:gaps_in")?)?;
    let outside = grid::edges(&option("general:gaps_out")?)?;
    let border = option("general:border_size")?
        .as_f64()
        .filter(|v| (0.0..=100.0).contains(v))
        .ok_or("Invalid border")?;
    let radius = option("decoration:rounding")?
        .as_f64()
        .filter(|v| (0.0..=1000.0).contains(v))
        .ok_or("Invalid rounding")?;
    Ok(
        json!({"available":true,"monitors":monitors,"grids":grid::screens(&raw,inside,outside)?,"frame":{"borderWidth":border,"radius":radius}}),
    )
}
pub fn snapshot() -> Value {
    let result = (|| {
        let runtime = PathBuf::from(env::var("XDG_RUNTIME_DIR").map_err(err)?);
        let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(err)?;
        if !runtime.is_absolute()
            || signature.is_empty()
            || signature.len() > 200
            || !signature
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err("Invalid compositor session".into());
        }
        let path = runtime.join("hypr").join(signature).join(".socket.sock");
        let mut cache = CACHE.get_or_init(|| Mutex::new(None)).lock().map_err(err)?;
        if let Some((cached_path, when, result)) = cache.as_ref() {
            if *cached_path == path && when.elapsed() < Duration::from_millis(250) {
                return result.clone();
            }
        }
        let result = query(&path);
        *cache = Some((path, Instant::now(), result.clone()));
        result
    })();
    match result {
        Ok(desktop) => desktop,
        Err(_) => {
            json!({"available":false,"monitors":{},"grids":{},"error":"Desktop geometry unavailable; widgets retain their positions and wait for the compositor"})
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monitor_projection_and_validation() {
        let raw=br#"[{"name":"DP-1","activeWorkspace":{"id":2},"specialWorkspace":{"id":-99},"secret":"not forwarded"},{"name":"eDP-1","activeWorkspace":{"id":1}}]"#;
        assert_eq!(parse(raw).unwrap(), json!({"DP-1":2,"eDP-1":1}));
        assert!(parse(b"{}").is_err());
        assert!(parse(br#"[{"name":"DP-1"}]"#).is_err());
        for value in [json!(-1), json!(0), json!(10000), json!(1.5), json!("2")] {
            assert!(!valid(&value));
        }
        for value in [Value::Null, json!(1), json!(9999)] {
            assert!(valid(&value));
        }
    }
    #[test]
    fn desktop_projection_supports_legacy_and_current_hyprland() {
        for current in [false, true] {
            let result=project(|command| Ok(match command {
                "j/monitors" => json!([{"name":"DP-1","width":1920,"height":1080,"scale":1.0,"transform":0,"reserved":[0,32,0,0],"activeWorkspace":if current {json!({"address":"2","type":"numbered"})} else {json!({"id":2})}}]),
                "j/getoption general:gaps_in" => if current {json!({"css":"5 5 5 5"})} else {json!({"custom":"5 5 5 5"})},
                "j/getoption general:gaps_out" => json!({"custom":"10 20 30 40"}),
                "j/getoption general:border_size" => json!({"int":2}),
                "j/getoption decoration:rounding" => json!({"int":0}),
                _ => panic!("Unexpected compositor command")
            })).unwrap();
            assert_eq!(result["monitors"]["DP-1"], 2);
            assert_eq!(result["grids"]["DP-1"]["x"], 40.0);
            assert_eq!(result["grids"]["DP-1"]["gapX"], 10.0);
            assert_eq!(result["frame"]["radius"], 0.0);
        }
        assert!(project(|_| Ok(json!({}))).is_err());
    }
    #[test]
    fn socket_round_trip() {
        use std::os::unix::net::UnixListener;
        let path = env::temp_dir().join(format!("widget-workspace-{}.sock", std::process::id()));
        let listener = UnixListener::bind(&path).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 10];
            stream.read_exact(&mut request).unwrap();
            assert_eq!(&request, b"j/monitors");
            stream
                .write_all(br#"[{"name":"DP-1","activeWorkspace":{"id":2}}]"#)
                .unwrap();
        });
        assert_eq!(
            request(&path, b"j/monitors").unwrap(),
            json!([{"name":"DP-1","activeWorkspace":{"id":2}}])
        );
        worker.join().unwrap();
        fs::remove_file(&path).unwrap();
        assert!(query(&path).is_err());
    }
}
