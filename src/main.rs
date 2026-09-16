mod grid;
mod island;
mod resources;
mod reveal;
mod sdk;
mod settings;
mod weather;
mod workspaces;
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
    let mut v = read_json(&dir.join("widget.json"), 16384)?;
    if v["kind"] != "desktop-widget"
        || !((v["schemaVersion"] == 1 && v["coreApi"] == 1)
            || (v["schemaVersion"] == 2 && v["coreApi"] == 2))
    {
        return Err("Requires desktop-widget schema/coreApi 1 or 2".into());
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
    if !v["defaults"].is_object() || serde_json::to_vec(&v["defaults"]).map_err(err)?.len() > 8192 {
        return Err("Defaults must be an object up to 8 KiB".into());
    }
    // API 1 remains loadable, but geometry is always owned by Core.
    let families: Vec<String> = if v["coreApi"] == 1 {
        let sizes = v["sizes"].as_object().ok_or("Missing legacy sizes")?;
        if sizes.is_empty()
            || sizes.len() > 3
            || !sizes.contains_key(v["defaultSize"].as_str().unwrap_or(""))
        {
            return Err("Invalid legacy sizes".into());
        }
        sizes
            .keys()
            .map(|k| family(k).map(str::to_owned))
            .collect::<Result<_>>()?
    } else {
        let values = v["families"].as_array().ok_or("Missing families")?;
        if values.is_empty() || values.len() > 3 {
            return Err("Declare 1–3 families".into());
        }
        let mut result = Vec::new();
        for value in values {
            let name = value.as_str().ok_or("Invalid family")?;
            if !["small", "medium", "large"].contains(&name) || result.contains(&name.to_string()) {
                return Err("Families must be unique: small, medium, large".into());
            }
            result.push(name.to_string());
        }
        if !result.iter().any(|x| v["defaultFamily"] == x.as_str()) {
            return Err("Invalid defaultFamily".into());
        }
        result
    };
    let default = if v["coreApi"] == 1 {
        family(v["defaultSize"].as_str().unwrap_or(""))?
    } else {
        v["defaultFamily"].as_str().unwrap()
    }
    .to_string();
    v["families"] = json!(families);
    v["defaultFamily"] = json!(default);
    if let Some(editor) = v["settingsEntryPoint"].as_str() {
        if !safe_relative(editor)
            || !editor.ends_with(".qml")
            || !dir
                .join(editor)
                .canonicalize()
                .map_err(err)?
                .starts_with(&canonical)
        {
            return Err("Invalid settingsEntryPoint".into());
        }
    }
    if let Some(previews) = v.get("previews") {
        for (size, path) in previews
            .as_object()
            .ok_or("Previews must be a family-to-PNG object")?
        {
            if !families.contains(size) {
                return Err("Preview family is not supported".into());
            }
            let path = path.as_str().ok_or("Preview path must be a string")?;
            if !safe_relative(path) || !path.ends_with(".png") {
                return Err("Preview must be a relative PNG path".into());
            }
            let file = dir.join(path);
            let meta = fs::symlink_metadata(&file).map_err(err)?;
            if !meta.is_file()
                || meta.len() > 512 * 1024
                || !file.canonicalize().map_err(err)?.starts_with(&canonical)
            {
                return Err("Preview must be a bounded package PNG".into());
            }
            let mut header = [0; 24];
            File::open(file)
                .map_err(err)?
                .read_exact(&mut header)
                .map_err(err)?;
            if &header[..8] != b"\x89PNG\r\n\x1a\n"
                || &header[12..16] != b"IHDR"
                || ![16, 20].iter().all(|i| {
                    (1..=1024).contains(&u32::from_be_bytes(header[*i..*i + 4].try_into().unwrap()))
                })
            {
                return Err("Preview must be PNG with dimensions from 1 to 1024".into());
            }
        }
    }
    if let Some(caps) = v.get("capabilities") {
        let caps = caps.as_array().ok_or("Capabilities must be an array")?;
        if caps.len() > 1 || caps.iter().any(|c| c != "weather") {
            return Err("Unsupported capability; only weather is available".into());
        }
    }
    if let Some(refresh) = v.get("refresh") {
        if !["none", "minute", "weather"].iter().any(|x| refresh == x) {
            return Err("Refresh must be none, minute or weather".into());
        }
        if refresh == "weather" && !weather::declared(&v) {
            return Err("Weather refresh requires weather capability".into());
        }
    }
    settings::contract(&v)?;
    sdk::dependencies(&v)?;
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
fn family(name: &str) -> Result<&str> {
    match name {
        "small" | "compact" => Ok("small"),
        "medium" | "standard" => Ok("medium"),
        "large" | "wide" => Ok("large"),
        _ => Err("Unknown widget family".into()),
    }
}
fn nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
fn atomic_json(path: &Path, v: &Value) -> Result<()> {
    let parent = path.parent().ok_or("Missing parent")?;
    fs::create_dir_all(parent).map_err(err)?;
    let tmp = parent.join(format!(".write-{}", nonce()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(err)?;
        f.write_all(serde_json::to_string(v).map_err(err)?.as_bytes())
            .map_err(err)?;
        f.sync_all().map_err(err)?;
        fs::rename(&tmp, path).map_err(err)?;
        File::open(parent).map_err(err)?.sync_all().map_err(err)
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
impl Registry {
    fn from_env() -> Result<Self> {
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME unavailable")?);
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
    fn lock(&self) -> Result<File> {
        fs::create_dir_all(&self.state).map_err(err)?;
        // File locks are released by the kernel, including after SIGKILL.
        // Never unlink this inode: another process may already have it open.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(self.state.join("registry.lock"))
            .map_err(err)?;
        file.try_lock()
            .map_err(|_| "Registry busy; retry after the current operation finishes".to_string())?;
        Ok(file)
    }
    fn package(&self, id: &str) -> Result<PathBuf> {
        if !id_ok(id) {
            return Err("Invalid widget ID".into());
        }
        Ok(self.data.join("packages").join(id))
    }
    fn source(&self, l: &Value, id: &str) -> Result<PathBuf> {
        let legacy = self.package(id)?;
        let record = &l["packages"][id];
        if record.is_null() {
            return Ok(legacy);
        }
        let relative = record["current"]
            .as_str()
            .ok_or("Package is not installed")?;
        if !safe_relative(relative)
            || !(relative.starts_with(&format!("versions/{id}/"))
                || relative == format!("packages/{id}"))
        {
            return Err("Invalid package source; state preserved".into());
        }
        Ok(self.data.join(relative))
    }
    fn layout(&self) -> Result<Value> {
        let path = self.state.join("layout.json");
        if !path.exists() {
            return Ok(json!({"version":2,"revision":0,"placements":{},"packages":{}}));
        }
        let mut v = read_json(&path, 1024 * 1024)?;
        if ![json!(1), json!(2)].contains(&v["version"]) || !v["placements"].is_object() {
            return Err("Unsupported layout; file preserved".into());
        }
        let legacy = v["version"] == 1;
        if legacy {
            v["packages"] = json!({});
            v["revision"] = json!(0);
            v["version"] = json!(2);
        }
        if !v["packages"].is_object()
            || v["revision"].as_u64().is_none()
            || v["placements"].as_object().unwrap().len() > 128
        {
            return Err("Invalid registry metadata; file preserved".into());
        }
        for (id, p) in v["placements"].as_object_mut().unwrap() {
            if !id_ok(id)
                || !p.is_object()
                || !p["enabled"].is_boolean()
                || !p["settings"].is_object()
                || !["x", "y"]
                    .iter()
                    .all(|k| p[*k].as_f64().is_some_and(|n| (0.0..=20000.0).contains(&n)))
                || !p["monitor"].as_str().is_some_and(|s| s.len() <= 120)
                || !p["size"].is_string()
                || !workspaces::valid(&p["workspace"])
                || !grid::valid_preference(p)
            {
                return Err("Invalid placement; layout preserved".into());
            }
            p["size"] = json!(family(p["size"].as_str().unwrap())?);
            if legacy {
                p["packageId"] = json!(id);
                p["definitionId"] = json!("main");
                p["revision"] = json!(0);
            }
            if !id_ok(p["packageId"].as_str().unwrap_or(""))
                || p["definitionId"] != "main"
                || p["revision"].as_u64().is_none()
            {
                return Err("Invalid instance identity or revision".into());
            }
        }
        Ok(v)
    }
    fn commit(&self, l: &mut Value) -> Result<()> {
        let path = self.state.join("layout.json");
        if path.exists()
            && read_json(&path, 1024 * 1024)?["version"] == 1
            && !self.state.join("layout-v1.backup.json").exists()
        {
            let old = read_json(&path, 1024 * 1024)?;
            atomic_json(&self.state.join("layout-v1.backup.json"), &old)?;
        }
        l["revision"] = json!(l["revision"]
            .as_u64()
            .ok_or("Invalid revision")?
            .checked_add(1)
            .ok_or("Revision exhausted")?);
        if serde_json::to_vec(l).map_err(err)?.len() > 1024 * 1024 {
            return Err("Registry exceeds 1 MiB".into());
        }
        atomic_json(&path, l)
    }
    fn ids(&self, l: &Value) -> Result<std::collections::BTreeSet<String>> {
        let mut ids = std::collections::BTreeSet::new();
        let legacy = self.data.join("packages");
        if legacy.exists() {
            for entry in fs::read_dir(legacy).map_err(err)? {
                let entry = entry.map_err(err)?;
                ids.insert(entry.file_name().to_string_lossy().to_string());
                if ids.len() > MAX_PACKAGES {
                    return Err("Package limit exceeded".into());
                }
            }
        }
        ids.extend(l["packages"].as_object().unwrap().keys().cloned());
        ids.retain(|id| l["packages"][id].is_null() || !l["packages"][id]["current"].is_null());
        if ids.len() > MAX_PACKAGES {
            return Err("Package limit exceeded".into());
        }
        Ok(ids)
    }
    fn palette(&self) -> Value {
        let path = self
            .state
            .parent()
            .unwrap()
            .join("current/theme/colors.toml");
        let mut palette = json!({});
        let Ok(file) = File::open(&path) else {
            return palette;
        };
        let mut text = String::new();
        if file.take(16385).read_to_string(&mut text).is_err() || text.len() > 16384 {
            return palette;
        }
        // Consume only the documented flat hex palette, not arbitrary TOML.
        for line in text.lines() {
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if !["foreground", "background", "accent", "red"].contains(&key) {
                continue;
            }
            let raw = raw.trim();
            let Some(quote) = raw.chars().next().filter(|c| *c == '\"' || *c == '\'') else {
                continue;
            };
            let Some(end) = raw[1..].find(quote) else {
                continue;
            };
            let value = &raw[1..end + 1];
            if value.starts_with('#')
                && [4, 7, 9].contains(&value.len())
                && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                palette[key] = json!(value);
            }
        }
        palette
    }
    fn request_edit(&self, id: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        if !l["placements"][id].is_object() {
            return Err("Add the widget before editing it".into());
        }
        if reveal::active(
            l["runtime"]["revealUntil"].as_u64().unwrap_or(0),
            reveal::now(),
        ) {
            return Err("Dismiss reveal before opening settings".into());
        }
        if !l["runtime"]["edit"].is_null() {
            if l["runtime"]["edit"]["instance"] == id {
                return Ok(json!(true));
            }
            return Err("Save or cancel the current settings window first".into());
        }
        l["runtime"]["edit"] = json!({"instance":id,"serial":l["revision"].as_u64().unwrap().checked_add(1).ok_or("Revision exhausted")?});
        self.commit(&mut l)?;
        Ok(json!(true))
    }
    fn finish_edit(&self, id: &str, serial: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        if l["runtime"]["edit"]["instance"] == id
            && l["runtime"]["edit"]["serial"]
                .as_u64()
                .map(|n| n.to_string())
                .as_deref()
                == Some(serial)
        {
            l["runtime"]["edit"] = Value::Null;
            self.commit(&mut l)?;
        }
        Ok(json!(true))
    }
    fn ensure_no_editor(&self, l: &Value, id: &str) -> Result<()> {
        let editor = l["runtime"]["edit"]["instance"].as_str().unwrap_or("");
        if l["placements"][editor]["packageId"] == id {
            return Err("Save or cancel this package's settings before changing its code".into());
        }
        Ok(())
    }
    fn package_control(&self, id: &str, action: &str) -> Result<Value> {
        if !["restart", "disable", "enable"].contains(&action) {
            return Err("Use restart, disable or enable".into());
        }
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        self.source(&l, id)?;
        let serial = l["revision"]
            .as_u64()
            .unwrap()
            .checked_add(1)
            .ok_or("Revision exhausted")?;
        let editor = l["runtime"]["edit"]["instance"].as_str().unwrap_or("");
        if l["placements"][editor]["packageId"] == id {
            l["runtime"]["edit"] = Value::Null;
        }
        l["runtime"]["packageControls"][id] = json!({"disabled":action=="disable","serial":serial});
        self.commit(&mut l)?;
        Ok(json!(true))
    }
    fn control(&self, method: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        let mut runtime = l["runtime"].clone();
        if !runtime.is_object() {
            runtime = json!({"shown":true,"editing":false,"managerOpen":false});
        }
        match method {
            "reveal" => {
                if !runtime["edit"].is_null() {
                    return Err("Finish settings before revealing widgets".into());
                }
                let active =
                    reveal::active(runtime["revealUntil"].as_u64().unwrap_or(0), reveal::now());
                runtime["revealUntil"] = json!(if active {
                    0
                } else {
                    reveal::now() + reveal::MAX_MS
                });
                runtime["editing"] = json!(false);
                runtime["managerOpen"] = json!(false);
            }
            "dismiss-reveal" => runtime["revealUntil"] = json!(0),
            "manage" => {
                runtime["revealUntil"] = json!(0);
                runtime["managerOpen"] = json!(!runtime["managerOpen"].as_bool().unwrap_or(false))
            }
            "close-manager" => runtime["managerOpen"] = json!(false),
            "arrange" => {
                runtime["revealUntil"] = json!(0);
                runtime["editing"] = json!(!runtime["editing"].as_bool().unwrap_or(false));
                runtime["shown"] = json!(true);
            }
            "finish-arrange" => runtime["editing"] = json!(false),
            "show" => runtime["shown"] = json!(true),
            "hide-all" => {
                runtime["revealUntil"] = json!(0);
                runtime["shown"] = json!(false);
                runtime["editing"] = json!(false);
            }
            "refresh" => (),
            _ => return Err("Unknown host control".into()),
        }
        l["runtime"] = runtime.clone();
        self.commit(&mut l)?;
        Ok(json!({"runtime":runtime,"revision":l["revision"]}))
    }
    fn snapshot(&self) -> Result<Value> {
        let mut l = self.layout()?;
        let active = reveal::active(
            l["runtime"]["revealUntil"].as_u64().unwrap_or(0),
            reveal::now(),
        );
        l["runtime"]["revealing"] = json!(active);
        let desktop = workspaces::snapshot();
        if grid::migrate(&mut l["placements"], &desktop) {
            // Polling must not terminate the supervisor when an external writer holds the lock.
            if let Ok(_lock) = self.lock() {
                let mut fresh = self.layout()?;
                if grid::migrate(&mut fresh["placements"], &desktop) {
                    self.commit(&mut fresh)?;
                }
                l = fresh;
            }
        }
        let effective = grid::resolve(&l["placements"], &desktop);
        let mut widgets = Vec::new();
        let mut catalog = Vec::new();
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
        let health =
            read_json(&self.state.join("runner-health.json"), 65536).unwrap_or(Value::Null);
        let health_fresh =
            reveal::now().saturating_sub(health["updatedAt"].as_u64().unwrap_or(0)) < 5000;
        for id in self.ids(&l)? {
            let validated = self
                .source(&l, &id)
                .and_then(|path| manifest(&path).map(|m| (path, m)));
            match validated {
                Ok((path, m)) if m["id"] == id => {
                    let count = l["placements"]
                        .as_object()
                        .unwrap()
                        .values()
                        .filter(|p| p["packageId"] == id)
                        .count();
                    catalog.push(
                        json!({"packageId":id,"manifest":m,"directory":path,"instanceCount":count,"weatherAllowed":weather::authorised(self,&l,&id),"packageDisabled":l["runtime"]["packageControls"][&id]["disabled"]==true,"health":if health_fresh {health["packages"][&id].clone()} else {Value::Null}}),
                    );
                    let mut found = false;
                    for (instance, p) in l["placements"].as_object().unwrap() {
                        if p["packageId"] != id {
                            continue;
                        }
                        found = true;
                        widgets.push(json!({"instanceId":instance,"packageId":id,"definitionId":"main","manifest":m,"directory":path,"placement":p,"effective":effective[instance],"occupancyIndex":effective.as_object().unwrap().keys().position(|id| id==instance)}));
                    }
                    if !found {
                        widgets.push(json!({"instanceId":id,"packageId":id,"definitionId":"main","manifest":m,"directory":path,"placement":null}));
                    }
                }
                _ => {
                    let problem = format!(
                        "Invalid package: {}",
                        id.chars().take(100).collect::<String>()
                    );
                    problems.push(problem.clone());
                    catalog.push(json!({"packageId":id,"manifest":{"id":id,"name":id,"version":"unavailable","families":[]},"instanceCount":l["placements"].as_object().unwrap().values().filter(|p|p["packageId"]==id).count(),"problem":problem}));
                }
            }
        }
        let installed_ids = self.ids(&l)?;
        let retained: Vec<Value> = l["placements"].as_object().unwrap().iter()
            .filter(|(_, p)| !installed_ids.contains(p["packageId"].as_str().unwrap()))
            .map(|(id,p)| json!({"instanceId":id,"packageId":p["packageId"],"name":l["packages"][p["packageId"].as_str().unwrap()]["name"],"placement":p}))
            .collect();
        Ok(
            json!({"api":2,"revision":l["revision"],"installed":widgets,"catalog":catalog,"retained":retained,"problems":problems,"appearance":appearance,"palette":self.palette(),"runtime":l["runtime"],"desktop":desktop,"occupancy":effective.as_object().unwrap().values().cloned().collect::<Vec<_>>()}),
        )
    }
    fn install(&self, source: &Path) -> Result<Value> {
        self.deploy(source, false)
    }
    fn deploy(&self, source: &Path, update: bool) -> Result<Value> {
        let _lock = self.lock()?;
        let m = validate(source)?;
        let id = m["id"].as_str().unwrap();
        let mut l = self.layout()?;
        self.ensure_no_editor(&l, id)?;
        let old = self.source(&l, id).ok().filter(|p| p.exists());
        if l["placements"].as_object().unwrap().values().any(|p| {
            p["packageId"] == id && !m["families"].as_array().unwrap().contains(&p["size"])
        }) {
            return Err("Package does not support a saved instance size; remove that instance or install a compatible version".into());
        }
        if old.is_some() && !update {
            return Err("Already installed; use update PATH (settings are preserved)".into());
        }
        if old.is_none() && update {
            return Err("Not installed; use install PATH".into());
        }
        if old.is_none() && self.ids(&l)?.len() >= MAX_PACKAGES {
            return Err("Package limit reached".into());
        }
        let before = settings::checkpoint(&l["placements"], id);
        let old_version = old
            .as_ref()
            .and_then(|p| manifest(p).ok())
            .map(|m| settings::version(&m))
            .unwrap_or(1);
        for p in l["placements"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .filter(|p| p["packageId"] == id)
        {
            let from = p["settingsVersion"].as_u64().unwrap_or(old_version);
            let migrated = settings::migrate(&m, from, &p["settings"])?;
            if migrated != p["settings"] || from != settings::version(&m) {
                p["revision"] = json!(p["revision"]
                    .as_u64()
                    .unwrap()
                    .checked_add(1)
                    .ok_or("Revision exhausted")?);
            }
            p["settings"] = migrated;
            p["settingsVersion"] = json!(settings::version(&m));
        }
        let after = settings::checkpoint(&l["placements"], id);
        let relative = format!("versions/{id}/{}", nonce());
        let stage = self.data.join(&relative);
        fs::create_dir_all(&stage).map_err(err)?;
        let result = (|| {
            let mut files = Vec::new();
            walk(source, Path::new(""), &mut files, &mut 0)?;
            for p in files {
                let to = stage.join(&p);
                fs::create_dir_all(to.parent().unwrap()).map_err(err)?;
                fs::copy(source.join(&p), &to).map_err(err)?;
                fs::set_permissions(&to, fs::Permissions::from_mode(0o600)).map_err(err)?;
                File::open(&to).map_err(err)?.sync_all().map_err(err)?;
            }
            if validate(&stage)? != m {
                return Err("Package changed during staging; retry the update".into());
            }
            sync_tree(&stage)?;
            File::open(stage.parent().unwrap())
                .map_err(err)?
                .sync_all()
                .map_err(err)?;
            File::open(self.data.join("versions"))
                .map_err(err)?
                .sync_all()
                .map_err(err)?;
            File::open(&self.data)
                .map_err(err)?
                .sync_all()
                .map_err(err)?;
            let previous = old
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.data).ok())
                .map(|p| p.to_string_lossy().to_string());
            l["packages"][id] = json!({"current":relative,"previous":previous,"checkpoint":{"before":before,"after":after}});
            self.commit(&mut l)?;
            Ok(
                json!({"installed":id,"updated":update,"revision":l["revision"],"restartRequired":true}),
            )
        })();
        // An unreferenced version is harmless after a crash. Do not delete here:
        // a directory-fsync error can occur after the pointer commit succeeds.
        result
    }
    fn rollback(&self, id: &str) -> Result<Value> {
        let _lock = self.lock()?;
        self.package(id)?;
        let mut l = self.layout()?;
        self.ensure_no_editor(&l, id)?;
        let previous = l["packages"][id]["previous"].clone();
        if !previous.is_string() {
            return Err("No previous package version".into());
        }
        let current = l["packages"][id]["current"].clone();
        l["packages"][id]["current"] = previous;
        let target = validate(&self.source(&l, id)?)?;
        if target["id"] != id {
            return Err("Rollback identity mismatch".into());
        }
        let checkpoint = l["packages"][id]["checkpoint"].clone();
        let before = settings::checkpoint(&l["placements"], id);
        for (instance, p) in l["placements"]
            .as_object_mut()
            .unwrap()
            .iter_mut()
            .filter(|(_, p)| p["packageId"] == id)
        {
            if !target["families"].as_array().unwrap().contains(&p["size"]) {
                return Err("Rollback does not support an existing instance size".into());
            }
            if p["settingsVersion"].as_u64().unwrap_or(1) != settings::version(&target) {
                if before[instance] != checkpoint["after"][instance]
                    || !checkpoint["before"][instance].is_object()
                {
                    return Err("Settings changed after update; rollback would overwrite them. Export settings and install a compatible package.".into());
                }
                p["settings"] = checkpoint["before"][instance]["settings"].clone();
                p["settingsVersion"] = checkpoint["before"][instance]["settingsVersion"].clone();
                p["revision"] = json!(p["revision"]
                    .as_u64()
                    .unwrap()
                    .checked_add(1)
                    .ok_or("Revision exhausted")?);
            }
            settings::validate(&target, &p["settings"])?;
        }
        l["packages"][id]["previous"] = current;
        l["packages"][id]["weatherGrant"] = Value::Null;
        l["packages"][id]["checkpoint"] =
            json!({"before":before,"after":settings::checkpoint(&l["placements"],id)});
        self.commit(&mut l)?;
        Ok(json!({"rolledBack":id,"revision":l["revision"]}))
    }
    fn placement(&self, id: &str, operation: &str, value: Option<&str>) -> Result<Value> {
        self.placement_using(id, operation, value, workspaces::snapshot)
    }
    fn placement_using(
        &self,
        id: &str,
        operation: &str,
        value: Option<&str>,
        desktop: impl FnOnce() -> Value,
    ) -> Result<Value> {
        let _lock = self.lock()?;
        if !id_ok(id) {
            return Err("Invalid instance ID".into());
        }
        let mut l = self.layout()?;
        let desktop = desktop();
        if l["placements"][id].is_null() && operation == "save" {
            return Err("Unknown widget instance; add it before changing it".into());
        }
        grid::migrate(&mut l["placements"], &desktop);
        let current = grid::resolve(&l["placements"], &desktop);
        let package_id = l["placements"][id]["packageId"]
            .as_str()
            .unwrap_or(id)
            .to_owned();
        let m = manifest(&self.source(&l, &package_id)?)?;
        let instance = if operation == "create" {
            if package_id != id {
                return Err("Create requires a package ID".into());
            }
            format!("instance-{}", nonce())
        } else if operation == "duplicate" {
            if l["placements"][id].is_null() {
                return Err("Add the widget before duplicating".into());
            }
            format!("instance-{}", nonce())
        } else {
            id.to_string()
        };
        if l["placements"][&instance].is_null() && l["placements"].as_object().unwrap().len() >= 128
        {
            return Err("Instance limit reached".into());
        }
        if operation == "duplicate" {
            l["placements"][&instance] = l["placements"][id].clone();
        }
        let activation_order = l["revision"].as_u64().unwrap_or(0) + 1;
        let p = &mut l["placements"][&instance];
        if p.is_null() {
            *p = json!({"packageId":package_id,"definitionId":"main","revision":0,"enabled":false,"x":0,"y":0,"monitor":"","cell":{"column":0,"row":0},"size":m["defaultFamily"],"settings":m["defaults"]});
        }
        match operation {
            "create" => {
                let size = value.ok_or("Choose a widget family")?;
                if !m["families"].as_array().unwrap().contains(&json!(size)) {
                    return Err("Unsupported widget family".into());
                }
                p["size"] = json!(size);
                p["enabled"] = json!(true);
                p["activationOrder"] = json!(activation_order);
            }
            "add" | "duplicate" => {
                if p["enabled"] != true || operation == "duplicate" {
                    p["activationOrder"] = json!(activation_order);
                }
                p["enabled"] = json!(true);
            }
            "hide" => p["enabled"] = json!(false),
            "workspace" => {
                let raw = value.ok_or("Missing workspace: use all or 1–9999")?;
                let assignment = if raw == "all" {
                    Value::Null
                } else {
                    json!(raw
                        .parse::<u64>()
                        .map_err(|_| "Workspace must be all or 1–9999")?)
                };
                if !workspaces::valid(&assignment) {
                    return Err("Workspace must be all or 1–9999".into());
                }
                p["workspace"] = assignment;
            }
            "configure" | "save" => {
                let raw = value.ok_or("Missing settings")?;
                if raw.len() > 16384 {
                    return Err("Settings request too large".into());
                }
                let v: Value = serde_json::from_str(raw).map_err(err)?;
                let settings = if operation == "save" {
                    if v["revision"].as_u64() != p["revision"].as_u64() {
                        return Err("Settings changed elsewhere. Reload before saving; your draft has been retained.".into());
                    }
                    v["settings"].clone()
                } else {
                    v
                };
                if !settings.is_object() || serde_json::to_vec(&settings).map_err(err)?.len() > 8192
                {
                    return Err("Settings must be an object up to 8 KiB".into());
                }
                settings::validate(&m, &settings)?;
                p["settingsVersion"] = json!(settings::version(&m));
                p["settings"] = settings;
                p["revision"] = json!(p["revision"]
                    .as_u64()
                    .unwrap()
                    .checked_add(1)
                    .ok_or("Revision exhausted")?);
            }
            "place" => {
                let raw = value.ok_or("Missing placement")?;
                if raw.len() > 1024 {
                    return Err("Placement too large".into());
                }
                let v: Value = serde_json::from_str(raw).map_err(err)?;
                if !["column", "row"]
                    .iter()
                    .all(|k| v[*k].as_u64().is_some_and(|n| n <= 10000))
                    || !v["monitor"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 120)
                {
                    return Err("Placement requires column, row, monitor and size".into());
                }
                let size = family(v["size"].as_str().unwrap_or(""))?;
                if !m["families"].as_array().unwrap().contains(&json!(size)) {
                    return Err("Unsupported widget family".into());
                }
                p["cell"] = json!({"column":v["column"],"row":v["row"]});
                p["monitor"] = v["monitor"].clone();
                p["size"] = json!(size);
            }
            _ => return Err("Unknown operation".into()),
        }
        settings::validate(&m, &p["settings"])?;
        p["settingsVersion"] = json!(settings::version(&m));
        if operation == "place" {
            grid::validate_change(&instance, p, &current, &desktop)?;
        } else if operation == "workspace" && p["enabled"] == true {
            // Changing workspace must not silently displace any existing occupant.
            if let Some(c) = current.get(&instance) {
                let mut target = p.clone();
                target["monitor"] = c["monitor"].clone();
                target["cell"] = json!({"column":c["column"],"row":c["row"]});
                grid::validate_change(&instance, &target, &current, &desktop)?;
            }
        }
        if operation == "create"
            || operation == "duplicate"
            || (operation == "add" && p["monitor"] == "")
        {
            // New instances reserve a free preference; existing hidden instances keep theirs.
            let resolved = grid::resolve(&l["placements"], &desktop);
            if let Some(c) = resolved.get(&instance) {
                l["placements"][&instance]["monitor"] = c["monitor"].clone();
                l["placements"][&instance]["cell"] = json!({"column":c["column"],"row":c["row"]});
            }
        }
        let placement = l["placements"][&instance].clone();
        self.commit(&mut l)?;
        Ok(json!({"updated":instance,"placement":placement,"revision":l["revision"]}))
    }
    fn remove(&self, id: &str) -> Result<Value> {
        self.uninstall(id, "delete")
    }
    fn remove_instance(&self, id: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        if l["placements"]
            .as_object_mut()
            .unwrap()
            .remove(id)
            .is_none()
        {
            return Err("Unknown widget instance".into());
        }
        if l["runtime"]["edit"]["instance"] == id {
            l["runtime"]["edit"] = Value::Null;
        }
        self.commit(&mut l)?;
        Ok(json!({"removedInstance":id,"revision":l["revision"]}))
    }
    fn uninstall(&self, id: &str, policy: &str) -> Result<Value> {
        if !["keep", "delete"].contains(&policy) {
            return Err("Choose uninstall PACKAGE_ID keep|delete".into());
        }
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        self.package(id)?;
        if !self.ids(&l)?.contains(id) {
            return Err("Package is not installed".into());
        }
        let name = self
            .source(&l, id)
            .and_then(|p| manifest(&p))
            .ok()
            .map(|m| m["name"].clone())
            .unwrap_or(json!(id));
        let edit = l["runtime"]["edit"]["instance"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        if l["placements"][&edit]["packageId"] == id {
            l["runtime"]["edit"] = Value::Null;
        }
        l["packages"][id] = json!({"current":null,"previous":null,"name":name});
        if policy == "delete" {
            l["placements"]
                .as_object_mut()
                .unwrap()
                .retain(|_, p| p["packageId"] != id);
        } else {
            for p in l["placements"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .filter(|p| p["packageId"] == id)
            {
                p["enabled"] = json!(false);
            }
        }
        self.commit(&mut l)?;
        Ok(json!({"removed":id,"savedInstances":policy,"revision":l["revision"]}))
    }
}
fn sync_tree(dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(err)? {
        let entry = entry.map_err(err)?;
        if entry.file_type().map_err(err)?.is_dir() {
            sync_tree(&entry.path())?;
        }
    }
    File::open(dir).map_err(err)?.sync_all().map_err(err)
}
fn run(args: &[String]) -> Result<Value> {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    if let Ok(socket) = env::var("OMARCHY_WIDGET_BROKER") {
        return island::client(&socket, args);
    }
    if cmd == "wayland-policy" {
        return island::wayland_policy(args, &env::var("WL_MITM_MSG_JSON").map_err(err)?);
    }
    if cmd == "resource-plan" && args.len() == 2 {
        return Ok(json!(resources::service_args(&args[1])?));
    }
    if cmd == "resource-preflight" && args.len() == 1 {
        return resources::preflight();
    }
    if cmd == "resource-check" && args.len() == 1 {
        return resources::verify();
    }
    if cmd == "island-worker" && args.len() == 4 {
        return resources::worker(
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
        );
    }
    if cmd == "supervise" && args.len() == 2 {
        return island::supervise(Path::new(&args[1]));
    }
    if cmd == "edit" && args.len() == 2 {
        return Registry::from_env()?.request_edit(&args[1]);
    }
    if cmd == "edit-done" && args.len() == 3 {
        return Registry::from_env()?.finish_edit(&args[1], &args[2]);
    }

    if cmd == "package-control" && args.len() == 3 {
        let r = Registry::from_env()?;
        return r.package_control(&args[1], &args[2]);
    }
    if cmd == "weather-permission" && args.len() == 3 {
        return weather::grant(&Registry::from_env()?, &args[1], &args[2]);
    }
    if cmd == "new" && args.len() == 4 {
        return sdk::scaffold(Path::new(&args[1]), &args[2], &args[3]);
    }
    if cmd == "help" {
        return Ok(
            json!({"commands":["new PATH ID NAME","weather-permission PACKAGE allow|deny","package-control PACKAGE restart|disable|enable","validate PATH","install PATH","update PATH","rollback PACKAGE_ID","list","control METHOD","add ID","create PACKAGE_ID FAMILY","duplicate INSTANCE_ID","hide INSTANCE_ID","remove-instance INSTANCE_ID","uninstall PACKAGE_ID keep|delete","save INSTANCE_ID {revision,settings}","configure INSTANCE_ID JSON (legacy)","place INSTANCE_ID JSON","workspace INSTANCE_ID all|NUMBER","remove PACKAGE_ID (legacy: deletes package and settings)"],"api":2,"version":"0.0.2"}),
        );
    }
    let required = match cmd {
        "list" => 1,
        "validate" | "install" | "update" | "rollback" | "add" | "duplicate" | "hide"
        | "remove" | "remove-instance" | "control" => 2,
        "place" | "configure" | "save" | "workspace" | "create" | "uninstall" => 3,
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
        "control" => r.control(&args[1]),
        "install" => r.install(Path::new(&args[1])),
        "update" => r.deploy(Path::new(&args[1]), true),
        "rollback" => r.rollback(&args[1]),
        "remove" => r.remove(&args[1]),
        "remove-instance" => r.remove_instance(&args[1]),
        "uninstall" => r.uninstall(&args[1], &args[2]),
        _ => r.placement(&args[1], cmd, args.get(2).map(String::as_str)),
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
    fn manager_create_uses_defaults_and_duplicate_copies_only_its_source() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        let id = r
            .placement("io.example.test", "create", Some("medium"))
            .unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        r.placement(&id, "configure", Some(r#"{"city":"London"}"#))
            .unwrap();
        r.placement(&id, "workspace", Some("3")).unwrap();
        let duplicate = r.placement(&id, "duplicate", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let fresh = r
            .placement("io.example.test", "create", Some("medium"))
            .unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let l = r.layout().unwrap();
        assert_ne!(id, duplicate);
        assert_ne!(id, fresh);
        assert_ne!(duplicate, fresh);
        assert_eq!(l["placements"][&duplicate]["settings"]["city"], "London");
        assert_eq!(l["placements"][&duplicate]["workspace"], 3);
        assert_eq!(l["placements"][&fresh]["settings"], json!({}));
        assert!(l["placements"][&fresh]["workspace"].is_null());
        assert!(r
            .placement("io.example.test", "create", Some("large"))
            .is_err());
        assert_eq!(r.layout().unwrap(), l);
        let snapshot = r.snapshot().unwrap();
        assert_eq!(snapshot["catalog"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["catalog"][0]["instanceCount"], 3);
    }
    #[test]
    fn remove_one_instance_preserves_siblings_and_rejects_old_editor() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let other = r.placement("io.example.test", "duplicate", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let before = r.layout().unwrap()["placements"][&other].clone();
        let mut l = r.layout().unwrap();
        l["runtime"] = json!({"edit":{"instance":"io.example.test","serial":1}});
        r.commit(&mut l).unwrap();
        r.remove_instance("io.example.test").unwrap();
        assert!(r.layout().unwrap()["runtime"]["edit"].is_null());
        assert_eq!(r.layout().unwrap()["placements"][&other], before);
        assert!(r
            .placement(
                "io.example.test",
                "save",
                Some(r#"{"revision":0,"settings":{"city":"resurrected"}}"#)
            )
            .is_err());
        assert!(r.remove_instance("missing").is_err());
        assert_eq!(
            r.snapshot().unwrap()["catalog"].as_array().unwrap().len(),
            1
        );
    }
    #[test]
    fn uninstall_keep_preserves_hidden_instances_and_reinstall_does_not_activate() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.placement("io.example.test", "configure", Some(r#"{"city":"Paris"}"#))
            .unwrap();
        let mut expected = r.layout().unwrap()["placements"]["io.example.test"].clone();
        expected["enabled"] = json!(false);
        assert!(r.uninstall("io.example.test", "guess").is_err());
        r.uninstall("io.example.test", "keep").unwrap();
        let snapshot = r.snapshot().unwrap();
        assert_eq!(snapshot["installed"], json!([]));
        assert_eq!(snapshot["catalog"], json!([]));
        assert_eq!(snapshot["retained"][0]["placement"], expected);
        assert!(r.rollback("io.example.test").is_err());
        assert!(r
            .placement(
                "io.example.test",
                "save",
                Some(r#"{"revision":1,"settings":{}}"#)
            )
            .is_err());
        r.install(&src).unwrap();
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.test"],
            expected
        );
        assert_eq!(r.snapshot().unwrap()["retained"], json!([]));
        r.placement("io.example.test", "add", None).unwrap();
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.test"]["settings"]["city"],
            "Paris"
        );
    }
    #[test]
    fn uninstall_delete_and_retained_instance_removal_preserve_other_packages() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let mut m = manifest(&src).unwrap();
        m["id"] = json!("io.example.other");
        fs::write(src.join("widget.json"), m.to_string()).unwrap();
        r.install(&src).unwrap();
        r.placement("io.example.other", "add", None).unwrap();
        let before = r.layout().unwrap()["placements"]["io.example.other"].clone();
        r.uninstall("io.example.test", "keep").unwrap();
        r.remove_instance("io.example.test").unwrap();
        assert_eq!(r.snapshot().unwrap()["retained"], json!([]));
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.other"],
            before
        );
        r.uninstall("io.example.other", "delete").unwrap();
        assert_eq!(r.layout().unwrap()["placements"], json!({}));
        r.install(&src).unwrap();
        assert!(r.snapshot().unwrap()["installed"][0]["placement"].is_null());
    }
    #[test]
    fn invalid_packages_are_visible_and_can_be_uninstalled() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        let path = r.source(&r.layout().unwrap(), "io.example.test").unwrap();
        fs::write(path.join("widget.json"), "broken").unwrap();
        assert!(r.snapshot().unwrap()["catalog"][0]["problem"].is_string());
        r.uninstall("io.example.test", "delete").unwrap();
        assert_eq!(r.snapshot().unwrap()["catalog"], json!([]));
    }
    #[test]
    fn package_previews_are_local_bounded_pngs_for_supported_families() {
        let t = Temp::new();
        fixture(&t.0);
        let mut m = read_json(&t.0.join("widget.json"), 16384).unwrap();
        m["previews"] = json!({"medium":"preview.png"});
        fs::write(t.0.join("widget.json"), m.to_string()).unwrap();
        assert!(validate(&t.0).is_err());
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        header.extend(400_u32.to_be_bytes());
        header.extend(200_u32.to_be_bytes());
        fs::write(t.0.join("preview.png"), &header).unwrap();
        assert!(validate(&t.0).is_ok()); // Header bounds only; Qt handles decode failure with a footprint fallback.
        header[16..20].copy_from_slice(&2000_u32.to_be_bytes());
        fs::write(t.0.join("preview.png"), &header).unwrap();
        assert!(validate(&t.0).is_err());
        for previews in [
            json!({"medium":"../outside.png"}),
            json!({"medium":"https://example.com/a.png"}),
            json!({"large":"preview.png"}),
            json!({"medium":"View.qml"}),
        ] {
            m["previews"] = previews;
            fs::write(t.0.join("widget.json"), m.to_string()).unwrap();
            assert!(validate(&t.0).is_err());
        }
    }
    #[test]
    fn workspace_assignment_preserves_settings_and_layout() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let before = r.layout().unwrap()["placements"]["io.example.test"].clone();
        r.placement("io.example.test", "workspace", Some("2"))
            .unwrap();
        let after = r.layout().unwrap()["placements"]["io.example.test"].clone();
        assert_eq!(after["workspace"], 2);
        for key in [
            "settings", "revision", "monitor", "x", "y", "size", "enabled",
        ] {
            assert_eq!(before[key], after[key]);
        }
        let duplicate = r.placement("io.example.test", "duplicate", None).unwrap();
        assert_eq!(duplicate["placement"]["workspace"], 2);
        for bad in ["0", "-1", "10000", "2.5", "two"] {
            assert!(r
                .placement("io.example.test", "workspace", Some(bad))
                .is_err());
            assert_eq!(
                r.layout().unwrap()["placements"]["io.example.test"]["workspace"],
                2
            );
        }
        r.placement("io.example.test", "workspace", Some("all"))
            .unwrap();
        assert!(r.layout().unwrap()["placements"]["io.example.test"]["workspace"].is_null());
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
        m["coreApi"] = json!(99);
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
        assert!(r.layout().unwrap()["placements"]
            .as_object()
            .unwrap()
            .is_empty());
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
        fs::write(&p, "{\"version\":99,\"placements\":{}}").unwrap();
        assert!(r.layout().is_err());
        assert!(fs::read_to_string(p).unwrap().contains("99"));
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
    #[test]
    fn saves_are_acknowledged_and_stale_editors_cannot_overwrite() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let ack=r.placement("io.example.test","save",Some(r#"{"revision":0,"settings":{"cities":[{"label":"Paris","zone":"Europe/Paris"}]}}"#)).unwrap();
        assert_eq!(ack["placement"]["revision"], 1);
        assert!(r
            .placement(
                "io.example.test",
                "save",
                Some(r#"{"revision":0,"settings":{}}"#)
            )
            .is_err());
        let reopened = Registry {
            data: r.data.clone(),
            state: r.state.clone(),
        };
        assert_eq!(
            reopened.snapshot().unwrap()["installed"][0]["placement"]["settings"]["cities"][0]
                ["label"],
            "Paris"
        );
    }
    #[test]
    fn update_rollback_and_lower_version_preserve_instances() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let second = r.placement("io.example.test", "duplicate", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_string();
        r.placement(&second, "configure", Some(r#"{"city":"Tokyo"}"#))
            .unwrap();
        let mut m = read_json(&src.join("widget.json"), 16384).unwrap();
        m["version"] = json!("0.0.2");
        fs::write(src.join("widget.json"), m.to_string()).unwrap();
        r.deploy(&src, true).unwrap();
        assert_eq!(
            r.snapshot().unwrap()["installed"][0]["manifest"]["version"],
            "0.0.2"
        );
        r.rollback("io.example.test").unwrap();
        let snap = r.snapshot().unwrap();
        assert_eq!(snap["installed"][0]["manifest"]["version"], "0.1.0");
        assert_eq!(snap["installed"].as_array().unwrap().len(), 2);
        assert_eq!(
            r.layout().unwrap()["placements"][&second]["settings"]["city"],
            "Tokyo"
        );
    }
    #[test]
    fn legacy_migration_preserves_settings_and_original_backup() {
        let t = Temp::new();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        fixture(&r.package("io.example.test").unwrap());
        fs::create_dir_all(&r.state).unwrap();
        let old = json!({"version":1,"placements":{"io.example.test":{"enabled":true,"x":40,"y":80,"monitor":"DP-1","size":"standard","settings":{"cities":["London"],"appearance":{"radius":18}}}}});
        fs::write(r.state.join("layout.json"), old.to_string()).unwrap();
        assert_eq!(
            r.snapshot().unwrap()["installed"][0]["placement"]["size"],
            "medium"
        );
        r.placement("io.example.test", "hide", None).unwrap();
        assert_eq!(
            read_json(&r.state.join("layout-v1.backup.json"), 65536).unwrap(),
            old
        );
        let migrated = r.layout().unwrap();
        assert_eq!(migrated["version"], 2);
        assert_eq!(
            migrated["placements"]["io.example.test"]["settings"],
            old["placements"]["io.example.test"]["settings"]
        );
    }
    #[test]
    fn api_two_families_and_grid_are_enforced() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let mut m = read_json(&src.join("widget.json"), 16384).unwrap();
        m["schemaVersion"] = json!(2);
        m["coreApi"] = json!(2);
        m["families"] = json!(["medium", "large"]);
        m["defaultFamily"] = json!("large");
        fs::write(src.join("widget.json"), m.to_string()).unwrap();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        let ack = r
            .placement_using(
                "io.example.test",
                "place",
                Some(r#"{"column":1,"row":2,"monitor":"DP-1","size":"medium"}"#),
                grid::tests::desktop,
            )
            .unwrap();
        assert_eq!(ack["placement"]["cell"]["column"], 1);
        assert_eq!(ack["placement"]["cell"]["row"], 2);
        assert!(r
            .placement(
                "io.example.test",
                "place",
                Some(r#"{"x":0,"y":0,"monitor":"","size":"small"}"#)
            )
            .is_err());
        m["families"] = json!(["medium", "medium"]);
        fs::write(src.join("widget.json"), m.to_string()).unwrap();
        assert!(validate(&src).is_err());
    }
    #[test]
    fn grid_writes_are_atomic_and_do_not_move_other_instances() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement_using("io.example.test", "add", None, grid::tests::desktop)
            .unwrap();
        let duplicate = r
            .placement_using("io.example.test", "duplicate", None, grid::tests::desktop)
            .unwrap();
        let id = duplicate["updated"].as_str().unwrap();
        let before = fs::read(r.state.join("layout.json")).unwrap();
        assert_eq!(duplicate["placement"]["cell"]["column"], 2);
        let target = r#"{"column":0,"row":0,"size":"medium","monitor":"DP-1"}"#;
        assert!(r
            .placement_using(id, "place", Some(target), grid::tests::desktop)
            .is_err());
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        r.placement_using(id, "workspace", Some("2"), grid::tests::desktop)
            .unwrap();
        r.placement_using(
            "io.example.test",
            "workspace",
            Some("1"),
            grid::tests::desktop,
        )
        .unwrap();
        r.placement_using(id, "place", Some(target), grid::tests::desktop)
            .unwrap();
        let before = fs::read(r.state.join("layout.json")).unwrap();
        assert!(r
            .placement_using(id, "workspace", Some("all"), grid::tests::desktop)
            .is_err());
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        r.placement_using("io.example.test", "hide", None, grid::tests::desktop)
            .unwrap();
        r.placement_using(id, "workspace", Some("all"), grid::tests::desktop)
            .unwrap();
        r.placement_using("io.example.test", "add", None, grid::tests::desktop)
            .unwrap();
        let resolved = grid::resolve(&r.layout().unwrap()["placements"], &grid::tests::desktop());
        assert_eq!(resolved[id]["column"], 0);
        assert_eq!(resolved["io.example.test"]["column"], 2);
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.test"]["cell"]["column"],
            0
        );
    }
    #[test]
    fn invalid_update_leaves_current_package_and_state_intact() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        let before = r.layout().unwrap();
        fs::write(src.join("widget.json"), "bad json").unwrap();
        assert!(r.deploy(&src, true).is_err());
        assert_eq!(r.layout().unwrap(), before);
        assert_eq!(
            r.snapshot().unwrap()["installed"].as_array().unwrap().len(),
            1
        );
    }
    #[test]
    fn host_control_survives_reopen_without_session_ipc() {
        let t = Temp::new();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.control("manage").unwrap();
        assert_eq!(r.snapshot().unwrap()["runtime"]["managerOpen"], true);
        r.control("close-manager").unwrap();
        r.control("arrange").unwrap();
        assert_eq!(r.snapshot().unwrap()["runtime"]["editing"], true);
        r.control("hide-all").unwrap();
        let state = r.snapshot().unwrap();
        assert_eq!(state["runtime"]["shown"], false);
        assert_eq!(state["runtime"]["editing"], false);
        let before = r.layout().unwrap();
        assert!(r.control("exec").is_err());
        assert_eq!(r.layout().unwrap(), before);
    }
    #[test]
    fn reveal_toggles_without_changing_layout_or_hidden_preferences() {
        let t = Temp::new();
        let source = t.0.join("source");
        fixture(&source);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&source).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.control("hide-all").unwrap();
        let before = r.layout().unwrap()["placements"].clone();
        r.control("reveal").unwrap();
        let state = r.snapshot().unwrap();
        assert_eq!(state["runtime"]["revealing"], true);
        assert_eq!(state["runtime"]["shown"], false);
        r.control("reveal").unwrap();
        assert_eq!(r.snapshot().unwrap()["runtime"]["revealing"], false);
        assert_eq!(r.layout().unwrap()["placements"], before);
    }
    #[test]
    fn weather_grant_is_explicit_and_generation_bound() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let mut m = manifest(&src).unwrap();
        m["capabilities"] = json!(["weather"]);
        atomic_json(&src.join("widget.json"), &m).unwrap();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        assert!(!weather::authorised(
            &r,
            &r.layout().unwrap(),
            "io.example.test"
        ));
        weather::grant(&r, "io.example.test", "allow").unwrap();
        assert!(weather::authorised(
            &r,
            &r.layout().unwrap(),
            "io.example.test"
        ));
        r.deploy(&src, true).unwrap();
        assert!(!weather::authorised(
            &r,
            &r.layout().unwrap(),
            "io.example.test"
        ));
        weather::grant(&r, "io.example.test", "allow").unwrap();
        weather::grant(&r, "io.example.test", "deny").unwrap();
        assert!(!weather::authorised(
            &r,
            &r.layout().unwrap(),
            "io.example.test"
        ));
    }
    #[test]
    fn migrations_checkpoint_rollback_and_post_update_edits() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.placement("io.example.test", "configure", Some(r#"{"city":"Paris"}"#))
            .unwrap();
        let original = r.layout().unwrap();
        let mut m = manifest(&src).unwrap();
        m["settingsVersion"] = json!(2);
        m["defaults"] = json!({"label":"London"});
        m["settingsSchema"] =
            json!({"type":"object","properties":{"label":{"type":"string"}},"required":["label"]});
        atomic_json(&src.join("widget.json"), &m).unwrap();
        assert!(r.deploy(&src, true).is_err());
        assert_eq!(r.layout().unwrap(), original);
        m["migrations"] = json!([{"from":1,"to":2,"rename":{"city":"label"}}]);
        atomic_json(&src.join("widget.json"), &m).unwrap();
        r.deploy(&src, true).unwrap();
        let migrated = r.layout().unwrap();
        assert_eq!(
            migrated["placements"]["io.example.test"]["settings"],
            json!({"label":"Paris"})
        );
        r.placement("io.example.test", "hide", None).unwrap();
        r.rollback("io.example.test").unwrap();
        let rolled = r.layout().unwrap();
        assert_eq!(
            rolled["placements"]["io.example.test"]["settings"],
            json!({"city":"Paris"})
        );
        assert_eq!(rolled["placements"]["io.example.test"]["enabled"], false);
        assert_eq!(
            rolled["packages"]["io.example.test"]["current"],
            original["packages"]["io.example.test"]["current"]
        );
        r.rollback("io.example.test").unwrap();
        r.placement("io.example.test", "configure", Some(r#"{"label":"Tokyo"}"#))
            .unwrap();
        let edited = r.layout().unwrap();
        assert!(r.rollback("io.example.test").is_err());
        assert_eq!(r.layout().unwrap(), edited);
        assert!(r
            .placement("io.example.test", "configure", Some(r#"{"label":12}"#))
            .is_err());
        assert_eq!(r.layout().unwrap(), edited);
    }
    #[test]
    fn package_fault_controls_preserve_instances_and_reset_serial() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let before = r.layout().unwrap()["placements"].clone();
        r.package_control("io.example.test", "disable").unwrap();
        assert_eq!(r.snapshot().unwrap()["catalog"][0]["packageDisabled"], true);
        let serial = r.layout().unwrap()["runtime"]["packageControls"]["io.example.test"]["serial"]
            .as_u64()
            .unwrap();
        r.package_control("io.example.test", "restart").unwrap();
        let state = r.layout().unwrap();
        assert_eq!(
            state["runtime"]["packageControls"]["io.example.test"]["disabled"],
            false
        );
        assert!(
            state["runtime"]["packageControls"]["io.example.test"]["serial"]
                .as_u64()
                .unwrap()
                > serial
        );
        assert_eq!(state["placements"], before);
        assert!(r.package_control("io.example.test", "exec").is_err());
    }
    #[test]
    fn code_changes_wait_for_owned_editor_and_stale_close_cannot_release_it() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.request_edit("io.example.test").unwrap();
        let before = r.layout().unwrap();
        assert!(r.deploy(&src, true).is_err());
        assert_eq!(r.layout().unwrap(), before);
        r.finish_edit("io.example.test", "wrong").unwrap();
        assert_eq!(r.layout().unwrap(), before);
        r.finish_edit(
            "io.example.test",
            &before["runtime"]["edit"]["serial"]
                .as_u64()
                .unwrap()
                .to_string(),
        )
        .unwrap();
        r.deploy(&src, true).unwrap();
        r.request_edit("io.example.test").unwrap();
        assert!(r.rollback("io.example.test").is_err());
        r.package_control("io.example.test", "restart").unwrap();
        r.rollback("io.example.test").unwrap();
    }
}
