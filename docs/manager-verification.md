# Manager milestone verification

Development PR, 16 September 2026. Parent: Core #5,
`14d6e0e130505fa6cfbf0344d154c5b1d14c1009`. No merge or release.
The parent CI was checked at that exact commit: all five jobs passed for both
push and pull-request runs.

## Reproduced now

Linux x86_64, Rust/Cargo 1.98.1, PySide6 6.11.2, offscreen Qt.
`manager-source-sha256.json` records the tested runtime/test inputs. The earlier
foundation hash manifest remains historical evidence for its original PR inputs.

| Command | Result |
| --- | --- |
| `cargo +1.98.1 fmt --all -- --check` | Pass |
| `cargo test --locked --offline -- --skip socket_round_trip --skip broker_scopes_reads_writes_and_generations` | 32 passed; two environment-blocked socket tests explicitly excluded locally, still required in CI |
| `cargo +1.98.1 clippy --locked --offline --all-targets -- -D warnings` | Pass |
| `cargo build --locked --offline` | Pass |
| `python3 tests/qml_smoke.py` | Pass; parses production QML and exercises manager/frame components |
| `python3 tests/manager_integration.py target/debug/omarchy-widget` | Pass; mouse actions through production Manager and Host handlers, queue and actual Rust CLI |
| `python3 tests/runtime_integration.py target/debug/omarchy-widget` | Pass; durable acknowledged writes, stale saves, queue coalescing, lock release |
| `python3 tests/worldclock_integration.py target/debug/omarchy-widget ../clock` | Pass against World Clock `5cd7b0894376eb16d070fa244f4aeaea1448dd58`; removing an instance now also closes its open editor |

Manager coverage includes selecting Small/Large, adding independent default
instances, duplicating configured settings, opening Configure, hiding/showing,
Cancel and Escape from removal confirmation, removing exactly one instance,
uninstall with keep/delete, deleting one retained instance and reinstalling
without automatic activation. State assertions come from new CLI processes
reading the temporary on-disk registry. Configure verifies the real editor
request; the separate World Clock integration checks editor Save/Cancel.

Rust checks cover sibling preservation, stale-save rejection after removal,
retained settings and visibility, reinstall, invalid package removal, defaults
versus duplication, unsupported sizes and structural PNG preview bounds.
The existing broker isolation check now asserts management metadata is redacted
and the new commands are denied to package runners.

The manager test generates a flat-colour PNG as a decoder fixture. It is not a
widget screenshot. The resulting `test-results/manager-available.png` and
`manager-instances.png` are honest offscreen Qt captures of the test fixture.
Both were visually inspected for clipping at 760×660. No Omarchy screenshot is
claimed. Packages without artwork show a labelled footprint preview.

## Blocked or not run locally

The full Rust run reaches two Unix-socket bind tests that fail with
`Operation not permitted` in this environment; no source skips were added. CI
runs the full suite, including broker authority and resource/Wayland checks.

No live Omarchy session is available. Still required: actual manager launch,
focus and tab traversal, package-runner shutdown after uninstall, settings-window
closure after removal, theme changes, fractional scaling, monitor/workspace
behaviour, reboot and package installation on the XPS. Offscreen rendering and
QProcess adaptation do not establish any of those claims.

## Deliberate scope

The gallery lists installed types, not a remote marketplace. Static images never
execute package QML in the trusted manager. Uninstall unregisters executable code
and stops its runner; version directories remain retained on disk under the
existing recovery policy. Network data, reveal, migrations and code garbage
collection are not added here. Settings editors continue to run in package
islands. Core remains experimental v0.0.2.

## Desktop test-environment correction · 22 September 2026

On the XPS, the portable manager matrix initially failed 12 of 40 runs around
removal/uninstall assertions and control availability. Its `setdefault` allowed
the desktop's `QT_QPA_PLATFORM=wayland;xcb` to override the documented offscreen
fixture. Instrumented runs confirmed the Wayland backend and compositor-sized
windows (932×551 or 932×1122 instead of the requested 760×660). Four instrumented
native runs passed; an individual lost click was not captured, so those runs do
not establish the exact event-level cause of each earlier failure.

The suite now explicitly selects `offscreen` and the neutral `basic` platform
theme. Direct manager tests enforce the same environment, assert the actual
platform, and verify each synthetic click emits its control's action before
checking registry results. The neutral theme also avoids inherited GTK trying
to open a display in a headless process. No manager production logic changed.

With PySide6 6.11.2, all 40 offscreen workflows passed: ten runs each at scale
1, 1.25, 1.5 and 2. The first five runs preceded the additional click-delivery
assertion; the remaining 35 included it. A final direct invocation with inherited
desktop variables also passed. All six evidence-harness tests passed, including
the regression that checks environment isolation for every suite command.
This resolves the observed portable-test flakiness; it does not replace real
manager focus, input, compositor or desktop acceptance.

Local raw evidence: `~/Work/widget-review-20260922/manager-offscreen-baseline.jsonl`
and `manager-native-diagnostic.log`; the evidence report retains the original
failed runs and appends the successful matrix rather than erasing history.
