//! A reveal lease never grants permanent layer authority.
use super::*;
pub const MAX_MS: u64 = 30_000;
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn active(until: u64, time: u64) -> bool {
    until > time && until - time <= MAX_MS
}
pub fn lease(path: &Path) -> u64 {
    fs::read(path)
        .ok()
        .filter(|b| b.len() <= 32)
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
pub fn permitted() -> bool {
    env::var_os("OMARCHY_WIDGET_REVEAL_LEASE").is_some_and(|p| active(lease(Path::new(&p)), now()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_lease() {
        assert!(!active(0, 100));
        assert!(!active(100, 100));
        assert!(active(101, 100));
        assert!(active(30100, 100));
        assert!(!active(30101, 100));
        assert!(!active(u64::MAX, 100));
    }
}
