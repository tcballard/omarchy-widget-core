# Interaction lifetime hardening — 7 October 2026

Core remains experimental v0.0.4. This development change preserves the grid,
package isolation, storage format and disabled-by-default reveal policy.

## Implemented

- A drag commits only on a normal release after movement. Pointer cancellation,
  Escape, leaving Arrange, hiding the card or its window, losing window focus,
  and a changed placement/grid/workspace cancel the gesture and restore the
  resolved position. A later release cannot commit the cancelled movement.
  Arrow keys cancel any pointer gesture before making their own single move.
- Weather replies carry the requesting settings revision and package directory.
  Successes and failures from an old revision, replaced package or removed
  instance are ignored. Previously displayed data stops being exposed as soon
  as its revision or package no longer matches. The public weather result shape
  and provider permission checks are unchanged.
- The queue coalesces only adjacent pending requests of the same kind and
  instance. A save, hide, permission change or other intervening action keeps
  its ordering. Requests already scheduled for retry retain their retry budget.
  The existing 64-request bound remains in place.

The source review was informed by [Omarchy Stickies](https://github.com/eastbluewizard-official/omarchy-stickies/tree/683c573267d0c15f63aabf491ea0af123a79b1de),
especially local drag geometry, commit-on-release, focus ownership and state
notifications. These are independently implemented fixes to Core; no Stickies
source was copied. Stickies is MIT licensed, copyright Ricardo Otto.

The review also identified smooth layer switching and a reversible sidebar as
potential future work. Neither is implemented here. Core's isolated renderers
must not gain permanent top/overlay or exclusive-keyboard privileges to reproduce
a trusted shell plugin's behaviour. Hidden QML content is still retained with
the existing lifecycle notifications; this change does not promise lower idle
memory or event-driven replacement of snapshot polling.

## Reproduced locally

Base: `e1b8c1582d0a39cafe9a46add383b7651c6fa649`. Linux 6.18.44,
Python 3.12.14, PySide6 6.11.2, offscreen Qt; Cargo/Rust toolchain 1.99.0.
The repository's pinned 1.98.1 toolchain was unavailable locally, so CI retains
the pinned toolchain check.

SHA-256 identities of tested production inputs and the regression test:

| File | SHA-256 |
|---|---|
| `Host.qml` | `3455887cd12d1643915a8954885d5f169e911381ec8e3f7bf1f4cb6f27ec34fb` |
| `qml/WidgetFrame.qml` | `0ca19a390263350ed91c065c6ae91e5447ce07f4b087b30c3b9fec4b86abd21a` |
| `qml/RegistryQueue.qml` | `5c65f1b2eccfc89c2001ee09f3c77b0b7a3afe39e2bdefcd0518bb25c7d3072a` |
| `tests/interaction_lifetimes.py` | `15d2a055c0ea8d933849d85f66cad631c830cad0b382b3fffa91c19d76c62b35` |

Commands (use a Python environment with PySide6 6.11.2):

```bash
cargo +1.99.0 build --locked
for test in qml_smoke lifecycle window_lifecycle interaction_lifetimes; do
  python3 "tests/$test.py" || exit
done
for test in runtime_integration countdown_integration review_delivery declarative_controller manager_integration; do
  python3 "tests/$test.py" target/debug/omarchy-widget || exit
done
git diff --check
```

The new regression uses actual Qt mouse/key events, the production frame,
production Host handlers/context and RegistryQueue. Only transport and the
placement sink are controlled fixtures; existing integrations cover real Rust
saves and reopening persisted state. The expected broken-component diagnostic
in `review_delivery` is part of that test's recovery fixture.

Negative controls also fail as expected: restoring the old RegistryQueue loses
the ordering barrier; restoring the old Host exposes old-location weather;
restoring `onCanceled: root.finishedMoving()` commits a cancelled pointer drag.

## Failed locally / not established

`cargo +1.99.0 test --locked` exited 101: 81 passed, 4 failed, 2 ignored.
All four failures were `Operation not permitted` at Unix socket creation:
`broker_connection_wakes_housekeeping_wait`,
`broker_scopes_reads_writes_and_generations`,
`worker_lifetime_is_revoked_even_with_a_stale_socket_path`, and
`socket_round_trip`. No Rust files changed. The ignored checks require opt-in
captured or live GitHub data. This is not a locally green full Rust run; GitHub
CI must validate the normal socket-capable environment.

No live Omarchy/Hyprland version was tested here. Offscreen Qt establishes
gesture cancellation logic, not native compositor focus, hotplug or remapping.

## XPS acceptance

Install this branch using the existing README's `bash install-local --update`
instructions. Keep the backup path printed by the installer for rollback; this
change does not migrate or discard user data. Do not enable experimental reveal.

1. Arrange a widget and release: exactly one final grid position persists after
   restarting Core. Clicking its header alone does not save a placement.
2. Start a drag, then separately test Escape, switching focus to another app,
   hiding widgets, and switching workspace. Release afterwards: the original
   position remains. A later fresh drag works normally.
3. Disconnect/reconnect a monitor or change scaling during a drag. The gesture
   cancels and Core retains its preferred placement/fallback behaviour.
4. Press an arrow key during a drag: one keyboard move is saved; the subsequent
   mouse release does not save another move. Check collision rejection too.
5. With the weather fixture enabled, change location while a reply is pending.
   Old temperatures/errors must not appear under the new location. Save, Cancel
   and package restart continue to work.

The existing README documents installation, runtime removal and data preservation.
