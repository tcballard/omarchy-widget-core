//! Logical-cell placement, shared by registry writes and every renderer snapshot.
use super::*;
const CELL: f64 = 192.0;
pub fn span(size: &str) -> (u64, u64) {
    match size {
        "small" => (1, 1),
        "large" => (2, 2),
        _ => (2, 1),
    }
}
fn number(v: &Value) -> Result<f64> {
    v.as_f64()
        .filter(|n| n.is_finite() && (0.0..=32768.0).contains(n))
        .ok_or("Invalid desktop geometry".into())
}
// CSS order: top, right, bottom, left. Hyprland also emits custom gap strings.
pub fn edges(v: &Value) -> Result<[f64; 4]> {
    let values: Vec<f64> = if let Some(s) = v.as_str() {
        s.split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<f64>().map_err(err))
            .collect::<Result<_>>()?
    } else if let Some(a) = v.as_array() {
        a.iter().map(number).collect::<Result<_>>()?
    } else {
        vec![number(v)?]
    };
    if !values
        .iter()
        .all(|n| n.is_finite() && (0.0..=32768.0).contains(n))
    {
        return Err("Invalid desktop gaps".into());
    }
    match values.as_slice() {
        [a] => Ok([*a; 4]),
        [a, b] => Ok([*a, *b, *a, *b]),
        [a, b, c] => Ok([*a, *b, *c, *b]),
        [a, b, c, d] => Ok([*a, *b, *c, *d]),
        _ => Err("Invalid desktop gaps".into()),
    }
}
pub fn screens(raw: &Value, inside: [f64; 4], outside: [f64; 4]) -> Result<Value> {
    let mut out = serde_json::Map::new();
    for m in raw
        .as_array()
        .filter(|a| a.len() <= 64)
        .ok_or("Invalid monitors")?
    {
        if m["disabled"] == true {
            continue;
        }
        let name = m["name"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 120)
            .ok_or("Invalid monitor")?;
        if out.contains_key(name) {
            return Err("Duplicate monitor name".into());
        }
        let scale = number(&m["scale"])?;
        if !(0.25..=8.0).contains(&scale) {
            return Err("Invalid monitor scale".into());
        }
        let (mut width, mut height) = (number(&m["width"])? / scale, number(&m["height"])? / scale);
        let transform = m["transform"]
            .as_u64()
            .filter(|n| *n <= 7)
            .ok_or("Invalid transform")?;
        if transform % 2 == 1 {
            std::mem::swap(&mut width, &mut height);
        }
        let reserved = m["reserved"]
            .as_array()
            .filter(|a| a.len() == 4)
            .ok_or("Invalid reserved area")?;
        let r = reserved.iter().map(number).collect::<Result<Vec<_>>>()?; // left, top, right, bottom; already logical
        let x = r[0] + outside[3];
        let y = r[1] + outside[0];
        let w = (width - x - r[2] - outside[1]).max(0.0);
        let h = (height - y - r[3] - outside[2]).max(0.0);
        let gx = inside[1] + inside[3];
        let gy = inside[0] + inside[2];
        out.insert(
            name.into(),
            json!({"x":x,"y":y,"cell":CELL,"gapX":gx,"gapY":gy,
            "columns":((w+gx)/(CELL+gx)).floor() as u64,"rows":((h+gy)/(CELL+gy)).floor() as u64}),
        );
    }
    Ok(Value::Object(out))
}
pub fn valid_preference(p: &Value) -> bool {
    p["cell"].is_null()
        || ["column", "row"]
            .iter()
            .all(|k| p["cell"][k].as_u64().is_some_and(|n| n <= 10000))
}
pub fn migrate(placements: &mut Value, desktop: &Value) -> bool {
    let mut changed = false;
    for p in placements.as_object_mut().unwrap().values_mut() {
        if !p["cell"].is_null() {
            if p["monitor"] == "" {
                if let Some(name) = desktop["grids"].as_object().and_then(|g| g.keys().next()) {
                    p["monitor"] = json!(name);
                    changed = true;
                }
            }
            continue;
        }
        let preferred = p["monitor"].as_str().unwrap_or("").to_owned();
        let Some((name, g)) = desktop["grids"].as_object().and_then(|gs| {
            if preferred.is_empty() {
                gs.iter().next()
            } else {
                gs.get_key_value(&preferred)
            }
        }) else {
            continue;
        };
        let col = ((p["x"].as_f64().unwrap_or(0.0) - g["x"].as_f64().unwrap())
            / (CELL + g["gapX"].as_f64().unwrap()))
        .round()
        .max(0.0) as u64;
        let row = ((p["y"].as_f64().unwrap_or(0.0) - g["y"].as_f64().unwrap())
            / (CELL + g["gapY"].as_f64().unwrap()))
        .round()
        .max(0.0) as u64;
        p["cell"] = json!({"column":col,"row":row});
        if preferred.is_empty() {
            p["monitor"] = json!(name);
        }
        changed = true;
    }
    changed
}
fn candidate(p: &Value, monitor: &str, column: u64, row: u64, desktop: &Value) -> Option<Value> {
    let g = &desktop["grids"][monitor];
    let (w, h) = span(p["size"].as_str().unwrap_or("medium"));
    if column + w > g["columns"].as_u64()? || row + h > g["rows"].as_u64()? {
        return None;
    }
    let gx = g["gapX"].as_f64()?;
    let gy = g["gapY"].as_f64()?;
    Some(
        json!({"monitor":monitor,"column":column,"row":row,"columns":w,"rows":h,
        "workspace":p["workspace"],"x":g["x"].as_f64()?+column as f64*(CELL+gx),
        "y":g["y"].as_f64()?+row as f64*(CELL+gy),"width":w as f64*CELL+(w-1) as f64*gx,
        "height":h as f64*CELL+(h-1) as f64*gy}),
    )
}
pub fn overlap(a: &Value, b: &Value) -> bool {
    a["monitor"] == b["monitor"]
        && (a["workspace"].is_null()
            || b["workspace"].is_null()
            || a["workspace"] == b["workspace"])
        && a["column"].as_u64().unwrap()
            < b["column"].as_u64().unwrap() + b["columns"].as_u64().unwrap()
        && b["column"].as_u64().unwrap()
            < a["column"].as_u64().unwrap() + a["columns"].as_u64().unwrap()
        && a["row"].as_u64().unwrap() < b["row"].as_u64().unwrap() + b["rows"].as_u64().unwrap()
        && b["row"].as_u64().unwrap() < a["row"].as_u64().unwrap() + a["rows"].as_u64().unwrap()
}
pub fn resolve(placements: &Value, desktop: &Value) -> Value {
    let mut result = serde_json::Map::new();
    if desktop["available"] == false {
        return Value::Object(result);
    }
    let mut entries: Vec<_> = placements.as_object().unwrap().iter().collect();
    entries.sort_by_key(|(id, p)| (p["activationOrder"].as_u64().unwrap_or(0), *id));
    // Reserve all viable preferences before allocating any temporary fallback.
    for (id, p) in entries.iter().filter(|(_, p)| p["enabled"] == true) {
        let c = candidate(
            p,
            p["monitor"].as_str().unwrap_or(""),
            p["cell"]["column"].as_u64().unwrap_or(0),
            p["cell"]["row"].as_u64().unwrap_or(0),
            desktop,
        );
        if let Some(c) = c {
            if !result.values().any(|v| overlap(&c, v)) {
                result.insert((*id).clone(), c);
            }
        }
    }
    for (id, p) in entries.iter().filter(|(_, p)| p["enabled"] == true) {
        if result.contains_key(*id) {
            continue;
        }
        if let Some(gs) = desktop["grids"].as_object() {
            let mut monitors: Vec<_> = gs.keys().collect();
            monitors.sort_by_key(|name| (p["monitor"].as_str() != Some(name.as_str()), *name));
            'search: for name in monitors {
                let g = &gs[name];
                for row in 0..g["rows"].as_u64().unwrap_or(0) {
                    for col in 0..g["columns"].as_u64().unwrap_or(0) {
                        if let Some(c) = candidate(p, name, col, row, desktop) {
                            if !result.values().any(|v| overlap(&c, v)) {
                                result.insert((*id).clone(), c);
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
    }
    Value::Object(result)
}
// Validate against the current effective positions, not a reordered hypothetical layout.
pub fn validate_change(id: &str, p: &Value, current: &Value, desktop: &Value) -> Result<Value> {
    let c = candidate(
        p,
        p["monitor"].as_str().unwrap_or(""),
        p["cell"]["column"].as_u64().ok_or("Missing column")?,
        p["cell"]["row"].as_u64().ok_or("Missing row")?,
        desktop,
    )
    .ok_or("That footprint does not fit an available screen")?;
    if current
        .as_object()
        .unwrap()
        .iter()
        .any(|(other, v)| other != id && overlap(&c, v))
    {
        return Err("Those cells are occupied; placement unchanged".into());
    }
    Ok(c)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    pub fn desktop() -> Value {
        json!({"available":true,"grids":{"DP-1":{"x":10.0,"y":42.0,"cell":192.0,"gapX":10.0,"gapY":10.0,"columns":4,"rows":3}},"frame":{"borderWidth":2,"radius":0}})
    }
    fn p(monitor: &str, col: u64, row: u64, size: &str, workspace: Value) -> Value {
        json!({"enabled":true,"monitor":monitor,"cell":{"column":col,"row":row},"size":size,"workspace":workspace,"settings":{"retained":true}})
    }
    #[test]
    fn gaps_reserved_rotation_and_fractional_scale() {
        assert_eq!(edges(&json!("1 2 3 4")).unwrap(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(edges(&json!("5, 10")).unwrap(), [5.0, 10.0, 5.0, 10.0]);
        for bad in [json!(-1), json!("NaN"), json!([]), json!("1 2 3 4 5")] {
            assert!(edges(&bad).is_err());
        }
        let raw = json!([{"name":"DP-1","width":1920,"height":1200,"scale":1.25,"transform":0,"reserved":[0,32,0,0]}]);
        let g = screens(&raw, [5.0; 4], [10.0, 20.0, 30.0, 40.0]).unwrap();
        assert_eq!(g["DP-1"]["x"], 40.0);
        assert_eq!(g["DP-1"]["y"], 42.0);
        assert_eq!(g["DP-1"]["columns"], 7);
        assert_eq!(g["DP-1"]["rows"], 4);
        assert_eq!(g["DP-1"]["gapX"], 10.0);
        let mut rotated = raw.clone();
        rotated[0]["transform"] = json!(1);
        let g = screens(&rotated, [0.0; 4], [0.0; 4]).unwrap();
        assert_eq!(g["DP-1"]["columns"], 5);
        assert_eq!(g["DP-1"]["rows"], 7);
        assert_eq!(g["DP-1"]["y"], 32.0);
        assert_eq!(g["DP-1"]["gapX"], 0.0);
    }
    #[test]
    fn workspace_occupancy_and_atomic_target_validation() {
        let d = desktop();
        let a = p("DP-1", 0, 0, "medium", json!(1));
        let b = p("DP-1", 0, 0, "medium", json!(2));
        let placements = json!({"a":a,"b":b});
        let current = resolve(&placements, &d);
        assert_eq!(current["a"]["column"], 0);
        assert_eq!(current["b"]["column"], 0);
        let mut target = a.clone();
        target["workspace"] = Value::Null;
        assert!(validate_change("a", &target, &current, &d).is_err());
        target["cell"]["column"] = json!(2);
        assert!(validate_change("a", &target, &current, &d).is_ok());
        target["cell"]["column"] = json!(3);
        assert!(validate_change("a", &target, &current, &d).is_err());
    }
    #[test]
    fn full_grid_disable_resize_and_preference_restoration() {
        let mut d = desktop();
        d["grids"]["DP-1"]["columns"] = json!(2);
        d["grids"]["DP-1"]["rows"] = json!(2);
        let mut ps =
            json!({"a":p("DP-1",0,0,"large",Value::Null),"b":p("DP-1",0,0,"small",json!(2))});
        let r = resolve(&ps, &d);
        assert!(r.get("b").is_none());
        assert_eq!(r["a"]["width"], 394.0);
        ps["a"]["enabled"] = json!(false);
        assert!(resolve(&ps, &d).get("b").is_some());
        ps["a"]["enabled"] = json!(true);
        d["grids"]["HDMI-A-1"] = d["grids"]["DP-1"].clone();
        let before = ps.clone();
        let r = resolve(&ps, &d);
        assert_eq!(r["b"]["monitor"], "HDMI-A-1");
        let saved = d["grids"]["DP-1"].take();
        d["grids"].as_object_mut().unwrap().remove("DP-1");
        assert_eq!(resolve(&ps, &d)["a"]["monitor"], "HDMI-A-1");
        d["grids"]["DP-1"] = saved;
        assert_eq!(resolve(&ps, &d)["a"]["monitor"], "DP-1");
        assert_eq!(ps, before);
        ps["a"]["size"] = json!("small");
        let r = resolve(&ps, &d);
        let mut resize = ps["a"].clone();
        resize["size"] = json!("large");
        assert!(validate_change("a", &resize, &r, &d).is_err());
    }
    #[test]
    fn migration_once_and_ricing_keep_cells_and_settings() {
        let mut d = desktop();
        let mut ps = json!({"a":{"enabled":true,"x":412,"y":42,"monitor":"DP-1","size":"medium","settings":{"city":"London"}}});
        assert!(migrate(&mut ps, &d));
        let saved = ps.clone();
        assert_eq!(ps["a"]["cell"]["column"], 2);
        d["grids"]["DP-1"]["gapX"] = json!(0.0);
        assert!(!migrate(&mut ps, &d));
        assert_eq!(ps, saved);
        assert_eq!(resolve(&ps, &d)["a"]["x"], 394.0);
        d["grids"]["DP-1"]["columns"] = json!(2);
        assert_eq!(resolve(&ps, &d)["a"]["column"], 0);
        d["grids"]["DP-1"]["columns"] = json!(4);
        assert_eq!(resolve(&ps, &d)["a"]["column"], 2);
    }
    #[test]
    fn disconnected_legacy_position_waits_for_its_monitor() {
        let mut d = desktop();
        let mut ps = json!({"a":{"enabled":true,"monitor":"HDMI-A-1","x":412,"y":42,"size":"small","settings":{"city":"London"}}});
        let original = ps.clone();
        assert!(!migrate(&mut ps, &d));
        assert_eq!(ps, original);
        assert_eq!(resolve(&ps, &d)["a"]["monitor"], "DP-1");
        d["grids"]["HDMI-A-1"] = d["grids"]["DP-1"].clone();
        assert!(migrate(&mut ps, &d));
        assert_eq!(ps["a"]["cell"]["column"], 2);
        assert_eq!(resolve(&ps, &d)["a"]["monitor"], "HDMI-A-1");
    }
    #[test]
    fn topology_matrix_preserves_preferences_and_valid_bounds() {
        let ps = json!({"a":p("DP-1",2,1,"large",Value::Null),"b":p("HDMI-A-1",0,0,"medium",Value::Null),"c":p("DP-1",0,0,"small",json!(2))});
        let original = ps.clone();
        for scale in [1.0, 1.25, 1.5, 2.0] {
            for transform in 0..8 {
                for connected in [false, true] {
                    let mut raw = json!([{"name":"DP-1","width":1920,"height":1200,"scale":scale,"transform":transform,"reserved":[0,32,0,24]}]);
                    if connected {
                        let mut m = raw[0].clone();
                        m["name"] = json!("HDMI-A-1");
                        raw.as_array_mut().unwrap().push(m);
                    }
                    let mut d =
                        json!({"available":true,"grids":screens(&raw,[5.0;4],[10.0;4]).unwrap()});
                    let resolved = resolve(&ps, &d);
                    let rs: Vec<_> = resolved.as_object().unwrap().values().collect();
                    for (i, r) in rs.iter().enumerate() {
                        let g = &d["grids"][r["monitor"].as_str().unwrap()];
                        assert!(
                            r["column"].as_u64().unwrap() + r["columns"].as_u64().unwrap()
                                <= g["columns"].as_u64().unwrap()
                        );
                        assert!(
                            r["row"].as_u64().unwrap() + r["rows"].as_u64().unwrap()
                                <= g["rows"].as_u64().unwrap()
                        );
                        assert!(rs[i + 1..].iter().all(|other| !overlap(r, other)));
                    }
                    d["available"] = json!(false);
                    assert_eq!(resolve(&ps, &d), json!({}));
                    assert_eq!(ps, original);
                }
            }
        }
    }
}
