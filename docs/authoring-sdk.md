> Default authoring now uses the [declarative contract](declarative-widgets.md). The QML workflow below is the advanced isolated option; use `new-qml` for that starter.

# Authoring a Widget Core package

This SDK targets API 3. Older hosts reject these packages before installation.
See [compatibility.md](compatibility.md) for the declared runtime and evidence.
Live desktop acceptance remains a release gate.

## Start and validate

```bash
omarchy-widget new-qml ./my-widget io.example.my-widget "My widget"
omarchy-widget validate ./my-widget
omarchy-widget install ./my-widget
omarchy-widget manage
```

`new-qml` writes a complete standalone package and refuses an existing destination.
The parent directory must exist. `validate` checks identity, API, supported sizes,
paths, bounds, defaults, declared settings schema, migration shapes, capabilities
and dependencies. It never executes the package. It does not prove that arbitrary
QML compiles or that its behaviour is safe. Test actual loading inside Core.

`dependencies.commands` declares up to 16 executable basenames required under
`/usr/bin`, for example `{"commands":["date"]}`. Missing commands fail validation.
Dependencies do not grant new permissions. QtQuick and QtQuick.Layouts are supplied
by the runtime; additional QML imports require testing against the installed Qt.
Core owns network access: use the optional weather service, not an HTTP client.

## One contract

- View root: `required property var widgetContext`. Render content within the
  supplied frame. Use context theme and metrics; never position a desktop window.
- Editor root: `required property var settingsContext`. Assign a new draftSettings
  object when editing. Optional validationError is displayed by Core and disables
  Save. Declare a schema to enforce the same rules at the persistence boundary.
- Families: small 1×1, medium 2×1, large 2×2. Supply useful additional content at
  larger sizes. The starter shows a note excerpt, full note, then saved metadata.
- Lifecycle: gate timers on active/backgroundAllowed. The host emits resuming,
  becameVisible, becameHidden and suspending. A crashed/unloaded view may receive
  no final callback. Persist settings through Core; reconstruct timers from absolute
  timestamps. No author hook has authority to run outside the package sandbox.
- Capabilities: none by default; weather is the only optional broker service.
  A user grants each installed generation permission explicitly.

Full signatures and ownership rules: [widget-contract.md](widget-contract.md).
Settings schemas, migrations and rollback: [recovery.md](recovery.md).
Weather permissions and bounded data: [network-widgets.md](network-widgets.md).
World Clock PRs [#1](https://github.com/tcballard/omarchy-widget-worldclock/pull/1)
and [#2](https://github.com/tcballard/omarchy-widget-worldclock/pull/2) demonstrate a
real settings-heavy consumer. `examples/weather` exercises the data contract.

## Previews and verification

`python3 sdk/render_previews.py OUTPUT_DIRECTORY` renders the **bundled starter**
through the actual Core frame with manifest defaults, font, theme and family sizes.
It requires PySide6 6.11.2. Use the sandbox wrapper below for your own package. Output
is genuine offscreen Qt imagery, not evidence of a live Omarchy session. Font/Qt
changes may alter pixels; use the pinned CI environment for repeatable output.

For your own widget, install system `python-pyside6` and `bubblewrap`, then run
`bash sdk/capture-package ./my-widget ./empty-preview-output`. It renders your
actual entry point and defaults inside a networkless Bubblewrap process with no
home, session bus or compositor socket and only the empty output folder writable.
It has a 30-second watchdog and fails closed when sandboxing is unavailable.
Do not run the internal Python worker directly on third-party QML. Preview saves
and networking are deliberately inactive; dynamic widgets may show empty states.
Use a disposable Omarchy profile to capture live dynamic content. Add `previews:{"small":"previews/small.png",...}`. Images must remain
inside the package, at most 512 KiB and 1024×1024. The manager never executes
preview QML. If images are absent it presents a labelled footprint preview.
Do not reuse the starter screenshots as images of a different widget.

`python3 tests/sdk.py target/debug/omarchy-widget` reproduces an external author's
portable workflow: scaffold, refusal to overwrite, render/validate previews,
install, create two instances, save different settings, reject invalid settings,
hide/remove one, retain/reinstall the other, update and roll back. Tests use fresh
CLI processes and isolated state. They do not replace the live desktop checks.

Before distributing: test all declared families, invalid settings, Save/Cancel,
loading/stale/error states, inactive timers, theme changes, two different instances,
upgrade/rollback, unplug/replug and runner restart. Include your license, dependency
requirements and actual tested Core/Qt/Omarchy versions. Keep migration fixtures
from each supported settings version. Never claim compatibility from a manifest
validation result alone.

## Interactive reference

`examples/countdown` implements Start, Pause and Resume using only the public
revisioned action API. `python3 tests/countdown_integration.py
 target/debug/omarchy-widget` exercises the real Host context and queue against
Rust storage, two independent instances and the shared keyboard gear. The Clock
integration additionally checks action-versus-editor conflicts. Preview rendering
is not evidence for timer alarms during sleep; no background alarm service exists.

## Preview contract and limitations

Capture supplies deterministic API 3 identity, settings/revision, save state,
weather state, appearance and the real lifecycle interface. It is inactive and
side-effect methods return without saving or fetching. `Quickshell.Io.Process`,
`StdioCollector` and `IpcHandler` imports have capture-only inert implementations:
no helper command runs and no IPC endpoint opens. This allows World Clock to render
its real inactive/loading surface in all families; it is not a screenshot of live
clock data or a runtime-integration test. Unsupported Quickshell modules fail
visibly. Use an Omarchy session for live content and actual runtime acceptance.

`tests/preview_contract.py` exercises a separate consumer of every public context
member and verifies disabled side effects. CI also runs it and pinned World Clock
through the Bubblewrap capture wrapper. The preview watchdog is not the installed
runner's cgroup resource policy; only use reviewed/trusted packages.
