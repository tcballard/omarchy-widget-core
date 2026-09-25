mod declarative;
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
    let v = read_json(&dir.join("widget.json"), 16384)?;
    if v["coreApi"] == 1 {
        return Err("Widget API 1 has been removed; port this package to schemaVersion 2/coreApi 3 with small, medium and large families".into());
    }
    if v["kind"] != "desktop-widget"
        || v["schemaVersion"] != 2
        || !(v["coreApi"] == 2 || v["coreApi"] == 3)
    {
        return Err("Requires desktop-widget schema 2/API 2–3; upgrade Core for newer APIs".into());
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
    let canonical = dir.canonicalize().map_err(err)?;
    if declarative::is(&v) {
        declarative::contract(&v)?;
    } else {
        if v.get("renderer").is_some_and(|x| x != "qml") {
            return Err("Unknown renderer".into());
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
    }
    if !v["defaults"].is_object() || serde_json::to_vec(&v["defaults"]).map_err(err)?.len() > 8192 {
        return Err("Defaults must be an object up to 8 KiB".into());
    }
    let values = v["families"].as_array().ok_or("Missing families")?;
    if values.is_empty() || values.len() > 3 {
        return Err("Declare 1–3 families".into());
    }
    let mut families = Vec::new();
    for value in values {
        let name = value.as_str().ok_or("Invalid family")?;
        if family(name).is_err() || families.contains(&name) {
            return Err("Families must be unique: small, medium, large".into());
        }
        families.push(name);
    }
    if !families.iter().any(|name| v["defaultFamily"] == *name) {
        return Err("Invalid defaultFamily".into());
    }
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
            if !families.contains(&size.as_str()) {
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
    if let Some(features) = v.get("requires") {
        if v["coreApi"] != 3 {
            return Err("Required features need Core API 3".into());
        }
        let features = features
            .as_array()
            .filter(|a| a.len() <= 8)
            .ok_or("requires must be a bounded feature array")?;
        for feature in features {
            if ![
                "settings-schema",
                "settings-migrations",
                "acknowledged-actions",
                "shared-weather",
                "frame-settings",
                "command-dependencies",
                "declarative-v1",
            ]
            .iter()
            .any(|s| feature == s)
            {
                return Err(format!(
                    "Unsupported required feature: {feature}; upgrade Core"
                ));
            }
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
        "small" | "medium" | "large" => Ok(name),
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
        self.lock_kind(false)
    }
    fn read_lock(&self) -> Result<File> {
        self.lock_kind(true)
    }
    fn lock_kind(&self, shared: bool) -> Result<File> {
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
        (if shared {
            file.try_lock_shared()
        } else {
            file.try_lock()
        })
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
            return Ok(
                json!({"version":2,"revision":0,"placements":{},"packages":{},"retiredLegacyIds":{}}),
            );
        }
        let original = read_json(&path, 1024 * 1024)?;
        let retire_unknown_aliases = original["retiredLegacyIds"].is_null();
        let mut layout = Self::validate_layout(original)?;
        if retire_unknown_aliases {
            for id in self.ids(&layout)? {
                if layout["placements"][&id].is_null() {
                    layout["retiredLegacyIds"][id] = json!(true);
                }
            }
        }
        Ok(layout)
    }
    fn validate_layout(mut v: Value) -> Result<Value> {
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
        Self::validate_runtime(&v["runtime"])?;
        if v["packages"]
            .as_object()
            .unwrap()
            .iter()
            .any(|(id, p)| !id_ok(id) || !p.is_object())
        {
            return Err(
                "Invalid package metadata; layout preserved. Restore a matching backup.".into(),
            );
        }
        // Old registries cannot tell whether an absent package-ID instance once
        // existed. Retire those aliases on upgrade; existing identities survive.
        if v["retiredLegacyIds"].is_null() {
            v["retiredLegacyIds"] = json!({});
            let ids: Vec<_> = v["packages"].as_object().unwrap().keys().cloned().collect();
            for id in ids {
                if v["placements"][&id].is_null() {
                    v["retiredLegacyIds"][id] = json!(true);
                }
            }
        }
        if !v["retiredLegacyIds"].as_object().is_some_and(|ids| {
            ids.iter()
                .all(|(id, retired)| id_ok(id) && *retired == true)
        }) {
            return Err("Invalid retired identity metadata; layout preserved".into());
        }
        for (id, p) in v["placements"].as_object_mut().unwrap() {
            if !id_ok(id)
                || !p.is_object()
                || !p["enabled"].is_boolean()
                || !p["settings"].is_object()
                || !["x", "y"]
                    .iter()
                    .all(|k| p[*k].as_f64().is_some_and(|n| (0.0..=20000.0).contains(&n)))
                || p["monitor"].as_str().is_none_or(|s| s.len() > 120)
                || !p["size"].is_string()
                || !workspaces::valid(&p["workspace"])
                || !grid::valid_preference(p)
            {
                return Err("Invalid placement; layout preserved".into());
            }
            if legacy {
                // Registry recovery is independent of the removed widget API.
                p["size"] = json!(match p["size"].as_str().unwrap() {
                    "compact" => "small",
                    "standard" => "medium",
                    "wide" => "large",
                    name => name,
                });
                p["packageId"] = json!(id);
                p["definitionId"] = json!("main");
                p["revision"] = json!(0);
            }
            family(p["size"].as_str().unwrap())?;
            if !id_ok(p["packageId"].as_str().unwrap_or(""))
                || p["definitionId"] != "main"
                || p["revision"].as_u64().is_none()
            {
                return Err("Invalid instance identity or revision".into());
            }
        }
        Ok(v)
    }
    fn validate_runtime(v: &Value) -> Result<()> {
        let bad =
            || "Invalid runtime metadata; layout preserved. Use omarchy-widget repair.".to_string();
        if v.is_null() {
            return Ok(());
        }
        if !v.is_object() {
            return Err(bad());
        }
        for key in ["shown", "editing", "managerOpen", "revealing"] {
            if !v[key].is_null() && !v[key].is_boolean() {
                return Err(bad());
            }
        }
        if !v["revealUntil"].is_null() && v["revealUntil"].as_u64().is_none() {
            return Err(bad());
        }
        let edit = &v["edit"];
        if !edit.is_null()
            && (!edit.is_object()
                || !edit["instance"].as_str().is_some_and(id_ok)
                || edit["serial"].as_u64().is_none())
        {
            return Err(bad());
        }
        if !v["packageControls"].is_null() {
            let controls = v["packageControls"].as_object().ok_or_else(bad)?;
            for (id, c) in controls {
                if !id_ok(id)
                    || !c.is_object()
                    || !c["disabled"].is_boolean()
                    || c["serial"].as_u64().is_none()
                    || (!c["loadFailure"].is_null() && !c["loadFailure"].is_boolean())
                {
                    return Err(bad());
                }
            }
        }
        Ok(())
    }
    fn repair_candidate(mut v: Value) -> Result<(Value, Vec<String>)> {
        if !v.is_object() || !v["placements"].is_object() {
            return Err(
                "Layout cannot be repaired automatically; restore a matching backup.".into(),
            );
        }
        let mut problems = Vec::new();
        if Self::validate_runtime(&v["runtime"]).is_err() {
            v["runtime"] = Value::Null;
            problems.push("Invalid runtime controls quarantined".into());
        }
        let placements = v["placements"].clone();
        v["placements"] = json!({});
        // Refuse unknown versions/invalid registry metadata before isolating rows.
        Self::validate_layout(v.clone())?;
        for (id, placement) in placements.as_object().unwrap() {
            v["placements"][id] = placement.clone();
            if Self::validate_layout(v.clone()).is_err() {
                v["placements"].as_object_mut().unwrap().remove(id);
                problems.push(format!("Invalid placement quarantined: {id}"));
                if id_ok(id) {
                    v["retiredLegacyIds"][id] = json!(true);
                }
            }
        }
        if !v["runtime"]["edit"].is_null()
            && v["placements"][v["runtime"]["edit"]["instance"].as_str().unwrap_or("")].is_null()
        {
            v["runtime"]["edit"] = Value::Null;
            problems.push("Orphaned settings registration cleared".into());
        }
        Ok((Self::validate_layout(v)?, problems))
    }
    fn display_layout(&self) -> Result<(Value, Vec<String>)> {
        match self.layout() {
            Ok(v) => Ok((v, vec![])),
            Err(original) => {
                let (v, mut problems) = Self::repair_candidate(read_json(
                    &self.state.join("layout.json"),
                    1024 * 1024,
                )?)?;
                if problems.is_empty() {
                    return Err(original);
                }
                problems.push("Saved layout needs repair. Valid widgets remain visible; choose Repair saved layout in Widgets. Original file preserved.".into());
                Ok((v, problems))
            }
        }
    }
    fn repair(&self) -> Result<Value> {
        let _lock = self.lock()?;
        let path = self.state.join("layout.json");
        let (mut v, problems) = Self::repair_candidate(read_json(&path, 1024 * 1024)?)?;
        if problems.is_empty() {
            return Ok(json!({"message":"Saved layout is valid; no changes made."}));
        }
        let backup = self
            .state
            .join(format!("layout-quarantine-{}.json", nonce()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&backup)
            .map_err(err)?;
        file.write_all(&fs::read(&path).map_err(err)?)
            .map_err(err)?;
        file.sync_all().map_err(err)?;
        File::open(&self.state)
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
        // Repairing an older registry must not resurrect an absent legacy alias.
        for id in self.ids(&v)? {
            if v["placements"][&id].is_null() {
                v["retiredLegacyIds"][id] = json!(true);
            }
        }
        self.commit(&mut v)?;
        Ok(
            json!({"message":format!("Layout repaired. Original saved at {}",backup.display()),"backup":backup,"quarantined":problems}),
        )
    }
    fn editor_remedy(l: &Value) -> String {
        let id = l["runtime"]["edit"]["instance"]
            .as_str()
            .unwrap_or("unknown widget");
        let package = l["placements"][id]["packageId"].as_str().unwrap_or(id);
        format!("Settings pending for {package} ({id}). Save or cancel that window, cancel the pending settings in Widgets, or Restart {package} in Widgets → Available.")
    }
    fn clear_dead_editor(&self, package: Option<&str>) -> Result<()> {
        let lock = self.lock()?;
        let l = self.layout()?;
        let id = l["runtime"]["edit"]["instance"].as_str().unwrap_or("");
        if !id.is_empty() && package.is_none_or(|p| l["placements"][id]["packageId"] == p) {
            self.finish_edit_locked(id, &l["runtime"]["edit"]["serial"].to_string(), &lock)?;
        }
        Ok(())
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
        let lock = self.lock()?;
        self.request_edit_locked(id, &lock)
    }
    fn request_edit_locked(&self, id: &str, _lock: &File) -> Result<Value> {
        let mut l = self.layout()?;
        if !l["placements"][id].is_object() {
            return Err("Add the widget before editing it".into());
        }
        let package = l["placements"][id]["packageId"].as_str().unwrap();
        if l["runtime"]["packageControls"][package]["disabled"] == true {
            return Err("Enable this package before opening settings".into());
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
            return Err(Self::editor_remedy(&l));
        }
        l["runtime"]["edit"] = json!({"instance":id,"serial":l["revision"].as_u64().unwrap().checked_add(1).ok_or("Revision exhausted")?});
        self.commit(&mut l)?;
        Ok(json!(true))
    }
    fn finish_edit(&self, id: &str, serial: &str) -> Result<Value> {
        let lock = self.lock()?;
        self.finish_edit_locked(id, serial, &lock)
    }
    fn finish_edit_locked(&self, id: &str, serial: &str, _lock: &File) -> Result<Value> {
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
            return Err(Self::editor_remedy(l));
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
        let lock = self.lock()?;
        self.control_locked(method, &lock)
    }
    fn control_locked(&self, method: &str, _lock: &File) -> Result<Value> {
        let mut l = self.layout()?;
        let mut runtime = l["runtime"].clone();
        if !runtime.is_object() {
            runtime = json!({"shown":true,"editing":false,"managerOpen":false});
        }
        match method {
            "reveal" => {
                if !cfg!(feature = "experimental-reveal") {
                    return Err("Quick reveal is experimental and disabled in this build. Use Widgets or Arrange widgets.".into());
                }
                if !runtime["edit"].is_null() {
                    return Err(Self::editor_remedy(&l));
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
            "toggle" => {
                runtime["shown"] = json!(!runtime["shown"].as_bool().unwrap_or(true));
                runtime["revealUntil"] = json!(0);
                runtime["editing"] = json!(false);
            }
            "hide-all" => {
                runtime["revealUntil"] = json!(0);
                runtime["shown"] = json!(false);
                runtime["editing"] = json!(false);
            }
            "refresh" => (),
            _ => return Err("Unknown host control".into()),
        }
        if l["runtime"] == runtime && method != "refresh" {
            return Ok(json!({"runtime":runtime,"revision":l["revision"]}));
        }
        l["runtime"] = runtime.clone();
        self.commit(&mut l)?;
        Ok(json!({"runtime":runtime,"revision":l["revision"]}))
    }
    fn snapshot(&self) -> Result<Value> {
        let (mut l, mut problems) = self.display_layout()?;
        let repair_required = !problems.is_empty();
        let active = cfg!(feature = "experimental-reveal")
            && reveal::active(
                l["runtime"]["revealUntil"].as_u64().unwrap_or(0),
                reveal::now(),
            );
        l["runtime"]["revealing"] = json!(active);
        let desktop = workspaces::snapshot();
        if grid::migrate(&mut l["placements"], &desktop) && !repair_required {
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
                        json!({"packageId":id,"manifest":m,"directory":path,"instanceCount":count,"weatherAllowed":weather::authorised(self,&l,&id),"packageDisabled":l["runtime"]["packageControls"][&id]["disabled"]==true,"loadFailure":l["runtime"]["packageControls"][&id]["loadFailure"]==true,"health":if health_fresh {health["packages"][&id].clone()} else {Value::Null}}),
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
            json!({"api":2,"repairRequired":repair_required,"experimentalReveal":cfg!(feature="experimental-reveal"),"revision":l["revision"],"installed":widgets,"catalog":catalog,"retained":retained,"problems":problems,"appearance":appearance,"palette":self.palette(),"runtime":l["runtime"],"desktop":desktop,"occupancy":effective.as_object().unwrap().values().cloned().collect::<Vec<_>>()}),
        )
    }
    fn install(&self, source: &Path) -> Result<Value> {
        self.deploy(source, false)
    }
    fn deploy(&self, source: &Path, update: bool) -> Result<Value> {
        self.deploy_observing(source, update, |_| Ok(()))
    }
    fn deploy_observing(
        &self,
        source: &Path,
        update: bool,
        phase: impl Fn(&str) -> Result<()>,
    ) -> Result<Value> {
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
                // Only owner execute survives; never propagate setuid/setgid or group/world modes.
                let executable = fs::metadata(source.join(&p))
                    .map_err(err)?
                    .permissions()
                    .mode()
                    & 0o100
                    != 0;
                fs::set_permissions(
                    &to,
                    fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
                )
                .map_err(err)?;
                File::open(&to).map_err(err)?.sync_all().map_err(err)?;
            }
            if validate(&stage)? != m {
                return Err("Package changed during staging; retry the update".into());
            }
            phase("staged")?;
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
            phase("before-commit")?;
            self.commit(&mut l)?;
            phase("committed")?;
            Ok(
                json!({"installed":id,"updated":update,"revision":l["revision"],"restartRequired":true}),
            )
        })();
        // An unreferenced version is harmless after a crash. Do not delete here:
        // a directory-fsync error can occur after the pointer commit succeeds.
        result
    }
    fn export_settings(&self, id: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let l = self.layout()?;
        let m = manifest(&self.source(&l, id)?)?;
        let file = self
            .state
            .join("exports")
            .join(format!("{id}-{}.json", nonce()));
        fs::create_dir_all(file.parent().unwrap()).map_err(err)?;
        let value = json!({"format":1,"packageId":id,"packageVersion":m["version"],"generation":l["packages"][id]["current"],"registryRevision":l["revision"],"instances":settings::checkpoint(&l["placements"],id)});
        atomic_json(&file, &value)?;
        Ok(json!({"exported":file,"message":format!("Settings exported to {}",file.display())}))
    }
    fn restore_settings(&self, id: &str, file: &Path) -> Result<Value> {
        let _lock = self.lock()?;
        let value = read_json(file, 1024 * 1024).map_err(|_| "That file is not a readable Widget Core settings export; current settings preserved".to_string())?;
        if value["format"] != 1 || value["packageId"] != id || !value["instances"].is_object() {
            return Err("Invalid settings export or package identity".into());
        }
        let mut l = self.layout()?;
        self.ensure_no_editor(&l, id)?;
        let m = manifest(&self.source(&l, id)?)?;
        let mut skipped = Vec::new();
        let mut restored = 0;
        for (instance, saved) in value["instances"].as_object().unwrap() {
            if l["placements"][instance].is_null() {
                skipped.push(instance.clone());
                continue;
            }
            let p = &mut l["placements"][instance];
            if p["packageId"] != id {
                return Err(
                    "Export contains a removed or foreign instance; current settings preserved"
                        .into(),
                );
            }
            let migrated = settings::migrate(
                &m,
                saved["settingsVersion"].as_u64().unwrap_or(1),
                &saved["settings"],
            )?;
            restored += 1;
            p["settings"] = migrated;
            p["settingsVersion"] = json!(settings::version(&m));
            p["revision"] = json!(p["revision"]
                .as_u64()
                .unwrap()
                .checked_add(1)
                .ok_or("Revision exhausted")?);
        }
        if restored == 0 {
            return Err(
                "No matching instances remain in this export; current settings preserved".into(),
            );
        }
        self.commit(&mut l)?;
        Ok(
            json!({"restored":id,"count":restored,"skippedRemovedInstances":skipped,"revision":l["revision"]}),
        )
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
                    return Err("Settings changed after update; rollback would overwrite them. Use Export settings in Available, then install a compatible package. Your current code and settings are unchanged.".into());
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
        let lock = self.lock()?;
        self.placement_locked(id, operation, value, desktop, &lock)
    }
    fn placement_locked(
        &self,
        id: &str,
        operation: &str,
        value: Option<&str>,
        desktop: impl FnOnce() -> Value,
        _lock: &File,
    ) -> Result<Value> {
        if !id_ok(id) {
            return Err("Invalid instance ID".into());
        }
        let mut l = self.layout()?;
        let desktop = desktop();
        if l["placements"][id].is_null() && !["add", "create"].contains(&operation) {
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
        } else if operation == "add"
            && l["placements"][id].is_null()
            && l["retiredLegacyIds"][id] == true
        {
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
            "add" | "show" | "duplicate" => {
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
            "recover-placement" => {
                let raw = value.ok_or("Choose a size and monitor")?;
                if raw.len() > 1024 {
                    return Err("Placement too large".into());
                }
                let v: Value = serde_json::from_str(raw).map_err(err)?;
                let size = family(v["size"].as_str().unwrap_or(""))?;
                if !m["families"].as_array().unwrap().contains(&json!(size)) {
                    return Err("Unsupported widget family".into());
                }
                let monitor = v["monitor"]
                    .as_str()
                    .ok_or("Choose Automatic or an available monitor")?;
                let mut candidate = p.clone();
                candidate["size"] = json!(size);
                let grids = desktop["grids"]
                    .as_object()
                    .ok_or("Desktop unavailable; retry when connected")?;
                let mut target = if (monitor.is_empty() || p["monitor"] == monitor)
                    && grid::validate_change(&instance, &candidate, &current, &desktop).is_ok()
                {
                    Some(candidate.clone())
                } else {
                    None
                };
                'search: for (name, g) in grids {
                    if target.is_some() {
                        break;
                    }
                    if !monitor.is_empty() && monitor != name {
                        continue;
                    }
                    for row in 0..g["rows"].as_u64().unwrap_or(0) {
                        for column in 0..g["columns"].as_u64().unwrap_or(0) {
                            candidate["monitor"] = json!(name);
                            candidate["cell"] = json!({"column":column,"row":row});
                            if grid::validate_change(&instance, &candidate, &current, &desktop)
                                .is_ok()
                            {
                                target = Some(candidate.clone());
                                break 'search;
                            }
                        }
                    }
                }
                *p=target.ok_or("No free cells for that size on the selected monitor. Choose a smaller size or another monitor.")?;
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
        if operation == "recover-placement" && self.layout()?["placements"][&instance] == placement
        {
            return Ok(json!({"updated":instance,"placement":placement,"revision":l["revision"]}));
        }
        self.commit(&mut l)?;
        Ok(json!({"updated":instance,"placement":placement,"revision":l["revision"]}))
    }
    fn remove(&self, id: &str) -> Result<Value> {
        self.uninstall(id, "delete")
    }
    fn remove_instance(&self, id: &str) -> Result<Value> {
        let _lock = self.lock()?;
        let mut l = self.layout()?;
        if l["placements"][id]["packageId"] == id {
            l["retiredLegacyIds"][id] = json!(true);
        }
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
            l["retiredLegacyIds"][id] = json!(true);
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
    if cmd == "clock-times" && args.len() == 2 {
        return declarative::clock_times(&args[1]);
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
    if (cmd == "new" || cmd == "new-qml") && args.len() == 4 {
        return sdk::scaffold(Path::new(&args[1]), &args[2], &args[3], cmd == "new-qml");
    }
    if cmd == "help" {
        return Ok(
            json!({"commands":["repair","show INSTANCE_ID","export-settings PACKAGE_ID","restore-settings PACKAGE_ID PATH","recover-placement INSTANCE_ID {size,monitor}","new PATH ID NAME (declarative)","new-qml PATH ID NAME (isolated QML)","weather-permission PACKAGE allow|deny","package-control PACKAGE restart|disable|enable","validate PATH","install PATH","update PATH","rollback PACKAGE_ID","list","control METHOD","add ID","create PACKAGE_ID FAMILY","duplicate INSTANCE_ID","hide INSTANCE_ID","remove-instance INSTANCE_ID","uninstall PACKAGE_ID keep|delete","save INSTANCE_ID {revision,settings}","configure INSTANCE_ID JSON (legacy)","place INSTANCE_ID JSON","workspace INSTANCE_ID all|NUMBER","remove PACKAGE_ID (legacy: deletes package and settings)"],"api":2,"version":"0.0.2"}),
        );
    }
    let required = match cmd {
        "list" | "repair" => 1,
        "validate" | "install" | "update" | "rollback" | "add" | "duplicate" | "hide" | "show"
        | "remove" | "remove-instance" | "control" | "export-settings" => 2,
        "place" | "configure" | "save" | "workspace" | "create" | "uninstall"
        | "recover-placement" | "restore-settings" => 3,
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
        "repair" => r.repair(),
        "control" => r.control(&args[1]),
        "install" => r.install(Path::new(&args[1])),
        "update" => r.deploy(Path::new(&args[1]), true),
        "rollback" => r.rollback(&args[1]),
        "export-settings" => r.export_settings(&args[1]),
        "restore-settings" => r.restore_settings(&args[1], Path::new(&args[2])),
        "remove" => r.remove(&args[1]),
        "remove-instance" => r.remove_instance(&args[1]),
        "uninstall" => r.uninstall(&args[1], &args[2]),
        _ => r.placement(&args[1], cmd, args.get(2).map(String::as_str)),
    }
}
fn retry_busy(mut operation: impl FnMut() -> Result<Value>) -> Result<Value> {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match operation() {
            Err(e) if e.starts_with("Registry busy;") && std::time::Instant::now() < until => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            result => return result,
        }
    }
}
fn error_response(e: &str) -> Value {
    json!({"error":e.chars().take(1000).collect::<String>(),"code":if e.starts_with("Registry busy;") {"busy"} else if e.starts_with("Runner write budget") {"rate_limited"} else {"operation_failed"}})
}
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    // Busy means the lock was refused before dispatch; retry no uncertain writes.
    let result = retry_busy(|| run(&args));
    match result {
        Ok(v) => println!("{v}"),
        Err(e) => {
            println!("{}", error_response(&e));
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
        fs::write(p.join("widget.json"),json!({"schemaVersion":2,"kind":"desktop-widget","coreApi":3,"id":"io.example.test","name":"Contract fixture","version":"0.1.0","entryPoint":"View.qml","families":["medium"],"defaultFamily":"medium","defaults":{}}).to_string()).unwrap();
    }
    fn api_one_manifest(p: &Path) -> Value {
        let mut m = read_json(&p.join("widget.json"), 16384).unwrap();
        m["schemaVersion"] = json!(1);
        m["coreApi"] = json!(1);
        m["sizes"] = json!({"standard":{"width":300,"height":200}});
        m["defaultSize"] = json!("standard");
        m.as_object_mut().unwrap().remove("families");
        m.as_object_mut().unwrap().remove("defaultFamily");
        m
    }
    #[test]
    fn api_one_rejected_by_validate_install_and_update_without_registry_changes() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let current = manifest(&src).unwrap();
        let mut old = api_one_manifest(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        for schema in [1, 2] {
            old["schemaVersion"] = json!(schema);
            atomic_json(&src.join("widget.json"), &old).unwrap();
            assert!(validate(&src)
                .unwrap_err()
                .contains("API 1 has been removed"));
            assert!(r
                .install(&src)
                .unwrap_err()
                .contains("API 1 has been removed"));
            assert!(!r.state.join("layout.json").exists());
            assert!(!r.data.join("versions").exists());
        }
        atomic_json(&src.join("widget.json"), &current).unwrap();
        r.install(&src).unwrap();
        let before = fs::read(r.state.join("layout.json")).unwrap();
        atomic_json(&src.join("widget.json"), &old).unwrap();
        assert!(r
            .deploy(&src, true)
            .unwrap_err()
            .contains("API 1 has been removed"));
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        assert_eq!(
            r.snapshot().unwrap()["catalog"][0]["manifest"]["coreApi"],
            3
        );
    }
    #[test]
    fn already_installed_api_one_is_not_loaded_and_rollback_cannot_restore_it() {
        let t = Temp::new();
        let r = review_registry(&t);
        r.placement(
            "io.example.test",
            "configure",
            Some(r#"{"label":"Keep me"}"#),
        )
        .unwrap();
        let before = fs::read(r.state.join("layout.json")).unwrap();
        let old_path = r.source(&r.layout().unwrap(), "io.example.test").unwrap();
        atomic_json(&old_path.join("widget.json"), &api_one_manifest(&old_path)).unwrap();
        let snapshot = r.snapshot().unwrap();
        assert!(snapshot["installed"].as_array().unwrap().is_empty());
        assert!(snapshot["catalog"][0]["problem"]
            .as_str()
            .unwrap()
            .contains("Invalid package"));
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        r.deploy(&t.0.join("source"), true).unwrap();
        let before_rollback = fs::read(r.state.join("layout.json")).unwrap();
        assert!(r
            .rollback("io.example.test")
            .unwrap_err()
            .contains("API 1 has been removed"));
        assert_eq!(
            fs::read(r.state.join("layout.json")).unwrap(),
            before_rollback
        );
        assert_eq!(
            r.snapshot().unwrap()["installed"][0]["manifest"]["coreApi"],
            3
        );
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.test"]["settings"]["label"],
            "Keep me"
        );
    }
    #[test]
    fn placement_commands_reject_api_one_size_aliases() {
        let t = Temp::new();
        let r = review_registry(&t);
        let before = fs::read(r.state.join("layout.json")).unwrap();
        for size in ["compact", "standard", "wide"] {
            let value = json!({"column":0,"row":0,"monitor":"DP-1","size":size}).to_string();
            for operation in ["place", "recover-placement"] {
                assert!(r
                    .placement_using(
                        "io.example.test",
                        operation,
                        Some(&value),
                        grid::tests::desktop
                    )
                    .is_err());
            }
        }
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
    }
    #[test]
    fn deleted_instances_reject_every_stale_mutation_without_writes() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        let fresh = r
            .placement("io.example.test", "create", Some("medium"))
            .unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        r.placement("io.example.test", "add", None).unwrap();
        for id in ["io.example.test", fresh.as_str()] {
            r.remove_instance(id).unwrap();
            let before = fs::read(r.state.join("layout.json")).unwrap();
            for (op, value) in [
                ("hide", None),
                ("save", Some(r#"{"revision":0,"settings":{}}"#)),
                ("configure", Some("{}")),
                ("workspace", Some("all")),
                (
                    "place",
                    Some(r#"{"column":0,"row":0,"monitor":"DP-1","size":"medium"}"#),
                ),
                ("duplicate", None),
            ] {
                assert!(r.placement(id, op, value).is_err(), "{op}");
                assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
            }
            assert!(r.request_edit(id).is_err());
        }
    }
    #[test]
    fn installed_helpers_keep_only_owner_execute_through_update_and_rollback() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        fs::copy("/usr/bin/true", src.join("helper")).unwrap();
        fs::set_permissions(src.join("helper"), fs::Permissions::from_mode(0o6755)).unwrap();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        for phase in 0..3 {
            if phase == 1 {
                r.deploy(&src, true).unwrap();
            }
            if phase == 2 {
                r.rollback("io.example.test").unwrap();
            }
            let path = r.source(&r.layout().unwrap(), "io.example.test").unwrap();
            assert_eq!(
                fs::metadata(path.join("helper"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                0o700
            );
            assert_eq!(
                fs::metadata(path.join("View.qml"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                0o600
            );
        }
    }
    #[test]
    fn api_three_validates_required_features_and_rejects_future_contracts() {
        let t = Temp::new();
        let src = t.0.join("source");
        sdk::scaffold(&src, "io.example.api", "API", true).unwrap();
        let mut m = read_json(&src.join("widget.json"), 16384).unwrap();
        assert_eq!(m["coreApi"], 3);
        validate(&src).unwrap();
        m["requires"] = json!(["unknown-feature"]);
        atomic_json(&src.join("widget.json"), &m).unwrap();
        assert!(validate(&src).is_err());
        m["requires"] = json!([]);
        m["coreApi"] = json!(4);
        atomic_json(&src.join("widget.json"), &m).unwrap();
        assert!(validate(&src).is_err());
    }
    #[test]
    fn deployment_phase_failures_keep_matching_code_and_settings() {
        for boundary in ["staged", "before-commit", "committed"] {
            let t = Temp::new();
            let src = t.0.join("source");
            fixture(&src);
            let r = Registry {
                data: t.0.join("data"),
                state: t.0.join("state"),
            };
            r.install(&src).unwrap();
            r.placement("io.example.test", "add", None).unwrap();
            let old = r.layout().unwrap();
            let mut m = manifest(&src).unwrap();
            m["version"] = json!("0.2.0");
            m["settingsVersion"] = json!(2);
            m["defaults"] = json!({"title":"new"});
            m["migrations"] = json!([{"from":1,"to":2,"defaults":{"title":"new"}}]);
            atomic_json(&src.join("widget.json"), &m).unwrap();
            assert!(r
                .deploy_observing(&src, true, |phase| if phase == boundary {
                    Err("injected I/O failure".into())
                } else {
                    Ok(())
                })
                .is_err());
            let after = r.layout().unwrap();
            if boundary == "committed" {
                assert_eq!(after["placements"]["io.example.test"]["settingsVersion"], 2);
                assert_eq!(
                    manifest(&r.source(&after, "io.example.test").unwrap()).unwrap()["version"],
                    "0.2.0"
                );
            } else {
                assert_eq!(after, old);
            }
        }
    }
    #[test]
    fn export_restore_is_atomic_and_never_recreates_removed_instances() {
        let t = Temp::new();
        let src = t.0.join("source");
        fixture(&src);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        let export = r.export_settings("io.example.test").unwrap();
        let file = Path::new(export["exported"].as_str().unwrap());
        assert_eq!(
            fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        r.placement("io.example.test", "configure", Some(r#"{"changed":true}"#))
            .unwrap();
        r.restore_settings("io.example.test", file).unwrap();
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.test"]["settings"],
            json!({})
        );
        r.remove_instance("io.example.test").unwrap();
        let before = r.layout().unwrap();
        assert!(r.restore_settings("io.example.test", file).is_err());
        assert_eq!(before, r.layout().unwrap());
    }
    #[test]
    fn manager_can_recover_customised_unplaced_large_instance() {
        let t = Temp::new();
        let src = t.0.join("source");
        sdk::scaffold(&src, "io.example.test", "Test", true).unwrap();
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&src).unwrap();
        let id = r
            .placement("io.example.test", "create", Some("large"))
            .unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        r.placement(
            &id,
            "configure",
            Some(r#"{"title":"Mine","note":"Keep this"}"#),
        )
        .unwrap();
        let mut desktop = grid::tests::desktop();
        desktop["grids"]["DP-1"]["columns"] = json!(1);
        desktop["grids"]["DP-1"]["rows"] = json!(1);
        let before = r.layout().unwrap();
        assert!(grid::resolve(&before["placements"], &desktop)[&id].is_null());
        r.placement_using(
            &id,
            "recover-placement",
            Some(r#"{"size":"small","monitor":""}"#),
            || desktop.clone(),
        )
        .unwrap();
        let after = r.layout().unwrap();
        assert_eq!(
            before["placements"][&id]["settings"],
            after["placements"][&id]["settings"]
        );
        assert_eq!(
            before["placements"][&id]["revision"],
            after["placements"][&id]["revision"]
        );
        assert!(grid::resolve(&after["placements"], &desktop)[&id].is_object());
        assert!(r
            .placement_using(
                &id,
                "recover-placement",
                Some(r#"{"size":"large","monitor":""}"#),
                || desktop.clone()
            )
            .is_err());
        assert_eq!(after, r.layout().unwrap());
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
        r.placement("io.example.test", "add", None).unwrap();
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
        r.placement("io.example.test", "add", None).unwrap();
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
    fn visibility_toggle_preserves_instances_and_manager() {
        let t = Temp::new();
        let source = t.0.join("source");
        fixture(&source);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&source).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r.placement("io.example.test", "hide", None).unwrap();
        r.control("manage").unwrap();
        r.control("arrange").unwrap();
        let before = r.layout().unwrap()["placements"].clone();
        r.control("toggle").unwrap();
        let state = r.layout().unwrap();
        assert_eq!(state["runtime"]["shown"], false);
        assert_eq!(state["runtime"]["editing"], false);
        assert_eq!(state["runtime"]["managerOpen"], true);
        assert_eq!(state["runtime"]["revealUntil"], 0);
        r.control("toggle").unwrap();
        assert_eq!(r.layout().unwrap()["runtime"]["shown"], true);
        assert_eq!(r.layout().unwrap()["placements"], before);
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
    #[cfg(feature = "experimental-reveal")]
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
        assert!(r.request_edit("io.example.test").is_err());
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
    fn review_registry(t: &Temp) -> Registry {
        let source = t.0.join("source");
        fixture(&source);
        let r = Registry {
            data: t.0.join("data"),
            state: t.0.join("state"),
        };
        r.install(&source).unwrap();
        r.placement("io.example.test", "add", None).unwrap();
        r
    }
    #[test]
    fn removed_legacy_lifetime_cannot_target_its_replacement() {
        let t = Temp::new();
        let r = review_registry(&t);
        let sibling = r.placement("io.example.test", "duplicate", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let sibling_before = r.layout().unwrap()["placements"][&sibling].clone();
        r.remove_instance("io.example.test").unwrap();
        let next = r.placement("io.example.test", "add", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_ne!(next, "io.example.test");
        let before = fs::read(r.state.join("layout.json")).unwrap();
        for (op, value) in [
            ("save", Some(r#"{"revision":0,"settings":{"old":true}}"#)),
            ("hide", None),
            ("show", None),
            ("workspace", Some("all")),
            (
                "place",
                Some(r#"{"column":0,"row":0,"monitor":"DP-1","size":"medium"}"#),
            ),
        ] {
            assert!(r.placement("io.example.test", op, value).is_err(), "{op}");
            assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        }
        r.request_edit(&next).unwrap();
        r.finish_edit("io.example.test", "1").unwrap();
        assert_eq!(r.layout().unwrap()["runtime"]["edit"]["instance"], next);
        r.clear_dead_editor(Some("io.example.test")).unwrap();
        assert_eq!(r.layout().unwrap()["placements"][&sibling], sibling_before);
        r.placement(
            &next,
            "save",
            Some(r#"{"revision":0,"settings":{"new":true}}"#),
        )
        .unwrap();
        r.uninstall("io.example.test", "delete").unwrap();
        r.install(&t.0.join("source")).unwrap();
        assert_ne!(
            r.placement("io.example.test", "add", None).unwrap()["updated"],
            "io.example.test"
        );
    }
    #[test]
    fn old_registry_without_tombstones_retires_absent_aliases() {
        let t = Temp::new();
        let r = review_registry(&t);
        r.remove_instance("io.example.test").unwrap();
        let mut v = r.layout().unwrap();
        v.as_object_mut().unwrap().remove("retiredLegacyIds");
        atomic_json(&r.state.join("layout.json"), &v).unwrap();
        assert_ne!(
            r.placement("io.example.test", "add", None).unwrap()["updated"],
            "io.example.test"
        );
    }
    #[test]
    fn manager_writes_wait_for_short_locks_but_not_uncertain_results() {
        let t = Temp::new();
        let r = review_registry(&t);
        let lock = r.lock().unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(80));
            drop(lock);
        });
        retry_busy(|| r.placement("io.example.test", "hide", None)).unwrap();
        release.join().unwrap();
        let mut calls = 0;
        assert!(retry_busy(|| {
            calls += 1;
            Err("Unknown response; write may have completed".into())
        })
        .is_err());
        assert_eq!(calls, 1);
    }
    #[test]
    fn recovery_keeps_a_valid_preferred_cell_without_a_commit() {
        let t = Temp::new();
        let r = review_registry(&t);
        r.placement_using(
            "io.example.test",
            "place",
            Some(r#"{"column":2,"row":1,"monitor":"DP-1","size":"medium"}"#),
            grid::tests::desktop,
        )
        .unwrap();
        let before = fs::read(r.state.join("layout.json")).unwrap();
        let ack = r
            .placement_using(
                "io.example.test",
                "recover-placement",
                Some(r#"{"size":"medium","monitor":"DP-1"}"#),
                grid::tests::desktop,
            )
            .unwrap();
        assert_eq!(ack["placement"]["cell"], json!({"column":2,"row":1}));
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
    }
    #[test]
    fn corrupt_placement_isolated_and_original_quarantined_before_repair() {
        let t = Temp::new();
        let r = review_registry(&t);
        let valid = r.placement("io.example.test", "duplicate", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut v = r.layout().unwrap();
        v["placements"]["io.example.test"]["x"] = json!(-5);
        atomic_json(&r.state.join("layout.json"), &v).unwrap();
        let before = fs::read(r.state.join("layout.json")).unwrap();
        let snapshot = r.snapshot().unwrap();
        assert_eq!(snapshot["repairRequired"], true);
        assert!(snapshot["installed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["instanceId"] == valid));
        assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        assert!(r.placement(&valid, "hide", None).is_err());
        let repaired = r.repair().unwrap();
        assert_eq!(
            fs::read(repaired["backup"].as_str().unwrap()).unwrap(),
            before
        );
        assert!(r.layout().unwrap()["placements"]["io.example.test"].is_null());
        assert!(r.layout().unwrap()["placements"][&valid].is_object());
    }
    #[test]
    fn malformed_runtime_metadata_is_reported_without_panicking_or_writing() {
        let t = Temp::new();
        let r = review_registry(&t);
        let original = r.layout().unwrap();
        for runtime in [
            json!("bad"),
            json!([]),
            json!({"edit":7}),
            json!({"packageControls":{"io.example.test":false}}),
        ] {
            let mut v = original.clone();
            v["runtime"] = runtime;
            atomic_json(&r.state.join("layout.json"), &v).unwrap();
            let before = fs::read(r.state.join("layout.json")).unwrap();
            assert!(r.layout().unwrap_err().contains("runtime metadata"));
            assert_eq!(r.snapshot().unwrap()["repairRequired"], true);
            assert_eq!(fs::read(r.state.join("layout.json")).unwrap(), before);
        }
    }
    #[test]
    fn restore_retains_atomic_validation_and_skips_removed_instances() {
        let t = Temp::new();
        let r = review_registry(&t);
        let other = r.placement("io.example.test", "duplicate", None).unwrap()["updated"]
            .as_str()
            .unwrap()
            .to_owned();
        let export = r.export_settings("io.example.test").unwrap();
        let file = Path::new(export["exported"].as_str().unwrap());
        r.remove_instance(&other).unwrap();
        r.placement("io.example.test", "configure", Some(r#"{"changed":true}"#))
            .unwrap();
        let restored = r.restore_settings("io.example.test", file).unwrap();
        assert_eq!(restored["count"], 1);
        assert_eq!(restored["skippedRemovedInstances"], json!([other]));
        assert_eq!(
            r.layout().unwrap()["placements"]["io.example.test"]["settings"],
            json!({})
        );
        assert!(r
            .restore_settings("io.example.test", Path::new("/etc/hostname"))
            .unwrap_err()
            .contains("settings export"));
    }
    #[test]
    fn dead_editor_clears_only_its_package_and_old_close_cannot_close_new_editor() {
        let t = Temp::new();
        let r = review_registry(&t);
        r.request_edit("io.example.test").unwrap();
        let serial = r.layout().unwrap()["runtime"]["edit"]["serial"].to_string();
        r.clear_dead_editor(Some("io.other.package")).unwrap();
        assert!(!r.layout().unwrap()["runtime"]["edit"].is_null());
        r.clear_dead_editor(Some("io.example.test")).unwrap();
        assert!(r.layout().unwrap()["runtime"]["edit"].is_null());
        r.request_edit("io.example.test").unwrap();
        r.finish_edit("io.example.test", &serial).unwrap();
        assert!(!r.layout().unwrap()["runtime"]["edit"].is_null());
    }
    #[test]
    #[cfg(not(feature = "experimental-reveal"))]
    fn normal_build_cannot_enable_reveal() {
        let t = Temp::new();
        let r = review_registry(&t);
        assert!(r.control("reveal").unwrap_err().contains("experimental"));
        assert_ne!(r.snapshot().unwrap()["runtime"]["revealing"], true);
    }
}
