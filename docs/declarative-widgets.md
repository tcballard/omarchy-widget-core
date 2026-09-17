# Declarative widgets (experimental v1)

Declarative packages describe bounded views and settings as JSON. Core renders all
of them in one trusted Quickshell process. The closed manager UI is unloaded.
No package QML, JavaScript, shell helper, URL or native library is loaded into that
process. Advanced QML packages retain one sandbox, proxy and renderer per package.

This is the first working slice, not the entire future component catalogue.
Supported: row, column, text, clock and one-level repeat; generated text, choice
and timezone-list settings. Custom actions, network providers, images, charts and
arbitrary expressions are not supported. A future provider protocol must keep
provider code outside the renderer and deliver bounded, validated data.

## Authoring

`omarchy-widget new PATH ID NAME` creates a declarative package by default.
`omarchy-widget new-qml PATH ID NAME` creates the advanced isolated QML starter.
Use `validate PATH`, then `install PATH` or `update PATH` as usual. Existing
installation, independent instances, revisioned Save/Cancel, migrations, export
and rollback apply to both formats. The JSON file is the executable contract;
there is no QML entry point for declarative widgets.

```json
{
  "renderer": "declarative",
  "requires": ["declarative-v1", "settings-schema", "frame-settings"],
  "view": {
    "type": "column",
    "children": [
      {"type": "text", "value": {"setting": "title"}, "style": "heading"},
      {"type": "clock", "timezone": "Europe/London", "mode": "digital"}
    ]
  }
}
```

This is an excerpt: identity, schemaVersion 2/coreApi 3, families, defaults,
settingsSchema and settingsUi are also required. See
[the complete World Clock](../examples/declarative-clock/widget.json).
Old hosts reject the missing required feature rather than treating this as QML.

Bindings are literal strings, `{ "setting": "topLevelKey" }`, or inside repeat,
`{ "item": "label" }` / `{ "item": "zone" }`. There is no evaluation language.
Text is plain text. No URLs or user-supplied component names are interpreted.

Limits: manifest 16 KiB, settings 8 KiB, view depth 6 and 64 authored nodes;
1–12 layout children; no nested repeats; repeat arrays capped at 12 items;
16 settings controls; choices capped at 8 strings. Repeat displays 1/3/6 items
in small/medium/large. For a timezone list, small prefers settings.homeZone when
present. This is the initial paging policy; no scrolling or extra pages yet.

Settings controls declare `key`, `label`, and `type` (`text`, `choice`,
`timezone-list`). Core validates the matching schema. Timezone-list entries are
label/zone objects with installed IANA names. Invalid saves preserve the draft
and return a validation error. Home timezone is a text setting; a value outside
the city list falls back to the first city. Empty city lists render no clocks.

Clock data is shared across visible instances (maximum 128 distinct zones),
refreshed once per observed minute or when requested zones change. Core invokes
its own short-lived Rust `clock-times` command, which uses system tzdata/libc;
there are no package helpers and no new runtime dependencies. Hidden/workspace-
inactive clocks request no time data. The existing one-second snapshot polling
also checks for minute changes and catches resume/wall-clock changes. Polling is
not yet replaced by push notifications. Seconds hands and animation are absent.
Previews use the same renderer with unavailable clock data (an em dash), not an
invented live time.

## Process and failure boundary

The trusted UI process owns declarative surfaces and generated editors and opens
the manager on demand. It is not a package sandbox: only Core code runs there.
A Core renderer defect can affect all declarative widgets. Strict limits reduce
that risk but do not give them separate process isolation. Package disable/hide
removes that package's surfaces; advanced QML code still cannot access this
process or unfiltered Wayland. The shared process remains under the host service
resource limits. The installed proxy is still needed for the optional QML path.

## Development test and migration

```bash
cargo test --locked
python3 tests/declarative.py target/debug/omarchy-widget
python3 sdk/render_previews.py test-results/declarative --package examples/declarative-clock
```

Update an installed World Clock with `omarchy-widget update
/path/to/core/examples/declarative-clock`. Its package ID is unchanged. Settings
version 2 fills missing displayMode/homeZone while preserving cities. Invalid
older settings (e.g. more than 12 cities or unknown timezone) block the update
without changing the installed package. Use `omarchy-widget rollback
io.github.tcballard.worldclock` before subsequent settings edits to restore the
previous QML version/checkpoint.

The earlier 90.6 MiB / 1.58% single-clock capture describes the isolated QML
runtime, not this renderer. New memory and CPU figures require desktop captures.
Run single, multi and hidden captures with this revision; include two declarative
packages, several instances and a mixed declarative/QML scenario. Expect one
shared `qs` for declarative-only operation and zero island units. Do not infer
savings from process count alone. Check gear focus, Save/Cancel, arrange,
workspace/monitor changes, theme, update/rollback and restart on Hyprland.
