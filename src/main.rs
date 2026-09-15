use serde_json::{json, Value};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;
const MAX_PACKAGES: usize = 64;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id.bytes().next().is_some_and(|c| c.is_ascii_lowercase())
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'-')
        && !id.contains("..")
        && !id.ends_with('.')
}
fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && !Path::new(value).is_absolute()
}
fn read_json(path: &Path, limit: u64) -> Result<Value> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit {
        return Err("Not a bounded regular JSON file".into());
    }
    let mut data = Vec::new();
    File::open(path)
        .map_err(err)?
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(err)?;
    if data.len() as u64 > limit {
        return Err("JSON file exceeds limit".into());
    }
    serde_json::from_slice(&data).map_err(err)
}
fn manifest(dir: &Path) -> Result<Value> {
    let v = read_json(&dir.join("widget.json"), 16384)?;
    if v["schemaVersion"] != 1 || v["kind"] != "desktop-widget" || v["coreApi"] != 1 {
        return Err("Requires desktop-widget schema 1 / coreApi 1".into());
    }
    if !id_ok(v["id"].as_str().unwrap_or(""))
        || !v["name"]
            .as_str()
            .is_some_and(|x| !x.is_empty() && x.len() <= 100)
    {
        return Err("Invalid widget identity".into());
    }
    let version = v["version"].as_str().unwrap_or("");
    if version.split('.').count() != 3
        || !version
            .split('.')
            .all(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err("Version must be numeric major.minor.patch".into());
    }
    let entry = v["entryPoint"].as_str().unwrap_or("");
    if !safe_relative(entry) || !entry.ends_with(".qml") {
        return Err("Unsafe QML entry point".into());
    }
    let canonical = dir.canonicalize().map_err(err)?;
    let target = dir.join(entry).canonicalize().map_err(err)?;
    if !target.starts_with(&canonical) || !target.is_file() {
        return Err("Entry point escapes package".into());
    }
    let sizes = v["sizes"].as_object().ok_or("Missing sizes")?;
    if sizes.is_empty() || sizes.len() > 3 {
        return Err("Declare 1–3 supported sizes".into());
    }
    for (key, value) in sizes {
        if !["compact", "standard", "wide"].contains(&key.as_str())
            || !["width", "height"].iter().all(|k| {
                value[*k]
                    .as_u64()
                    .is_some_and(|n| (120..=1600).contains(&n))
            })
        {
            return Err("Invalid size declaration".into());
        }
    }
    if !sizes.contains_key(v["defaultSize"].as_str().unwrap_or("")) || !v["defaults"].is_object() {
        return Err("Invalid defaultSize or defaults".into());
    }
    Ok(v)
}
fn walk(dir: &Path, relative: &Path, out: &mut Vec<PathBuf>, bytes: &mut u64) -> Result<()> {
    for item in fs::read_dir(dir.join(relative)).map_err(err)? {
        let item = item.map_err(err)?;
        if item.file_name() == ".git" {
            continue;
        }
        let path = relative.join(item.file_name());
        let meta = fs::symlink_metadata(item.path()).map_err(err)?;
        if path.components().count() > 12
            || meta.file_type().is_symlink()
            || (!meta.is_file() && !meta.is_dir())
        {
            return Err("Package contains a symlink, special file or deep path".into());
        }
        if meta.is_dir() {
            *bytes += 32768;
            if *bytes > 8 * 1024 * 1024 {
                return Err("Package directory budget exceeded".into());
            }
            walk(dir, &path, out, bytes)?
        } else {
            *bytes += meta.len();
            out.push(path);
            if out.len() > 256 || *bytes > 8 * 1024 * 1024 {
                return Err("Package exceeds 256 files / 8 MiB".into());
            }
        }
    }
    Ok(())
}
fn validate(dir: &Path) -> Result<Value> {
    let meta = fs::symlink_metadata(dir).map_err(err)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("Package must be a regular directory".into());
    }
    let mut files = Vec::new();
    walk(dir, Path::new(""), &mut files, &mut 0)?;
    manifest(dir)
}
fn atomic_json(path: &Path, v: &Value) -> Result<()> {
    let parent = path.parent().ok_or("Missing parent")?;
    fs::create_dir_all(parent).map_err(err)?;
    let tmp = parent.join(format!(".write-{}", std::process::id()));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(err)?;
    let result = (|| {
        f.write_all(serde_json::to_string(v).map_err(err)?.as_bytes())
            .map_err(err)?;
        f.sync_all().map_err(err)?;
        fs::rename(&tmp, path).map_err(err)
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
struct Registry {
    data: PathBuf,
    state: PathBuf,
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}
impl Registry {
    fn from_env() -> Result<Self> {
        let home = env::var_os("HOME").ok_or("HOME unavailable")?;
        let home = PathBuf::from(home);
        let data = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".local/share"));
        let state = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or(home.join(".local/state"));
        if !data.is_absolute() || !state.is_absolute() {
            return Err("XDG roots must be absolute".into());
        }
        Ok(Self {
            data: data.join("omarchy/widgets"),
            state: state.join("omarchy/widgets"),
        })
    }
    fn lock(&self) -> Result<Lock> {
        fs::create_dir_all(&self.data).map_err(err)?;
        let p = self.data.join(".operation-lock");
        fs::create_dir(&p).map_err(|_|"Registry busy; a crashed operation may require removing .operation-lock after checking no manager is running".to_string())?;
        Ok(Lock(p))
    }
    fn package(&self, id: &str) -> Result<PathBuf> {
        if !id_ok(id) {
            return Err("Invalid widget ID".into());
        }
        Ok(self.data.join("packages").join(id))
    }
    fn layout(&self) -> Result<Value> {
        let path = self.state.join("layout.json");
        if !path.exists() {
            return Ok(json!({"version":1,"placements":{}}));
        }
        let v = read_json(&path, 65536)?;
        if v["version"] != 1 || !v["placements"].is_object() {
            return Err("Unsupported layout; file preserved".into());
        }
        for (id, p) in v["placements"].as_object().unwrap() {
            if !id_ok(id)
                || !p.is_object()
                || !p["enabled"].is_boolean()
                || !p["settings"].is_object()
                || !["x", "y"]
                    .iter()
                    .all(|k| p[*k].as_f64().is_some_and(|n| (0.0..=20000.0).contains(&n)))
                || !p["monitor"].is_string()
                || !p["size"].is_string()
            {
                return Err("Invalid placement; layout preserved".into());
            }
        }
        Ok(v)
    }
    fn snapshot(&self) -> Result<Value> {
        let layout = self.layout()?;
        let mut widgets = Vec::new();
        let mut problems = Vec::new();
        let theme_file = self
            .state
            .parent()
            .unwrap()
            .join("current/theme/widgets.json");
        let appearance = if theme_file.exists() {
            match read_json(&theme_file, 8192) {
                Ok(v) if v.is_object() => v,
                _ => {
                    problems.push("Invalid theme widgets.json; using defaults".to_string());
                    json!({})
                }
            }
        } else {
            json!({})
        };
        let packages = self.data.join("packages");
        if packages.exists() {
            for entry in fs::read_dir(packages).map_err(err)?.take(MAX_PACKAGES + 1) {
                let entry = entry.map_err(err)?;
                if widgets.len() + problems.len() >= MAX_PACKAGES {
                    return Err("Too many installed packages".into());
                }
                let path = entry.path();
                match validate(&path) {
                    Ok(m) => {
                        let id = m["id"].as_str().unwrap().to_owned();
                        if entry.file_name().to_str() != Some(id.as_str()) {
                            problems.push("Package directory identity mismatch".to_string());
                            continue;
                        }
                        widgets.push(json!({"manifest":m,"directory":path,"placement":layout["placements"][&id]}));
                    }
                    Err(_) => problems.push(format!(
                        "Invalid package: {}",
                        entry
                            .file_name()
                            .to_string_lossy()
                            .chars()
                            .take(100)
                            .collect::<String>()
                    )),
                }
            }
        }
        widgets.sort_by_key(|w| w["manifest"]["id"].as_str().unwrap_or("").to_string());
        Ok(json!({"api":1,"installed":widgets,"problems":problems,"appearance":appearance}))
    }
    fn install(&self, source: &Path) -> Result<Value> {
        let _lock = self.lock()?;
        let m = validate(source)?;
        let id = m["id"].as_str().unwrap();
        let dest = self.package(id)?;
        if dest.exists() {
            return Err("Already installed; remove the old package explicitly before installing a replacement".into());
        }
        let packages = dest.parent().unwrap();
        fs::create_dir_all(packages).map_err(err)?;
        if fs::read_dir(packages).map_err(err)?.count() >= MAX_PACKAGES {
            return Err("Package limit reached".into());
        }
        let stage = self.data.join(format!(".stage-{}", std::process::id()));
        fs::create_dir(&stage).map_err(err)?;
        let result = (|| {
            let mut files = Vec::new();
            walk(source, Path::new(""), &mut files, &mut 0)?;
            for p in files {
                let to = stage.join(&p);
                fs::create_dir_all(to.parent().unwrap()).map_err(err)?;
                fs::copy(source.join(&p), &to).map_err(err)?;
                fs::set_permissions(&to, fs::Permissions::from_mode(0o600)).map_err(err)?;
            }
            validate(&stage)?;
            fs::rename(&stage, &dest).map_err(err)?;
            Ok(json!({"installed":id,"added":false}))
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(stage);
        }
        result
    }
    fn placement(&self, id: &str, operation: &str, value: Option<&str>) -> Result<Value> {
        let _lock = self.lock()?;
        let m = manifest(&self.package(id)?)?;
        let mut l = self.layout()?;
        let p = &mut l["placements"][id];
        if p.is_null() {
            *p = json!({"enabled":false,"x":40,"y":80,"monitor":"","size":m["defaultSize"],"settings":m["defaults"]})
        }
        match operation {
            "add" => p["enabled"] = json!(true),
            "hide" => p["enabled"] = json!(false),
            "configure" => {
                let raw = value.ok_or("Missing settings")?;
                if raw.len() > 8192 {
                    return Err("Settings too large".into());
                }
                let v: Value = serde_json::from_str(raw).map_err(err)?;
                if !v.is_object() {
                    return Err("Settings must be an object".into());
                }
                p["settings"] = v;
            }
            "place" => {
                let raw = value.ok_or("Missing placement")?;
                if raw.len() > 1024 {
                    return Err("Placement too large".into());
                }
                let v: Value = serde_json::from_str(raw).map_err(err)?;
                if !v.is_object()
                    || !v["x"]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && (0.0..=20000.0).contains(&n))
                    || !v["y"]
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && (0.0..=20000.0).contains(&n))
                    || !v["monitor"].as_str().is_some_and(|s| s.len() <= 120)
                    || !m["sizes"]
                        .as_object()
                        .unwrap()
                        .contains_key(v["size"].as_str().unwrap_or(""))
                {
                    return Err("Invalid placement".into());
                }
                for k in ["x", "y", "monitor", "size"] {
                    p[k] = v[k].clone();
                }
            }
            _ => return Err("Unknown operation".into()),
        }
        if serde_json::to_vec(&l).map_err(err)?.len() > 65536 {
            return Err("Layout size limit reached".into());
        }
        atomic_json(&self.state.join("layout.json"), &l)?;
        Ok(json!({"updated":id}))
    }
    fn remove(&self, id: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let p = self.package(id)?;
        let metadata = fs::symlink_metadata(&p).map_err(err)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("Invalid installed package directory".into());
        }
        let mut l = self.layout()?;
        fs::remove_dir_all(p).map_err(err)?;
        l["placements"].as_object_mut().unwrap().remove(id);
        atomic_json(&self.state.join("layout.json"), &l)?;
        Ok(json!({"removed":id}))
    }
}
fn run(args: &[String]) -> Result<Value> {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    if cmd == "help" {
        return Ok(
            json!({"commands":["validate PATH","install PATH","list","add ID","hide ID","configure ID JSON","place ID JSON","remove ID"],"api":1}),
        );
    }
    let required = match cmd {
        "list" => 1,
        "validate" | "install" | "add" | "hide" | "remove" => 2,
        "place" | "configure" => 3,
        _ => return Err("Unknown command".into()),
    };
    if args.len() != required {
        return Err("Wrong argument count; use help".into());
    }
    if cmd == "validate" {
        return validate(Path::new(&args[1]));
    }
    let r = Registry::from_env()?;
    match cmd {
        "list" => r.snapshot(),
        "install" => r.install(Path::new(&args[1])),
        "remove" => r.remove(&args[1]),
        "add" | "hide" | "configure" | "place" => {
            r.placement(&args[1], cmd, args.get(2).map(String::as_str))
        }
        _ => Err("Unknown command".into()),
    }
}
fn main() {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(v) => println!("{v}"),
        Err(e) => {
            println!(
                "{}",
                json!({"error":e.chars().take(240).collect::<String>()})
            );
            std::process::exit(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = env::temp_dir().join(format!(
                "widget-core-test-{}-{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(p: &Path) {
        fs::create_dir_all(p).unwrap();
        fs::write(p.join("View.qml"), "import QtQuick\nItem {}\n").unwrap();
        fs::write(p.join("widget.json"),json!({"schemaVersion":1,"kind":"desktop-widget","coreApi":1,"id":"io.example.test","name":"Contract fixture","version":"0.1.0","entryPoint":"View.qml","defaultSize":"standard","sizes":{"standard":{"width":300,"height":200}},"defaults":{}}).to_string()).unwrap();
    }
    #[test]
    fn rejects_unsafe_paths() {
        for p in ["../x.qml", "/tmp/x.qml", "a/../../b", "", "a\\b.qml"] {
            assert!(!safe_relative(p));
        }
        assert!(safe_relative("qml/View.qml"));
        for id in ["../x", "-x", "a..b", "a/b"] {
            assert!(!id_ok(id))
        }
    }
    #[test]
    fn install_does_not_add() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        assert!(r.snapshot().unwrap()["installed"][0]["placement"].is_null());
        r.placement("io.example.test", "add", None).unwrap();
        assert_eq!(
            r.snapshot().unwrap()["installed"][0]["placement"]["enabled"],
            true
        );
        assert!(r.install(&src).is_err());
    }
    #[test]
    fn settings_and_hide_survive_reopen() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement(
            "io.example.test",
            "configure",
            Some("{\"cities\":[\"London\",\"Tokyo\"]}"),
        )
        .unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.placement("io.example.test", "hide", None).unwrap();
        let v = r.snapshot().unwrap();
        assert_eq!(
            v["installed"][0]["placement"]["settings"]["cities"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(v["installed"][0]["placement"]["enabled"], false);
    }
    #[test]
    fn symlinks_and_bad_contracts_rejected() {
        let t = Temp::new();
        fixture(&t.0);
        std::os::unix::fs::symlink("View.qml", t.0.join("Alias.qml")).unwrap();
        assert!(validate(&t.0).is_err());
        fs::remove_file(t.0.join("Alias.qml")).unwrap();
        let mut m = manifest(&t.0).unwrap();
        m["coreApi"] = json!(2);
        fs::write(t.0.join("widget.json"), m.to_string()).unwrap();
        assert!(validate(&t.0).is_err());
    }
    #[test]
    fn invalid_placement_does_not_write() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        assert!(r
            .placement("io.example.test", "place", Some("{\"x\":-1}"))
            .is_err());
        assert!(!r.state.join("layout.json").exists());
    }
    #[test]
    fn future_layout_preserved() {
        let t = Temp::new();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        fs::create_dir(&r.state).unwrap();
        let p = r.state.join("layout.json");
        fs::write(&p, "{\"version\":2,\"placements\":{}}").unwrap();
        assert!(r.layout().is_err());
        assert!(fs::read_to_string(p).unwrap().contains("2"));
    }
    #[test]
    fn concurrent_writer_refused() {
        let t = Temp::new();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        let l = r.lock().unwrap();
        assert!(r.lock().is_err());
        drop(l);
        assert!(r.lock().is_ok());
    }
    #[test]
    fn remove_clears_package_and_placement() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.remove("io.example.test").unwrap();
        assert!(r.snapshot().unwrap()["installed"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(r.layout().unwrap()["placements"]
            .get("io.example.test")
            .is_none());
    }
}
