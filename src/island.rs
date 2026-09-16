//! Authority is attached to a listener by the supervisor, never supplied by a client.
use super::*;
use std::collections::BTreeMap;
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const REQUEST_LIMIT: u64 = 16384;
const RESPONSE_LIMIT: u64 = 2 * 1024 * 1024;

fn read_message(stream: &mut UnixStream, limit: u64, budget: Duration) -> Result<Vec<u8>> {
    let deadline = Instant::now() + budget;
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("Broker read deadline")?;
        stream.set_read_timeout(Some(remaining)).map_err(err)?;
        let mut buffer = [0u8; 4096];
        let count = stream.read(&mut buffer).map_err(err)?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len() + count > limit as usize {
            return Err("Broker message too large".into());
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}
fn write_message(stream: &mut UnixStream, mut bytes: &[u8], budget: Duration) -> Result<()> {
    let deadline = Instant::now() + budget;
    while !bytes.is_empty() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("Broker write deadline")?;
        stream.set_write_timeout(Some(remaining)).map_err(err)?;
        let count = stream.write(bytes).map_err(err)?;
        if count == 0 {
            return Err("Broker disconnected".into());
        }
        bytes = &bytes[count..];
    }
    Ok(())
}
pub fn client(path: &str, args: &[String]) -> Result<Value> {
    let mut stream = UnixStream::connect(path).map_err(err)?;
    let request = serde_json::to_vec(args).map_err(err)?;
    if request.len() > REQUEST_LIMIT as usize {
        return Err("Broker request too large".into());
    }
    write_message(&mut stream, &request, Duration::from_secs(2))?;
    stream.shutdown(Shutdown::Write).map_err(err)?;
    let bytes = read_message(&mut stream, RESPONSE_LIMIT, Duration::from_secs(8))?;
    let response: Value = serde_json::from_slice(&bytes).map_err(err)?;
    if let Some(error) = response["error"].as_str() {
        return Err(error.into());
    }
    Ok(response)
}

fn scoped(r: &Registry, package: &str, source: &Path, args: &[String]) -> Result<Value> {
    // Reject a runner from a removed/replaced generation before reading or writing state.
    let layout = r.layout()?;
    if r.source(&layout, package)? != source {
        return Err("Runner generation expired".into());
    }
    match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => {
            let mut snapshot = r.snapshot()?;
            snapshot["installed"]
                .as_array_mut()
                .ok_or("Invalid snapshot")?
                .retain(|v| v["packageId"] == package);
            for entry in snapshot["installed"].as_array_mut().unwrap() {
                entry["directory"] = json!("/widget");
            }
            snapshot["problems"] = json!([]);
            snapshot["runtime"]["managerOpen"] = json!(false);
            let edit = snapshot["runtime"]["edit"]["instance"]
                .as_str()
                .unwrap_or("");
            if layout["placements"][edit]["packageId"] != package {
                snapshot["runtime"]["edit"] = Value::Null;
            }
            Ok(snapshot)
        }
        Some("edit-done") if args.len() == 3 => {
            let _lock = r.lock()?;
            let mut current = r.layout()?;
            if current["placements"][&args[1]]["packageId"] != package {
                return Err("Instance is outside this runner's authority".into());
            }
            if current["runtime"]["edit"]["instance"] == args[1]
                && current["runtime"]["edit"]["serial"]
                    .as_u64()
                    .map(|v| v.to_string())
                    .as_deref()
                    == Some(args[2].as_str())
            {
                current["runtime"]["edit"] = Value::Null;
                r.commit(&mut current)?;
            }
            Ok(json!(true))
        }
        Some(op @ ("save" | "place" | "hide" | "workspace")) => {
            let expected = if op == "hide" { 2 } else { 3 };
            if args.len() != expected || layout["placements"][&args[1]]["packageId"] != package {
                return Err("Instance is outside this runner's authority".into());
            }
            r.placement(&args[1], op, args.get(2).map(String::as_str))
        }
        Some("control")
            if args.len() == 2 && ["arrange", "finish-arrange"].contains(&args[1].as_str()) =>
        {
            r.control(&args[1])
        }
        _ => Err("Operation is not available to widget runners".into()),
    }
}

fn serve(listener: &UnixListener, r: &Registry, package: &str, source: &Path) -> Result<()> {
    // A fixed connection budget prevents one runner starving all the others.
    for _ in 0..1 {
        let (mut stream, _) = match listener.accept() {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(err(e)),
        };
        let result = (|| {
            let bytes = read_message(&mut stream, REQUEST_LIMIT, Duration::from_millis(100))?;
            let args: Vec<String> = serde_json::from_slice(&bytes).map_err(err)?;
            scoped(r, package, source, &args)
        })();
        let response =
            result.unwrap_or_else(|e| json!({"error":e.chars().take(240).collect::<String>()}));
        let _ = write_message(
            &mut stream,
            response.to_string().as_bytes(),
            Duration::from_millis(100),
        );
    }
    Ok(())
}

// wl-mitm's ask hook is a deterministic policy decision, not a user prompt.
// Denials use a fatal Wayland error; never silently discard object creation.
pub fn wayland_policy(args: &[String], message: &str) -> Result<Value> {
    if message.len() > REQUEST_LIMIT as usize {
        return Err("Wayland policy payload too large".into());
    }
    let v: Value = serde_json::from_str(message).map_err(err)?;
    let allowed = match args
        .get(1)
        .map(String::as_str)
        .zip(args.get(2).map(String::as_str))
    {
        Some(("zwlr_layer_shell_v1", "get_layer_surface")) => v["layer"] == 1,
        Some(("zwlr_layer_surface_v1", "set_layer")) => v["layer"] == 1,
        Some(("zwlr_layer_surface_v1", "set_exclusive_zone")) => v["zone"] == 0 || v["zone"] == -1,
        Some(("zwlr_layer_surface_v1", "set_keyboard_interactivity")) => {
            v["keyboard_interactivity"] == 0 || v["keyboard_interactivity"] == 2
        }
        _ => false,
    };
    if allowed {
        Ok(json!(true))
    } else {
        Err("Wayland capability denied".into())
    }
}

struct Runner {
    source: PathBuf,
    directory: PathBuf,
    listener: UnixListener,
    unit: String,
    child: Child,
}
impl Drop for Runner {
    fn drop(&mut self) {
        resources::stop(&self.unit);
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.directory);
    }
}
fn toml_string(value: &Path) -> Result<String> {
    // JSON quoting is also valid TOML for these paths; reject non-UTF8 paths.
    serde_json::to_string(value.to_str().ok_or("Non-UTF8 runtime path")?).map_err(err)
}
fn runtime_directory(base: &Path, package: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    package.hash(&mut hash);
    // Socket paths have a small kernel limit. The listener binding, not this hash,
    // determines authority. A collision fails create_dir, never reuses a listener.
    base.join(format!("{:016x}", hash.finish()))
}
fn launch(
    config: &Path,
    base: &Path,
    package: &str,
    source: &Path,
    upstream: &Path,
) -> Result<Runner> {
    let directory = runtime_directory(base, package);
    fs::create_dir(&directory).map_err(err)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).map_err(err)?;
    let socket = directory.join("broker");
    let listener = UnixListener::bind(&socket).map_err(err)?;
    listener.set_nonblocking(true).map_err(err)?;
    let display = directory.join("wayland");
    let policy = format!(
        "[socket]\nlisten = {}\nupstream = {}\n[exec]\nask_cmd = {}\n{}",
        toml_string(&display)?,
        toml_string(upstream)?,
        toml_string(&config.join("wayland-policy"))?,
        fs::read_to_string(config.join("wayland-filter.toml")).map_err(err)?
    );
    let policy_path = directory.join("wayland.toml");
    fs::write(&policy_path, policy).map_err(err)?;
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let unit = format!(
        "omarchy-widget-island-{}-{}-{sequence}.service",
        std::process::id(),
        directory.file_name().unwrap().to_string_lossy()
    );
    let child = Command::new("/usr/bin/systemd-run")
        .args(resources::service_args(&unit)?)
        .arg(config.join("bin/omarchy-widget"))
        .arg("island-worker")
        .arg(config)
        .arg(source)
        .arg(&directory)
        .stdin(Stdio::null())
        .spawn()
        .map_err(err)?;
    Ok(Runner {
        source: source.into(),
        directory,
        listener,
        unit,
        child,
    })
}

pub fn supervise(config: &Path) -> Result<Value> {
    let r = Registry::from_env()?;
    let runtime = PathBuf::from(env::var("XDG_RUNTIME_DIR").map_err(err)?);
    let display = env::var("WAYLAND_DISPLAY").map_err(err)?;
    if !runtime.is_absolute() || !safe_relative(&display) || display.contains('/') {
        return Err("Invalid Wayland environment".into());
    }
    let upstream = runtime.join(display);
    let base = runtime.join(format!("omarchy-widget-islands-{}", std::process::id()));
    fs::create_dir(&base).map_err(err)?;
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).map_err(err)?;
    let mut manager = Command::new("/usr/bin/qs")
        .args(["--no-duplicate", "-p"])
        .arg(config)
        .env("OMARCHY_WIDGET_ROLE", "manager")
        .env_remove("OMARCHY_WIDGET_BROKER")
        .stdin(Stdio::null())
        .spawn()
        .map_err(err)?;
    let mut runners: BTreeMap<String, Runner> = BTreeMap::new();
    let mut failures: BTreeMap<String, (u32, Instant)> = BTreeMap::new();
    let result = (|| {
        let mut next_scan = Instant::now();
        loop {
            if manager.try_wait().map_err(err)?.is_some() {
                return Err("Widget manager exited".into());
            }
            if Instant::now() >= next_scan {
                let snapshot = r.snapshot()?;
                let edit = snapshot["runtime"]["edit"]["instance"]
                    .as_str()
                    .unwrap_or("");
                let mut wanted = BTreeMap::new();
                for entry in snapshot["installed"].as_array().ok_or("Invalid snapshot")? {
                    if entry["placement"]["enabled"] == true || entry["instanceId"] == edit {
                        wanted.insert(
                            entry["packageId"]
                                .as_str()
                                .ok_or("Missing package")?
                                .to_owned(),
                            PathBuf::from(entry["directory"].as_str().ok_or("Missing source")?),
                        );
                    }
                }
                runners.retain(|id, runner| wanted.get(id) == Some(&runner.source));
                for (id, source) in wanted {
                    if runners.contains_key(&id) {
                        continue;
                    }
                    if let Some((count, when)) = failures.get(&id) {
                        if *count >= 3 || when.elapsed() < Duration::from_secs(5) {
                            continue;
                        }
                    }
                    match launch(config, &base, &id, &source, &upstream) {
                        Ok(runner) => {
                            runners.insert(id, runner);
                        }
                        Err(e) => {
                            eprintln!("Widget {id}: {e}");
                            let count = failures.get(&id).map_or(1, |v| v.0 + 1);
                            failures.insert(id.clone(), (count, Instant::now()));
                            if !runners
                                .values()
                                .any(|runner| runner.directory == runtime_directory(&base, &id))
                            {
                                let _ = fs::remove_dir_all(runtime_directory(&base, &id));
                            }
                        }
                    }
                }
                next_scan = Instant::now() + Duration::from_secs(1);
            }
            let mut dead = Vec::new();
            for (id, runner) in &mut runners {
                if runner.child.try_wait().map_err(err)?.is_some() {
                    dead.push(id.clone());
                    continue;
                }
                serve(&runner.listener, &r, id, &runner.source)?;
            }
            for id in dead {
                runners.remove(&id);
                let count = failures.get(&id).map_or(1, |v| v.0 + 1);
                failures.insert(id.clone(), (count, Instant::now()));
                eprintln!("Widget {id} stopped; failure {count}/3 (restart Core to reset)");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    drop(runners);
    let _ = manager.kill();
    let _ = manager.wait();
    let _ = fs::remove_dir_all(base);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trickled_requests_have_an_absolute_deadline() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        let attack = std::thread::spawn(move || {
            for _ in 0..50 {
                if writer.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let start = Instant::now();
        assert!(read_message(&mut reader, REQUEST_LIMIT, Duration::from_millis(100)).is_err());
        assert!(start.elapsed() < Duration::from_millis(750));
        drop(reader);
        attack.join().unwrap();
    }
    #[test]
    fn layer_policy_denies_privileged_requests() {
        for (interface, request, field, allowed, denied) in [
            (
                "zwlr_layer_shell_v1",
                "get_layer_surface",
                "layer",
                vec![1],
                vec![0, 2, 3],
            ),
            (
                "zwlr_layer_surface_v1",
                "set_layer",
                "layer",
                vec![1],
                vec![0, 2, 3],
            ),
            (
                "zwlr_layer_surface_v1",
                "set_exclusive_zone",
                "zone",
                vec![-1, 0],
                vec![1, 1920],
            ),
            (
                "zwlr_layer_surface_v1",
                "set_keyboard_interactivity",
                "keyboard_interactivity",
                vec![0, 2],
                vec![1, 3],
            ),
        ] {
            let args = ["wayland-policy", interface, request].map(str::to_owned);
            for value in allowed {
                assert!(wayland_policy(&args, &json!({field:value}).to_string()).is_ok());
            }
            for value in denied {
                assert!(wayland_policy(&args, &json!({field:value}).to_string()).is_err());
            }
            assert!(wayland_policy(&args, "{}").is_err());
        }
        assert!(wayland_policy(&[], "{}").is_err());
    }
    #[test]
    fn broker_scopes_reads_writes_and_generations() {
        let base = env::temp_dir().join(format!("island-test-{}", std::process::id()));
        fs::create_dir(&base).unwrap();
        let r = Registry {
            data: base.join("data"),
            state: base.join("state"),
        };
        for id in ["io.test.a", "io.test.b"] {
            let source = base.join(id);
            fs::create_dir(&source).unwrap();
            fs::write(source.join("View.qml"), "import QtQuick\nItem {}").unwrap();
            atomic_json(&source.join("widget.json"), &json!({"schemaVersion":2,"kind":"desktop-widget","coreApi":2,"id":id,"name":id,"version":"0.0.2","entryPoint":"View.qml","families":["medium"],"defaultFamily":"medium","defaults":{"secret":id}})).unwrap();
            r.install(&source).unwrap();
            r.placement(id, "add", None).unwrap();
        }
        let source = r.source(&r.layout().unwrap(), "io.test.a").unwrap();
        let call = |args: &[&str]| {
            scoped(
                &r,
                "io.test.a",
                &source,
                &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
        };
        let snapshot = call(&["list"]).unwrap();
        assert_eq!(snapshot["installed"].as_array().unwrap().len(), 1);
        assert!(!snapshot.to_string().contains("io.test.b"));
        assert_eq!(snapshot["installed"][0]["directory"], "/widget");
        for args in [
            vec!["save", "io.test.b", r#"{"revision":0,"settings":{}}"#],
            vec!["hide", "io.test.b"],
            vec!["workspace", "io.test.b", "2"],
            vec!["remove", "io.test.a"],
            vec!["install", "/widget"],
            vec!["configure", "io.test.a", "{}"],
        ] {
            assert!(call(&args).is_err());
        }
        call(&[
            "save",
            "io.test.a",
            r#"{"revision":0,"settings":{"city":"Paris"}}"#,
        ])
        .unwrap();
        assert!(call(&[
            "save",
            "io.test.a",
            r#"{"revision":0,"settings":{"city":"London"}}"#
        ])
        .is_err());
        assert_eq!(
            r.layout().unwrap()["placements"]["io.test.a"]["settings"]["city"],
            "Paris"
        );
        let socket_path = base.join("broker-test");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let socket_string = socket_path.to_str().unwrap().to_owned();
        let request = std::thread::spawn(move || client(&socket_string, &["list".into()]));
        serve(&listener, &r, "io.test.a", &source).unwrap();
        let response = request.join().unwrap().unwrap();
        assert_eq!(response["installed"].as_array().unwrap().len(), 1);
        assert!(!response.to_string().contains("io.test.b"));
        r.deploy(&base.join("io.test.a"), true).unwrap();
        assert!(call(&["list"]).is_err());
        fs::remove_dir_all(base).unwrap();
    }
}
