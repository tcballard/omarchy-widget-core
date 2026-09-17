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

fn scoped(
    r: &Registry,
    package: &str,
    source: &Path,
    serial: u64,
    args: &[String],
) -> Result<Value> {
    // Hold authority, ownership checks and mutation under the same registry lock.
    let reading = matches!(args.first().map(String::as_str), Some("list" | "weather"));
    let lock = if reading { r.read_lock()? } else { r.lock()? };
    let layout = if args.first().is_some_and(|op| op == "list") {
        r.display_layout()?.0
    } else {
        r.layout()?
    };
    if r.source(&layout, package)? != source
        || layout["runtime"]["packageControls"][package]["serial"]
            .as_u64()
            .unwrap_or(0)
            != serial
        || layout["runtime"]["packageControls"][package]["disabled"] == true
    {
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
            // Management metadata never crosses a package's broker boundary.
            snapshot["catalog"] = json!([]);
            snapshot["retained"] = json!([]);
            snapshot["runtime"]["managerOpen"] = json!(false);
            snapshot["runtime"]["packageControls"] = json!({});
            let edit = snapshot["runtime"]["edit"]["instance"]
                .as_str()
                .unwrap_or("");
            if layout["placements"][edit]["packageId"] != package {
                snapshot["runtime"]["edit"] = Value::Null;
            }
            Ok(snapshot)
        }
        Some("weather") if args.len() == 4 => {
            let p = &layout["placements"][&args[1]];
            if p["packageId"] != package
                || !weather::declared(&manifest(source)?)
                || !weather::authorised(r, &layout, package)
            {
                return Err("Weather access is not allowed for this package version".into());
            }
            let d = workspaces::snapshot();
            let positions = grid::resolve(&layout["placements"], &d);
            let monitor = positions[&args[1]]["monitor"].as_str().unwrap_or("");
            let active = layout["runtime"]["packageControls"][package]["disabled"] != true
                && p["enabled"] == true
                && !monitor.is_empty()
                && (layout["runtime"]["shown"] != false
                    || reveal::active(
                        layout["runtime"]["revealUntil"].as_u64().unwrap_or(0),
                        reveal::now(),
                    ))
                && (p["workspace"].is_null() || p["workspace"] == d["monitors"][monitor]);
            weather::request(package, &args[2], &args[3], active)
        }
        Some("content-failed")
            if args.len() == 2 && layout["placements"][&args[1]]["packageId"] == package =>
        {
            let mut layout = layout;
            let next = layout["revision"]
                .as_u64()
                .unwrap()
                .checked_add(1)
                .ok_or("Revision exhausted")?;
            layout["runtime"]["packageControls"][package] =
                json!({"disabled":true,"serial":next,"loadFailure":true});
            if layout["placements"][layout["runtime"]["edit"]["instance"].as_str().unwrap_or("")]
                ["packageId"]
                == package
            {
                layout["runtime"]["edit"] = Value::Null;
            }
            r.commit(&mut layout)?;
            Ok(json!(true))
        }
        Some("edit")
            if args.len() == 2 && layout["placements"][&args[1]]["packageId"] == package =>
        {
            r.request_edit_locked(&args[1], &lock)
        }
        Some("edit-done")
            if args.len() == 3 && layout["placements"][&args[1]]["packageId"] == package =>
        {
            r.finish_edit_locked(&args[1], &args[2], &lock)
        }
        Some(op @ ("save" | "place" | "hide" | "workspace")) => {
            let expected = if op == "hide" { 2 } else { 3 };
            if args.len() != expected || layout["placements"][&args[1]]["packageId"] != package {
                return Err("Instance is outside this runner's authority".into());
            }
            r.placement_locked(
                &args[1],
                op,
                args.get(2).map(String::as_str),
                workspaces::snapshot,
                &lock,
            )
        }
        Some("control") if args.len() == 2 && args[1] == "finish-arrange" => {
            r.control_locked(&args[1], &lock)
        }
        _ => Err("Operation is not available to widget runners".into()),
    }
}

#[derive(Default)]
struct WriteBudget(std::collections::VecDeque<Instant>);
impl WriteBudget {
    fn admit(&mut self, args: &[String], now: Instant) -> Result<()> {
        if matches!(args.first().map(String::as_str), Some("list" | "weather")) {
            return Ok(());
        }
        while self
            .0
            .front()
            .is_some_and(|start| now.saturating_duration_since(*start) >= Duration::from_secs(1))
        {
            self.0.pop_front();
        }
        if self.0.len() >= 5 {
            return Err("Runner write budget exceeded; retry shortly".into());
        }
        self.0.push_back(now);
        Ok(())
    }
}
fn serve(
    listener: &UnixListener,
    r: &Registry,
    package: &str,
    source: &Path,
    serial: u64,
    writes: &mut WriteBudget,
) -> Result<()> {
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
            writes.admit(&args, Instant::now())?;
            scoped(r, package, source, serial, &args)
        })();
        let response = result.unwrap_or_else(|e| error_response(&e));
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
    policy(args, message, reveal::permitted())
}
fn policy(args: &[String], message: &str, revealing: bool) -> Result<Value> {
    if message.len() > REQUEST_LIMIT as usize {
        return Err("Wayland policy payload too large".into());
    }
    let v: Value = serde_json::from_str(message).map_err(err)?;
    let allowed = match args
        .get(1)
        .map(String::as_str)
        .zip(args.get(2).map(String::as_str))
    {
        Some(("zwlr_layer_shell_v1", "get_layer_surface")) => {
            v["layer"] == 1 || (revealing && v["layer"] == 3)
        }
        Some(("zwlr_layer_surface_v1", "set_layer")) => {
            v["layer"] == 1 || (revealing && v["layer"] == 3)
        }
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
    serial: u64,
    reveal_until: u64,
    directory: PathBuf,
    listener: UnixListener,
    unit: String,
    child: Child,
    writes: WriteBudget,
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
    reveal_until: u64,
    serial: u64,
) -> Result<Runner> {
    let directory = runtime_directory(base, package);
    fs::create_dir(&directory).map_err(err)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).map_err(err)?;
    fs::write(directory.join("reveal-lease"), reveal_until.to_string()).map_err(err)?;
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
        serial,
        reveal_until,
        directory,
        listener,
        unit,
        child,
        writes: WriteBudget::default(),
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
    // No runner from this supervisor lifetime exists yet. A persisted editor
    // cannot refer to an open window; clear its registration before launch.
    let _ = retry_busy(|| r.clear_dead_editor(None).map(|()| json!(true)));
    let mut manager = Command::new("/usr/bin/qs")
        .args(["--no-duplicate", "-p"])
        .arg(config)
        .env("OMARCHY_WIDGET_ROLE", "manager")
        .env_remove("OMARCHY_WIDGET_BROKER")
        .stdin(Stdio::null())
        .spawn()
        .map_err(err)?;
    let mut runners: BTreeMap<String, Runner> = BTreeMap::new();
    let mut dead_editors: BTreeMap<String, String> = BTreeMap::new();
    let mut failures: BTreeMap<String, (u32, Instant)> = BTreeMap::new();
    let mut generations: BTreeMap<String, (PathBuf, u64)> = BTreeMap::new();
    let result = (|| {
        let mut next_scan = Instant::now();
        loop {
            if manager.try_wait().map_err(err)?.is_some() {
                return Err("Widget manager exited".into());
            }
            if Instant::now() >= next_scan {
                let snapshot = match r.snapshot() {
                    Ok(snapshot) => snapshot,
                    Err(e) => {
                        // Preserve current runners and keep the manager available
                        // for recovery even if the saved file cannot be parsed.
                        eprintln!("Widget layout unavailable: {e}");
                        let _ = atomic_json(
                            &r.state.join("runner-health.json"),
                            &json!({"updatedAt":reveal::now(),"error":e}),
                        );
                        next_scan = Instant::now() + Duration::from_secs(1);
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                };
                let reveal_until = if snapshot["runtime"]["revealing"] == true {
                    snapshot["runtime"]["revealUntil"].as_u64().unwrap_or(0)
                } else {
                    0
                };
                let edit = snapshot["runtime"]["edit"]["instance"]
                    .as_str()
                    .unwrap_or("");
                let mut wanted = BTreeMap::new();
                for entry in snapshot["installed"].as_array().ok_or("Invalid snapshot")? {
                    if (entry["placement"]["enabled"] == true || entry["instanceId"] == edit)
                        && snapshot["runtime"]["packageControls"]
                            [entry["packageId"].as_str().unwrap_or("")]["disabled"]
                            != true
                    {
                        wanted.insert(
                            entry["packageId"]
                                .as_str()
                                .ok_or("Missing package")?
                                .to_owned(),
                            PathBuf::from(entry["directory"].as_str().ok_or("Missing source")?),
                        );
                    }
                }
                for (id, source) in &wanted {
                    let generation = (
                        source.clone(),
                        snapshot["runtime"]["packageControls"][id]["serial"]
                            .as_u64()
                            .unwrap_or(0),
                    );
                    if generations.get(id) != Some(&generation) {
                        runners.remove(id);
                        failures.remove(id);
                        generations.insert(id.clone(), generation);
                    }
                }
                runners.retain(|id, runner| {
                    wanted.get(id) == Some(&runner.source) && runner.reveal_until == reveal_until
                });
                for (id, source) in wanted {
                    // Finish recovery before a replacement can open a new editor.
                    if runners.contains_key(&id) || dead_editors.contains_key(&id) {
                        continue;
                    }
                    if let Some((count, when)) = failures.get(&id) {
                        if *count >= 3 || when.elapsed() < Duration::from_secs(5) {
                            continue;
                        }
                    }
                    match launch(
                        config,
                        &base,
                        &id,
                        &source,
                        &upstream,
                        reveal_until,
                        generations[&id].1,
                    ) {
                        Ok(runner) => {
                            runners.insert(id, runner);
                        }
                        Err(e) => {
                            eprintln!("Widget {id}: {e}");
                            dead_editors.insert(
                                id.clone(),
                                "Clearing settings from the stopped runner".into(),
                            );
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
                let mut health = serde_json::Map::new();
                for entry in snapshot["catalog"].as_array().unwrap() {
                    let id = entry["packageId"].as_str().unwrap();
                    let count = failures.get(id).map_or(0, |v| v.0);
                    let state = if entry["packageDisabled"] == true {
                        "disabled"
                    } else if runners.contains_key(id) {
                        "running"
                    } else if dead_editors.contains_key(id) {
                        "recovery-blocked"
                    } else if count >= 3 {
                        "failed"
                    } else if count > 0 {
                        "retrying"
                    } else {
                        "idle"
                    };
                    health.insert(
                        id.into(),
                        runner_health(state, count, dead_editors.get(id).map(String::as_str)),
                    );
                }
                let _ = atomic_json(
                    &r.state.join("runner-health.json"),
                    &json!({"updatedAt":reveal::now(),"packages":health}),
                );
                next_scan = Instant::now() + Duration::from_secs(1);
            }
            let mut dead = Vec::new();
            for (id, runner) in &mut runners {
                if runner.child.try_wait().map_err(err)?.is_some() {
                    if runner.reveal_until == 0
                        || reveal::active(runner.reveal_until, reveal::now())
                    {
                        dead.push(id.clone());
                    } else {
                        next_scan = Instant::now();
                    }
                    continue;
                }
                serve(
                    &runner.listener,
                    &r,
                    id,
                    &runner.source,
                    runner.serial,
                    &mut runner.writes,
                )?;
            }
            for id in dead {
                dead_editors.insert(
                    id.clone(),
                    "Clearing settings from the stopped runner".into(),
                );
                runners.remove(&id);
                let count = failures.get(&id).map_or(1, |v| v.0 + 1);
                failures.insert(id.clone(), (count, Instant::now()));
                eprintln!("Widget {id} stopped; failure {count}/3 (restart this package in Widgets to retry)");
            }
            dead_editors.retain(|id, reason| match r.clear_dead_editor(Some(id)) {
                Ok(()) => false,
                Err(error) => {
                    *reason = error.chars().take(1000).collect();
                    true
                }
            });
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    drop(runners);
    let _ = manager.kill();
    let _ = manager.wait();
    let _ = fs::remove_dir_all(base);
    result
}

fn runner_health(state: &str, failures: u32, recovery_error: Option<&str>) -> Value {
    let message = recovery_error.map(|error| {
        format!("Restart paused until stale settings can be cleared: {error}. Resolve the registry error; if layout repair is required, run omarchy-widget repair. Recovery retries automatically.")
    });
    json!({"state":state,"failures":failures,"message":message})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocked_editor_recovery_explains_the_hold() {
        let error = "Invalid placement; layout preserved".to_string();
        let health = runner_health("recovery-blocked", 1, Some(&error));
        assert_eq!(health["state"], "recovery-blocked");
        let message = health["message"].as_str().unwrap();
        assert!(message.contains(&error));
        assert!(message.contains("omarchy-widget repair"));
        assert!(message.contains("automatically"));
        assert!(runner_health("running", 0, None)["message"].is_null());
    }

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
        let base = env::temp_dir().join(format!("island-test-{}", nonce()));
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
                0,
                &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
        };
        let snapshot = call(&["list"]).unwrap();
        assert_eq!(snapshot["installed"].as_array().unwrap().len(), 1);
        assert!(!snapshot.to_string().contains("io.test.b"));
        assert_eq!(snapshot["installed"][0]["directory"], "/widget");
        assert_eq!(snapshot["catalog"], json!([]));
        assert_eq!(snapshot["retained"], json!([]));
        for args in [
            vec!["save", "io.test.b", r#"{"revision":0,"settings":{}}"#],
            vec!["hide", "io.test.b"],
            vec!["workspace", "io.test.b", "2"],
            vec!["remove", "io.test.a"],
            vec!["remove-instance", "io.test.a"],
            vec!["uninstall", "io.test.a", "delete"],
            vec!["create", "io.test.a", "medium"],
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
        serve(
            &listener,
            &r,
            "io.test.a",
            &source,
            0,
            &mut WriteBudget::default(),
        )
        .unwrap();
        let response = request.join().unwrap().unwrap();
        assert_eq!(response["installed"].as_array().unwrap().len(), 1);
        assert!(!response.to_string().contains("io.test.b"));
        r.deploy(&base.join("io.test.a"), true).unwrap();
        assert!(call(&["list"]).is_err());
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn expired_runner_and_deleted_instance_cannot_mutate_under_lock() {
        let base = env::temp_dir().join(format!("broker-authority-{}", nonce()));
        fs::create_dir(&base).unwrap();
        let r = Registry {
            data: base.join("data"),
            state: base.join("state"),
        };
        let src = base.join("source");
        sdk::scaffold(&src, "io.example.authority", "Authority").unwrap();
        r.install(&src).unwrap();
        r.placement("io.example.authority", "add", None).unwrap();
        let source = r
            .source(&r.layout().unwrap(), "io.example.authority")
            .unwrap();
        let call = |args: &[&str]| {
            scoped(
                &r,
                "io.example.authority",
                &source,
                0,
                &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
        };
        let lock = r.lock().unwrap();
        assert!(call(&["hide", "io.example.authority"])
            .unwrap_err()
            .contains("busy"));
        drop(lock);
        call(&["edit", "io.example.authority"]).unwrap();
        r.package_control("io.example.authority", "restart")
            .unwrap();
        let before = r.layout().unwrap();
        assert!(call(&["hide", "io.example.authority"])
            .unwrap_err()
            .contains("expired"));
        assert_eq!(before, r.layout().unwrap());
        let serial = before["runtime"]["packageControls"]["io.example.authority"]["serial"]
            .as_u64()
            .unwrap();
        scoped(
            &r,
            "io.example.authority",
            &source,
            serial,
            &["content-failed".into(), "io.example.authority".into()],
        )
        .unwrap();
        assert_eq!(
            r.layout().unwrap()["runtime"]["packageControls"]["io.example.authority"]
                ["loadFailure"],
            true
        );
        assert!(scoped(
            &r,
            "io.example.authority",
            &source,
            serial,
            &["hide".into(), "io.example.authority".into()]
        )
        .is_err());
        r.remove_instance("io.example.authority").unwrap();
        assert!(call(&[
            "save",
            "io.example.authority",
            r#"{"revision":0,"settings":{}}"#
        ])
        .is_err());
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn reveal_only_adds_temporary_overlay_authority() {
        for request in ["get_layer_surface", "set_layer"] {
            let interface = if request == "set_layer" {
                "zwlr_layer_surface_v1"
            } else {
                "zwlr_layer_shell_v1"
            };
            let args = ["wayland-policy", interface, request].map(str::to_owned);
            assert!(policy(&args, r#"{"layer":3}"#, true).is_ok());
            assert!(policy(&args, r#"{"layer":3}"#, false).is_err());
            assert!(policy(&args, r#"{"layer":2}"#, true).is_err());
        }
        let args = [
            "wayland-policy",
            "zwlr_layer_surface_v1",
            "set_keyboard_interactivity",
        ]
        .map(str::to_owned);
        assert!(policy(&args, r#"{"keyboard_interactivity":1}"#, true).is_err());
    }
    #[test]
    fn rolling_write_budget_does_not_block_reads() {
        let mut budget = WriteBudget::default();
        let start = Instant::now();
        for _ in 0..5 {
            budget.admit(&["save".into()], start).unwrap();
        }
        assert!(budget
            .admit(&["save".into()], start + Duration::from_millis(999))
            .is_err());
        for _ in 0..100 {
            budget.admit(&["list".into()], start).unwrap();
        }
        budget
            .admit(&["edit-done".into()], start + Duration::from_secs(1))
            .unwrap();
        assert_eq!(budget.0.len(), 1);
    }
    #[test]
    fn refused_content_failure_can_retry_but_old_generation_cannot_stop_new_code() {
        let base = env::temp_dir().join(format!("content-delivery-{}", nonce()));
        fs::create_dir(&base).unwrap();
        let r = Registry {
            data: base.join("data"),
            state: base.join("state"),
        };
        let src = base.join("source");
        sdk::scaffold(&src, "io.review.recovery", "Recovery").unwrap();
        r.install(&src).unwrap();
        let id = r
            .placement("io.review.recovery", "create", Some("small"))
            .unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let source = r
            .source(&r.layout().unwrap(), "io.review.recovery")
            .unwrap();
        let request = vec!["content-failed".into(), id];
        let lock = r.lock().unwrap();
        assert!(scoped(&r, "io.review.recovery", &source, 0, &request)
            .unwrap_err()
            .starts_with("Registry busy;"));
        drop(lock);
        assert!(scoped(
            &r,
            "io.review.recovery",
            &source,
            0,
            &["control".into(), "arrange".into()]
        )
        .is_err());
        scoped(&r, "io.review.recovery", &source, 0, &request).unwrap();
        assert_eq!(
            r.layout().unwrap()["runtime"]["packageControls"]["io.review.recovery"]["loadFailure"],
            true
        );
        r.package_control("io.review.recovery", "enable").unwrap();
        r.deploy(&src, true).unwrap();
        let before = r.layout().unwrap();
        assert!(scoped(&r, "io.review.recovery", &source, 0, &request).is_err());
        assert_eq!(r.layout().unwrap(), before);
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn manager_writes_survive_repeated_broker_read_contention() {
        let base = env::temp_dir().join(format!("broker-contention-{}", nonce()));
        fs::create_dir(&base).unwrap();
        let r = std::sync::Arc::new(Registry {
            data: base.join("data"),
            state: base.join("state"),
        });
        let src = base.join("source");
        sdk::scaffold(&src, "io.review.contention", "Contention").unwrap();
        r.install(&src).unwrap();
        let id = r
            .placement("io.review.contention", "create", Some("small"))
            .unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let source = r
            .source(&r.layout().unwrap(), "io.review.contention")
            .unwrap();
        let reader = r.clone();
        let polling = std::thread::spawn(move || {
            for _ in 0..500 {
                let _ = scoped(
                    &reader,
                    "io.review.contention",
                    &source,
                    0,
                    &["list".into()],
                );
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        for n in 0..300 {
            retry_busy(|| r.placement(&id, if n % 2 == 0 { "hide" } else { "show" }, None))
                .unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
        polling.join().unwrap();
        fs::remove_dir_all(base).unwrap();
    }
}
