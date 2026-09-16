# Widget Core implementation stack

Requested 16 September 2026. Work through these milestones as dependent development
PRs. Preserve the existing Rust broker, per-package sandbox runners, grid and API 2.
World Clock is the first consumer of the infrastructure. Do not merge or release
as part of preparing this stack. Keep hardware acceptance distinct from portable
verification; no live Omarchy session is available in the build environment.

## Branch and merge order

1. World Clock [PR #1](https://github.com/tcballard/omarchy-widget-worldclock/pull/1)
   (`feat/clock-faces-and-settings` → `main`) supplies API 2 and gear-only settings.
2. World Clock [PR #2](https://github.com/tcballard/omarchy-widget-worldclock/pull/2)
   (`feat/widget-foundation` → `feat/clock-faces-and-settings`) fixes embedded
   Escape cancellation. Retarget after the parent merges. Its tested commit is
   `5cd7b0894376eb16d070fa244f4aeaea1448dd58`.
3. Core `feat/widget-foundation` → `main` protects settings drafts and checks
   two real World Clock editors against production QML and the Rust registry.
   The CI fixture is pinned to the World Clock commit above; it does not follow
   an unreviewed moving branch.
4. Subsequent Core branches target the preceding Core milestone branch. Record
   actual PR URLs here as they are opened. Do not create empty placeholder PRs.

## Milestones and remaining work

| Milestone | State | Deliverable / acceptance |
| --- | --- | --- |
| 1. Foundation + World Clock | Development implementation; portable checks reproduced | Independent Save/Cancel, stale revisions, duplicate identities, placement preservation and fresh-process reopen. Live settings windows and reboot remain unverified. |
| 2. Widgets manager | Next | Available types versus instances; size previews; Add/Duplicate/Configure/Hide/Remove; separate instance removal and package uninstall with retained-state choice. |
| 3. Desktop integration | Pending | Validate existing grid and inherited ricing; reserved bounds, scaling, hotplug, workspace occupancy and preference restoration. Preserve monitor-bound workspace semantics. |
| 4. Quick reveal | Feasibility pending | Current filter denies overlays. Prototype Core-controlled reveal without importing widget QML into the trusted manager or granting permanent overlay privileges. Prove focus return, dismissal, lock behaviour and unchanged application placement on Hyprland. |
| 5. Lifecycle + data | Pending | Explicit lifecycle, rendering versus background activity; one public weather broker with shared authorised caching, bounded fetches, permissions, stale data and retries. Keep renderer network denial. |
| 6. Recovery + faults | Pending | Code/settings checkpoints, staged versioned migrations, compatible rollback, preserve later edits, manager-visible failures and package-level retry/disable. Test fault injection. |
| 7. SDK + baseline | Pending | Starter, validator, shared status/settings components, deterministic previews, migration examples and external-author acceptance. Stabilise after Clock and weather exercise the contract. |

The quick-reveal feasibility work must precede its production implementation.
Existing policy is intentionally unchanged by the foundation PR; that PR does not
claim reveal is implemented or feasible under the unchanged policy.

## Foundation verification reproduced locally

Linux x86_64; Rust/Cargo 1.98.1; PySide6 6.11.2; offscreen Qt.
Source inputs: Core main `dc96dd4a4dc72603cabaae4e7151f7c15c47c662` plus this PR;
World Clock `5cd7b0894376eb16d070fa244f4aeaea1448dd58`.
The adjacent `foundation-source-sha256.json` identifies tested runtime/test files.

| Check | Result and boundary |
| --- | --- |
| `cargo test --locked --offline` on unmodified Core baseline | 26 pass; two Unix-socket tests fail with `Operation not permitted` in this environment. Not a green full Rust run. |
| `cargo build --locked --offline` | Pass; real binary used below. |
| `python3 tests/qml_smoke.py` | Pass; component parsing, manager, frame and example settings. |
| `python3 tests/runtime_integration.py target/debug/omarchy-widget` | Pass; production save/queue → Rust → disk, conflict handling, coalescing and lock release. |
| `python3 tests/worldclock_integration.py target/debug/omarchy-widget ../clock` | Pass; real World Clock settings, two independent instances, Save/Cancel/Escape, repeated open, old acknowledgement, stale-save draft retention, hide/show, placement and fresh-process reopen. |
| World Clock model, installer, Qt smoke and shell syntax checks | Pass; detailed commands recorded in World Clock PR #2. |
| Real Hyprland, sandbox runners, reboot, theme switching, hotplug and fractional scaling | Not run; require desktop acceptance. |

The integration harness uses the production controller, settings context, Loader,
panel, queue and widget editor. It substitutes a Qt window and QProcess transport;
it does not execute the Wayland proxy, package sandbox or supervisor. One old
acknowledgement is deliberately injected to verify generation ownership. Real
Save actions invoke the actual Rust binary against an isolated temporary profile.
Every CLI read reopens state in a fresh process; this is not a machine reboot.

## Next session

Start with milestone 2 on top of Core `feat/widget-foundation`; inspect open PRs
and heads first. Avoid duplicating World Clock #1. Carry forward the live desktop
gates and the early reveal investigation. Update this record as each PR lands.
