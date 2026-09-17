//! systemd owns the cgroup hierarchy. Widget services are siblings of Core.
use super::*;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub const MEMORY: u64 = 256 * 1024 * 1024;
pub const TASKS: u64 = 64;

pub fn service_args(unit: &str) -> Result<Vec<String>> {
    if !unit.starts_with("omarchy-widget-island-")
        || !unit.ends_with(".service")
        || unit.len() > 200
        || !unit
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
    {
        return Err("Invalid island unit name".into());
    }
    Ok(vec![
        "--user".into(),
        "--quiet".into(),
        "--wait".into(),
        "--collect".into(),
        "--service-type=exec".into(),
        "--expand-environment=no".into(),
        format!("--unit={unit}"),
        "--property=Slice=app.slice".into(),
        "--property=BindsTo=omarchy-widget-host.service".into(),
        "--property=After=omarchy-widget-host.service".into(),
        format!("--property=MemoryMax={MEMORY}"),
        "--property=MemorySwapMax=0".into(),
        "--property=CPUQuota=25%".into(),
        format!("--property=TasksMax={TASKS}"),
        "--property=OOMPolicy=kill".into(),
        "--property=KillMode=control-group".into(),
        "--property=TimeoutStopSec=3s".into(),
        "--property=NoNewPrivileges=yes".into(),
        "--property=Restart=no".into(),
        "--property=CPUAccounting=yes".into(),
        "--property=MemoryAccounting=yes".into(),
        "--property=TasksAccounting=yes".into(),
        "--".into(),
    ])
}

pub fn preflight() -> Result<Value> {
    let unit = format!("omarchy-widget-island-check-{}.service", std::process::id());
    let mut args = service_args(&unit)?;
    // Preflight must not start or depend on an installed Core service.
    args.retain(|s| {
        !s.starts_with("--property=BindsTo=")
            && !s.starts_with("--property=After=")
    });
    let status = Command::new("/usr/bin/systemd-run")
        .args(args)
        .arg(env::current_exe().map_err(err)?)
        .arg("resource-check")
        .stdin(Stdio::null())
        .status()
        .map_err(err)?;
    if !status.success() {
        return Err(
            "User-service resource preflight failed; CPU, memory and pids controllers are required"
                .into(),
        );
    }
    Ok(json!(true))
}

fn validate_values(memory: &str, swap: &str, tasks: &str, cpu: &str, oom: &str) -> Result<Value> {
    let numeric = |s: &str| s.trim().parse::<u64>().map_err(err);
    let memory = numeric(memory)?;
    let tasks = numeric(tasks)?;
    let fields: Vec<_> = cpu.split_whitespace().collect();
    if fields.len() != 2 {
        return Err("Missing CPU controller".into());
    }
    let quota = numeric(fields[0])?;
    let period = numeric(fields[1])?;
    if memory == 0
        || memory > MEMORY
        || tasks == 0
        || tasks > TASKS
        || numeric(swap)? != 0
        || quota == 0
        || period == 0
        || (quota as u128) * 4 > period as u128
        || numeric(oom)? != 1
    {
        return Err("Required package resource limits are not enforced".into());
    }
    Ok(
        json!({"memoryMax":memory,"memorySwapMax":0,"tasksMax":tasks,"cpuQuota":quota,"cpuPeriod":period,"oomGroup":true}),
    )
}

pub fn verify() -> Result<Value> {
    let membership = fs::read_to_string("/proc/self/cgroup").map_err(err)?;
    let relative = membership
        .lines()
        .find_map(|s| s.strip_prefix("0::/"))
        .filter(|s| safe_relative(s))
        .ok_or("A unified cgroup v2 service is required")?;
    let group = Path::new("/sys/fs/cgroup").join(relative);
    let read = |name: &str| fs::read_to_string(group.join(name)).map_err(err);
    validate_values(
        &read("memory.max")?,
        &read("memory.swap.max")?,
        &read("pids.max")?,
        &read("cpu.max")?,
        &read("memory.oom.group")?,
    )
}

pub fn stop(unit: &str) {
    // Stopping the unit kills proxy, policy hooks, sandbox and all descendants.
    // Killing systemd-run alone does not stop its transient service.
    let status = Command::new("/usr/bin/timeout")
        .args(["5s", "/usr/bin/systemctl", "--user", "stop", unit])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !status.is_ok_and(|s| s.success()) {
        eprintln!("Could not confirm island unit stopped: {unit}");
    }
}

struct Children(Vec<Child>);
impl Drop for Children {
    fn drop(&mut self) {
        for child in self.0.iter_mut().rev() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn worker(config: &Path, source: &Path, directory: &Path) -> Result<Value> {
    // Fail closed before any package code or protocol parser is started.
    verify().map_err(|e| format!("Package resource preflight failed: {e}"))?;
    let reveal_until = reveal::lease(&directory.join("reveal-lease"));
    let deadline = std::time::Instant::now()
        + Duration::from_millis(
            reveal_until
                .saturating_sub(reveal::now())
                .min(reveal::MAX_MS),
        );
    let mut children = Children(Vec::new());
    children.0.push(
        Command::new(config.join("bin/wl-mitm"))
            .arg(directory.join("wayland.toml"))
            .env("TOKIO_WORKER_THREADS", "1")
            .env(
                "OMARCHY_WIDGET_REVEAL_LEASE",
                directory.join("reveal-lease"),
            )
            .stdin(Stdio::null())
            .spawn()
            .map_err(err)?,
    );
    let display = directory.join("wayland");
    let mut ready = false;
    for _ in 0..100 {
        if children.0[0].try_wait().map_err(err)?.is_some() {
            return Err("Wayland filter exited before ready".into());
        }
        if fs::symlink_metadata(&display)
            .is_ok_and(|m| std::os::unix::fs::FileTypeExt::is_socket(&m.file_type()))
        {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if !ready {
        return Err("Wayland filter readiness timed out".into());
    }
    children.0.push(
        Command::new("/usr/bin/bash")
            .arg(config.join("sandbox-launch"))
            .arg("runner")
            .arg(source)
            .arg(directory.join("broker"))
            .arg(display)
            .env(
                "OMARCHY_WIDGET_REVEAL",
                if reveal_until > 0 { "1" } else { "0" },
            )
            .stdin(Stdio::null())
            .spawn()
            .map_err(err)?,
    );
    loop {
        if reveal_until > 0 && std::time::Instant::now() >= deadline {
            return Err("Reveal lease ended".into());
        }
        for child in &mut children.0 {
            if let Some(status) = child.try_wait().map_err(err)? {
                return Err(format!("Island child exited: {status}"));
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_is_per_service_and_bound_to_core() {
        let args = service_args("omarchy-widget-island-test.service").unwrap();
        for property in [
            "--property=MemoryMax=268435456",
            "--property=MemorySwapMax=0",
            "--property=CPUQuota=25%",
            "--property=TasksMax=64",
            "--property=OOMPolicy=kill",
            "--property=BindsTo=omarchy-widget-host.service",
            "--property=After=omarchy-widget-host.service",
            "--property=KillMode=control-group",
            "--expand-environment=no",
        ] {
            assert!(args.iter().any(|a| a == property));
        }
        assert!(!args.iter().any(|arg| arg.starts_with("--property=PartOf=")));
        assert!(!args
            .iter()
            .any(|a| a == "--scope" || a.contains("Delegate=")));
        assert!(service_args("other.service").is_err());
        assert!(service_args("omarchy-widget-island-$(id).service").is_err());
    }
    #[test]
    fn absent_unlimited_or_weaker_controllers_fail_closed() {
        assert!(validate_values("268435456", "0", "64", "25000 100000", "1").is_ok());
        assert!(validate_values("max", "0", "64", "25000 100000", "1").is_err());
        assert!(validate_values("536870912", "0", "64", "25000 100000", "1").is_err());
        assert!(validate_values("268435456", "max", "64", "25000 100000", "1").is_err());
        assert!(validate_values("268435456", "0", "max", "25000 100000", "1").is_err());
        assert!(validate_values("268435456", "0", "64", "max 100000", "1").is_err());
        assert!(validate_values("268435456", "0", "64", "50000 100000", "1").is_err());
        assert!(validate_values("268435456", "0", "64", "25000 100000", "0").is_err());
    }
}
