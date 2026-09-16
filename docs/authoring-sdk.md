# Authoring a Widget Core package

This SDK targets the experimental API 2 implementation in the seven-PR development
stack. It is not a promise that an older build advertising API 2 implements every
optional extension. Use the stack tip for development; a supported release awaits
live desktop acceptance.

## Start and validate

```bash
omarchy-widget new ./my-widget io.example.my-widget "My widget"
omarchy-widget validate ./my-widget
omarchy-widget install ./my-widget
omarchy-widget manage
```

`new` writes a complete standalone package and refuses an existing destination.
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
through the actual Core frame with fixed sample text, font, theme and family sizes.
It requires PySide6 6.11.2. It never loads an arbitrary installed package. Output
is genuine offscreen Qt imagery, not evidence of a live Omarchy session. Font/Qt
changes may alter pixels; use the pinned CI environment for repeatable output.

For your own widget, capture your actual supported layouts in a controlled test
profile and add `previews:{"small":"previews/small.png",...}`. Images must remain
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
