# Combined widget review remediation

17 September 2026. Follow-up to both independent reviews of Core `a3b90c4`
and World Clock `e65a5fb`. This is development remediation, not desktop acceptance.

| Findings | Correction | Evidence |
| --- | --- | --- |
| Astra F1 / pasted N6 | Retire deleted legacy identities, including upgrade from registries without tombstones. Replacement Add returns a fresh identity. Dedicated Show requires existence. Retire aliases on uninstall/delete as well. | Rust stale Save/Hide/Show/Place/Workspace and editor-close checks, sibling preservation, old-layout migration; manager Hide/Show integration. |
| Astra F2 / N4 | Retain serial-scoped close messages through lock refusal/full queue; acknowledge/reconcile before forgetting. Queue a requested next editor. Clear dead-runner and prior-supervisor registrations. Name the pending instance/package and expose cancellation. | Production QML controller/queue with actual Rust state and deliberately held file lock; queue-full and stale-close regressions. Supervisor liveness integration still requires a desktop. |
| Astra F3 | Keep one-shot content failures pending until acknowledgement or generation expiry. | Production controller/Loader/queue refusal fixture; actual Rust dispatcher checks refusal, eventual disable and rejection after code replacement. Existing real socket test stays enabled in CI. |
| N1 | Shared locks for broker reads; retain exclusive atomic authorization/mutation. Trusted CLI waits up to two seconds only after a pre-dispatch busy refusal. QML retries typed busy/rate-limit refusal with bounded backoff. | 300 manager writes while a broker reader polls repeatedly; no busy failures. Uncertain errors are not replayed. Existing immediate lock-refusal test retained. |
| N2 | Scroll focused buttons synchronously, avoiding a callback on a destroyed delegate. Unchanged snapshots already preserve delegates. | Manager integration repeated ten times at each of 100/125/150/200%; CI additionally repeats 150% three times. |
| N3 | Replace the permanent gear header with a 32-unit corner button. On-demand widget focus and the manager's keyboard Configure route. | Small frame retains 168 logical content units. Countdown reaches the actual gear using Tab and activates with Return. Offscreen evidence only. |
| N5 / density | Preserve a valid preferred cell for recovery and avoid a commit for unchanged recovery. Show recovery size/monitor rows only for enabled unplaced instances. | Rust preference/no-op check; manager workflow at four scales. |
| Astra F4 / N13 | Full inactive preview context with real lifecycle signals, plus inert Process/StdioCollector/IpcHandler capture imports. | A non-starter consumes public identity/state/lifecycle members and verifies no side effects. World Clock renders all three families. CI adds actual Bubblewrap captures of both fixtures. |
| Astra F5 / N8 | Validate runtime/control/edit shapes before indexing. Keep the supervisor alive on read errors. Display valid placements read-only and expose explicit repair. Save exact original bytes privately before committing a quarantine. | Malformed runtime variants and invalid-placement tests prove no read-side writes, valid-instance survival and exact backup preservation. Unparseable/unsupported files require matched backup restoration. |
| N7 | Rolling five-mutation-attempt-per-second runner budget; remove runner permission to enter global arrangement. Repeated finish-arrange is a no-op. | Budget/read tests and actual dispatcher refusal. |
| N9 | Expire pending weather after 30 seconds; mark retryable and permit eviction. Bind completion to a fetch serial. | Abandoned request recovers; an old completion cannot overwrite its retry. |
| N12 | Friendly export-file errors, percent-encoded editor paths, restore matching surviving identities atomically and report removed IDs skipped, correct Clock API 3 installer text. | Editor named Set#tings.qml loads; restore survivor/invalid input tests; Clock installer fixture. |
| N14 | Default builds cannot enable reveal or grant leased overlay privileges. Keep prototype behind explicit experimental-reveal feature. | Default refusal and feature-enabled policy tests; CI runs actual proxy tests in both modes. |
| N15 / N16 | Explicit separate application/service rationale, compatibility-bridge boundary and constrained grid promise. Retain the isolation and state/recovery responsibilities. | Documentation correction. Upstream/marketplace acceptance and supported Arch packaging are not claimed. |
| API 1 scope recommendation | Remove API 1 manifest conversion, display-context facade and runtime size aliases. Reject API 1 at all package entry points and in preview tooling. API 3 is the author target. | Rust validation/install/update/load/rollback rejection tests; existing fixtures use supported APIs. Preview rejects API 1 before rendering. |

## Scope decisions

Weather remains an independent bounded experimental service. A second provider
should drive a generic provider design; no arbitrary renderer networking was
added. Reveal and weather do not share a feature flag. API 1 support was removed
at the owner's request while the project is local development. API 2 remains
supported; API 3 is the author target. Registry-file recovery is independent of
widget API support and does not load API 1 code.

## Verification boundaries

Local Rust and offscreen Qt checks do not run the assembled Omarchy application.
Two existing pathname Unix-socket tests cannot bind in this container (`EPERM`);
they remain mandatory in GitHub CI. The live proxy, Bubblewrap and disposable
systemd resource jobs remain enabled. Their fixtures do not establish desktop
latency, keyboard focus, lock-screen behaviour, suspend/resume, hotplug, fractional
scale, theme switching or normal CPU/memory/wakeups. The real install/desktop and
performance matrices in the two reviews and `desktop-checks.md` remain acceptance
gates. No release, merge or claim of outside-author stability is part of this work.

Preview renders intentionally show World Clock's inactive/loading surface: its
helpers and network are not executed. A successful capture is evidence of the
preview contract and import compatibility, not live clock data or installed
Quickshell behaviour.
