//! Public GitHub contribution calendar. No credentials, third-party API or arbitrary URL.
use super::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
static CACHE: OnceLock<Arc<Mutex<public_data::Cache>>> = OnceLock::new();
pub fn username(input: &str) -> Result<String> {
    if input.is_empty()
        || input.len() > 39
        || input.starts_with('-')
        || input.ends_with('-')
        || input.contains("--")
        || !input
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err("Enter a GitHub username (letters, digits and single hyphens)".into());
    }
    Ok(input.to_ascii_lowercase())
}
fn attr<'a>(tag: &'a str, name: &str) -> Result<&'a str> {
    let needle = format!(" {name}=\"");
    let tail = tag
        .strip_prefix(&needle[1..])
        .or_else(|| tag.split_once(&needle).map(|(_, tail)| tail))
        .ok_or("Missing calendar attribute")?;
    tail.split_once('"')
        .map(|(v, _)| v)
        .ok_or("Unclosed calendar attribute".into())
}
// UTC civil day ordinal; validate every date before arithmetic, including leap years.
fn ordinal(date: &str) -> Result<i64> {
    let b = date.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return Err("Invalid calendar date".into());
    }
    let y: i64 = date[..4].parse().map_err(err)?;
    let m: i64 = date[5..7].parse().map_err(err)?;
    let d: i64 = date[8..].parse().map_err(err)?;
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1970..=9999).contains(&y)
        || !(1..=12).contains(&m)
        || d < 1
        || d > lengths[(m - 1) as usize]
    {
        return Err("Invalid calendar date".into());
    }
    let year = y - i64::from(m <= 2);
    let era = year / 400;
    let yo = year - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    Ok(era * 146097 + yo * 365 + yo / 4 - yo / 100 + doy - 719468)
}
pub(crate) fn project(html: &str, login: &str) -> Result<Value> {
    if html.len() > 524288 {
        return Err("Calendar exceeds size limit".into());
    }
    let mut counts = BTreeMap::new();
    for part in html.split("<tool-tip ").skip(1) {
        let (tag, body) = part.split_once('>').ok_or("Invalid calendar tooltip")?;
        let id = attr(tag, "for")?;
        if !id.starts_with("contribution-day-component-") {
            continue;
        }
        let text = body
            .split_once("</tool-tip>")
            .ok_or("Invalid calendar tooltip")?
            .0;
        let first = text
            .split_whitespace()
            .next()
            .ok_or("Missing contribution count")?;
        let count = if first == "No" {
            0
        } else {
            first.replace(',', "").parse::<u64>().map_err(err)?
        };
        if count > 1_000_000 || counts.insert(id.to_owned(), count).is_some() {
            return Err("Invalid contribution count".into());
        }
    }
    let mut days = BTreeMap::new();
    for part in html.split("<td ").skip(1) {
        let tag = part.split_once('>').ok_or("Invalid calendar cell")?.0;
        if !tag.contains(" data-date=") {
            continue;
        }
        let date = attr(tag, "data-date")?;
        let n = ordinal(date)?;
        let level = attr(tag, "data-level")?.parse::<u64>().map_err(err)?;
        let count = *counts
            .get(attr(tag, "id")?)
            .ok_or("Calendar count missing")?;
        if level > 4
            || (level == 0) != (count == 0)
            || days
                .insert(
                    n,
                    json!({"date":date,"count":count,"level":level,"weekday":(n+4).rem_euclid(7)}),
                )
                .is_some()
        {
            return Err("Invalid calendar cell".into());
        }
    }
    if !(365..=366).contains(&days.len()) {
        return Err("Incomplete annual contribution calendar".into());
    }
    let first = *days.keys().next().unwrap();
    let last = *days.keys().next_back().unwrap();
    if last - first + 1 != days.len() as i64 {
        return Err("Contribution calendar has gaps".into());
    }
    let total: u64 = days.values().map(|d| d["count"].as_u64().unwrap()).sum();
    Ok(
        json!({"username":login,"total":total,"days":days.into_values().collect::<Vec<_>>(),"attribution":"GitHub public profile"}),
    )
}
pub fn request(package: &str, login: &str, active: bool) -> Result<Value> {
    let login = username(login)?;
    public_data::request(&CACHE, package, login.clone(), active, move || {
        let bytes = public_data::fetch(public_data::Endpoint::Github(&login))?;
        project(std::str::from_utf8(&bytes).map_err(err)?, &login)
    })
}
pub fn declared(m: &Value) -> bool {
    m["capabilities"]
        .as_array()
        .is_some_and(|a| a.contains(&json!("github")))
}
pub fn authorised(r: &Registry, l: &Value, package: &str) -> bool {
    r.source(l, package)
        .ok()
        .is_some_and(|p| l["packages"][package]["githubGrant"] == json!(p.to_string_lossy()))
}
pub fn grant(r: &Registry, id: &str, choice: &str) -> Result<Value> {
    if !["allow", "deny"].contains(&choice) {
        return Err("Use allow or deny".into());
    }
    let _lock = r.lock()?;
    let mut l = r.layout()?;
    let source = r.source(&l, id)?;
    if !declared(&manifest(&source)?) {
        return Err("Package does not declare github".into());
    }
    if !l["packages"][id].is_object() {
        return Err("Reinstall legacy package before granting github".into());
    }
    l["packages"][id]["githubGrant"] = if choice == "allow" {
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
    fn fixture() -> String {
        let mut html = String::new();
        for m in 1..=12 {
            let length = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1];
            for d in 1..=length {
                let date = format!("2025-{m:02}-{d:02}");
                html.push_str(&format!("<td id=\"contribution-day-component-{m}-{d}\" data-date=\"{date}\" data-level=\"1\"></td><tool-tip id=\"tip\" for=\"contribution-day-component-{m}-{d}\">1 contribution on day.</tool-tip>"));
            }
        }
        html
    }
    #[test]
    fn validates_identity_and_civil_dates() {
        assert_eq!(username("TcBallard").unwrap(), "tcballard");
        for bad in [
            "",
            "--help",
            "a/b",
            "x?url=foo",
            "https://github.com",
            "foo--bar",
            "foo-",
            "a b",
            "<b>",
        ] {
            assert!(username(bad).is_err(), "{bad}");
        }
        assert!(username(&"a".repeat(40)).is_err());
        assert!(ordinal("2024-02-29").is_ok());
        for bad in [
            "2025-02-29",
            "2025-13-01",
            "2025-04-31",
            "2025-00-00",
            "é2025-1-1",
        ] {
            assert!(ordinal(bad).is_err());
        }
        assert_eq!((ordinal("2025-01-05").unwrap() + 4) % 7, 0);
    }
    #[test]
    fn calendar_projects_counts_and_rejects_partial_or_changed_markup() {
        let html = fixture();
        let result = project(&html, "tcballard").unwrap();
        assert_eq!(result["total"], 365);
        assert_eq!(result["days"][0]["weekday"], 3);
        assert_eq!(result["days"][364]["date"], "2025-12-31");
        for bad in [
            String::new(),
            html.replace("data-date=\"2025-01-01\"", "data-date=\"2025-01-02\""),
            html.replace("data-level=\"1\"", "data-level=\"5\""),
            html.replace("1 contribution", "??? contribution"),
            html.replace("data-level=\"1\"", "data-level=\"0\""),
            "x".repeat(524289),
        ] {
            assert!(project(&bad, "tcballard").is_err());
        }
    }
    #[test]
    #[ignore = "Opt-in real fixed endpoint; requires public DNS/HTTPS"]
    fn live_public_calendar() {
        let bytes = public_data::fetch(public_data::Endpoint::Github("tcballard")).unwrap();
        let result = project(std::str::from_utf8(&bytes).unwrap(), "tcballard").unwrap();
        assert!(result["days"].as_array().unwrap().len() >= 365);
        println!("Live public calendar: {} contributions", result["total"]);
    }
    #[test]
    #[ignore = "Opt-in captured HTML parser check; input/output from test-only environment"]
    fn captured_public_calendar() {
        let html = fs::read_to_string(env::var("GITHUB_CALENDAR_TEST_HTML").unwrap()).unwrap();
        let result = project(&html, "tcballard").unwrap();
        if let Ok(path) = env::var("GITHUB_CALENDAR_TEST_JSON") {
            fs::write(path, result.to_string()).unwrap();
        }
        println!(
            "Captured public calendar: {} contributions",
            result["total"]
        );
    }
}
