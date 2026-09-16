# Final development-stack verification

Recorded 16 September 2026, Linux x86_64, Rust/Cargo 1.98.1, PySide6 6.11.2,
offscreen Qt. World Clock fixture: `5cd7b0894376eb16d070fa244f4aeaea1448dd58`.
`stack-source-sha256.json` identifies the tested runtime, SDK, workflow and tests.
Earlier milestone hashes describe their historical snapshots.

| Check | Result / scope |
| --- | --- |
| Rust portable suite | 47 passed; two Unix-bind tests excluded because local bind returns EPERM. Full suite is run without exclusions by CI. |
| Rust format and strict Clippy | Passed. |
| QML parsing and component smoke | Passed, including Core, examples and starter. |
| Lifecycle/weather editor | Actual component transition order, activity gates and coordinate validation passed. |
| QML → Rust registry integration | Durable writes, stale-save rejection, queue behaviour and lock release passed. |
| Real World Clock integration | Two independently configured instances; Save/Cancel/Escape, repeated gear, old acknowledgement, stale conflicts, hide/show and reopen passed. |
| Manager integration | Actual mouse actions for Add, sizes, Duplicate, settings, Hide/Show, removal confirmation and retained uninstall/reinstall passed. |
| SDK author workflow | Scaffold/refusal to overwrite, actual starter previews, validation, install, two instances, invalid save, independent hide/remove, retained reinstall, update/rollback passed. |
| Installer, bindings, JS geometry/workspace and sandbox structure | Passed. |
| Preview inspection | Actual Small and Large starter PNGs inspected; text and frame fit. Generator also renders Medium. Offscreen output only. |
| Parent PR CI | #7–#9 fully green; #10 main run green, final companion resource run checked separately. |
| Live Hyprland, XPS, docking, focus/lock behaviour and actual weather provider | Not run. Required before a supported release. |

CI retains five jobs: Rust (including both socket tests), QML/integration/SDK,
live Bubblewrap, actual pinned Wayland proxy against a wire fixture, and resource
limits in a disposable systemd user session. Wire/resource tests do not prove
visual, lock-screen or physical-hotplug behaviour.

Final integration fixes included in the SDK PR: gear and manager both register
settings editors before loading; an open editor blocks code updates/rollback;
stale close acknowledgements cannot release another editor; explicit package
restart clears a stuck request. Exterior appearance uses only Core desktop tokens.
These changes have regression coverage in the final Rust/World Clock run.
