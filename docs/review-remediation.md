# Review remediation — 16 September 2026

This follow-up is based on Core `c84470723841da248b37f655ada16fae0779a100`
and World Clock `5cd7b0894376eb16d070fa244f4aeaea1448dd58`. It leaves the
reviewed draft stack intact and adds a cumulative fix branch above it.

| Finding | Implemented correction | Regression evidence |
| --- | --- | --- |
| F1 | API 3 manifests, required-feature rejection, explicit runtime evidence matrix | Current/old-host compatibility test; unknown/future contracts rejected |
| F2 | Only Add/Create allocate; broker authority, source generation and restart serial checked under mutation lock | Every stale mutation after legacy/fresh removal; restart/lock authority tests |
| F3/F4 | No allocation on inactive/throttled misses; per-package admission; bounded LRU; BOOTTIME freshness/backoff | Inactive flood, coordinate churn, sharing/dedup, stale failure and separate wall/elapsed clock tests |
| F5 | Sanitize installed helpers to 0700; data 0600 | Native compiled helper install/update/rollback; sandbox CI executes a helper |
| F6 | Documented display save with expected revision; acknowledgement cannot close another draft | Countdown Start/Pause/Resume and Clock action/draft conflict integration |
| F7 | Core owns API 3 gear; Clock removes duplicate; arbitrary-package sandbox capture command | Real frame gear keyboard activation, repeated settings, starter/author previews |
| F8/F9 | Manager resize/monitor recovery, preferred/fallback status, control-level scrolling and keyboard focus | Tiny-grid state-preserving recovery; mouse/Tab/Space/Return at 100/125/150/200% |
| E3 | Settings export/restore, package-scoped failed-content stop, phase fault hooks, explicit manual recovery | Staging/precommit/postcommit injected failures; restore/removal refusal; stale-editor and broker failure-state tests |

## Reproduced here

Linux x86_64; Rust 1.98.1; Python 3.12; PySide6/Qt 6.11.2 offscreen.
`cargo fmt --all -- --check` and all-target clippy with warnings denied pass.
The Rust suite has 55 passing tests and two environment failures: Unix socket
creation is denied in `broker_scopes_reads_writes_and_generations` and
`socket_round_trip`. Neither test was removed, skipped or weakened in CI.

Portable Qt/runtime/SDK, countdown, current Clock, manager at four scales,
compiled-helper, old-host compatibility, installer/keybinding, grid/workspace
and static sandbox checks pass. These adapt windows/transport and do not run an
assembled Omarchy session. Full CI results belong to the exact remote commit.

## Remaining acceptance gates

E1/E2 remain open: real Hyprland focus/lock/reveal/topology, physical suspend and
docking, first-load failure presentation, reboot and measured idle CPU/memory/
wakeups. Reveal stays an optional prototype with no new default shortcut. No
performance optimisation or budget is claimed without measurements. The review's
live matrix in `desktop-checks.md` and `reveal.md` still applies.

The arbitrary-package capture wrapper fails closed when system PySide6 or
Bubblewrap is unavailable; its actual namespace execution requires a suitable
host. Native helper execution in the package sandbox is retained in the live
sandbox CI probe. No live weather-provider, power-loss/disk-full, complete
backup/restore or first ordinary-user installation acceptance is claimed here.

See [distribution-and-backup.md](distribution-and-backup.md) for trusted-source
provenance and consistent local backup guidance. There is no catalogue, signing
service, background timer alarm, automatic destructive rollback or cross-machine
import. This is a development handoff; merge/release readiness is a separate gate.
