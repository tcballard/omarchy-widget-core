# Manager refinement · 22 September 2026

Focused pass using [Omarchy App Design](https://github.com/tcballard/build-omarchy-apps/blob/main/skills/omarchy-app-design/SKILL.md), retrieved 22 September 2026. Preserves Rust state ownership and the existing Quickshell/QML interface. This is a local development update, not a release.

## Workflow

- Your widgets: name, size, workspace and state first; Settings and Show/Hide are the primary actions.
- More: workspace assignment, display information, Duplicate, Remove and instance identity. Disclosure survives registry refreshes.
- Add widgets: static size/art preview, family selection, Add. Manage package contains restart, enable/disable, export, rollback, weather permissions and uninstall.
- Unplaced widgets and package faults remain visible without expanding details. Recovery controls remain immediately available when needed.
- Confirmations identify the affected widget, initially focus Cancel and return focus on cancellation. Empty states offer a direct path to the gallery.
- Manager-specific buttons use Core's theme colours, font and scaling. The widget frame and World Clock visuals are unchanged.

## Reproduced now

- QML smoke passed.
- Twelve real manager/controller/registry integration runs passed: three each at 1, 1.25, 1.5 and 2 scale. Includes disclosure by keyboard, independent instances, Save request routing, removal cancellation/focus, uninstall keep/delete, retained settings and reinstall.
- Sixteen offscreen fixture renders cover light/dark, 480/760 widths, default and expanded views, empty/error states and confirmation. Representative images inspected; no QML warnings.
- Installed manager opened and its actual desktop screenshot was inspected. This does not claim a complete live keyboard, focus, hotplug or reboot acceptance run.
- `git diff --check` passed.

Reproduce fixture renders with `python tools/render-manager.py` in the PySide6 test environment. Outputs and source hashes are under `test-results/manager-design/`; `live-manager.png` is the desktop capture, while the other PNGs are offscreen fixtures. Interaction results are in `interaction-matrix.jsonl`.

Installed UI backup: `~/Work/widget-review-20260922/manager-design-backup/Manager.qml`. Only `qml/Manager.qml` and the new `qml/ManagerButton.qml` were deployed for this pass; existing widget settings and placement were preserved. To undo the UI pass, close settings, restore the backed-up Manager.qml and restart Widget Core. The unused ManagerButton.qml can remain until the next clean installation.

## Packaging direction to carry forward

The user proposed the official Omarchy package flow. The [omarchy-pkgs documentation](https://github.com/omacom/omarchy-pkgs#sync-upstream-releases), checked 22 September 2026, supports an Omarchy-owned PKGBUILD with `.omarchy/package.json` upstream-release metadata (or a custom upstream hook). Its automation updates versions/checksums and builds, signs and publishes packages. Build recipes still require maintenance when packaging or dependencies change; repository admission is separate.

Preferred direction: that system distributes Widget Core, while Widget Manager owns instances, settings, visibility and placement. Before distributing individual widgets through it, resolve system-package ownership versus Core's existing per-user generations, migrations and rollback. No package submission, packaging migration or removal of current recovery functions was performed in this UI pass.

## Screensaver stacking fix · 22 September 2026

The original manager used an overlay-layer PanelWindow, which appeared above the
fullscreen terminal used by Omarchy's screensaver. It now uses a normal
FloatingWindow titled Widget Manager, with no layer-shell role. Hyprland applies
its normal application tiling/stacking policy. Native close sends Core's existing
close-manager command and clears local open state; the lazy manager UI and shared
renderer lifecycle remain intact.

Reproduced after the change: QML smoke, manager integration, declarative editor,
review delivery, countdown and pinned World Clock integration all passed. Four
fixtures now locate the settings window by its title rather than assuming it is
the first FloatingWindow in Host.qml. No Rust changes.

Live Hyprland verification: the manager appears in clients, not layers; the
actual screensaver mapped fullscreen on the manager's workspace and completely
covered it (screenshot inspected). Native close persisted through subsequent
refreshes, and reopening succeeded. The first attempt captured a different
workspace and used an unsupported legacy close dispatcher; it was not counted
as a pass. The successful repeat used the installed Lua dispatch API.

Evidence: ~/Work/widget-review-20260922/stacking/ contains client metadata, the
inspected screensaver-over-manager.png and final Host.qml hash. Portable results
are in test-results/manager-window-tests.json. Pre-fix installed Host.qml backup:
~/Work/widget-review-20260922/Host.before-window-fix.qml.
