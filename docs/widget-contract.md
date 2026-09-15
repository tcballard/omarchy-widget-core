# Widget contract: experimental API 1

## Additive context members in Core 0.1.1

`appearance` contains merged theme and per-widget appearance settings. See
[widget-appearance.md](widget-appearance.md). `requestInput(bool)` opts the
card into on-demand keyboard focus while an editor is open; call false on
close. `saving` reports registry activity, and `saveError` reports the last
Core error. These members do not exist in Core 0.1.0; widgets needing them
must guard access and document their minimum Core version.

A widget repository contains `widget.json`, a QML content entry point and any relative assets it needs. It does not contain a Quattro plugin manifest or start another Quickshell instance. Core is installed once, independently.

## Manifest

Illustrative metadata, not a bundled widget:

```json
{
  "schemaVersion": 1,
  "kind": "desktop-widget",
  "coreApi": 1,
  "id": "io.github.example.worldclock",
  "name": "World Clock",
  "version": "0.1.0",
  "entryPoint": "WorldClock.qml",
  "defaultSize": "standard",
  "sizes": {
    "standard": {"width": 360, "height": 300},
    "wide": {"width": 520, "height": 300}
  },
  "defaults": {"cities": ["Europe/London", "America/New_York", "Asia/Tokyo"]}
}
```

IDs start with a lowercase ASCII letter, contain lowercase letters, digits, dots or hyphens, are at most 100 bytes and cannot contain `..` or end with a dot. Version is numeric `major.minor.patch`. Name is nonempty and at most 100 bytes. API/schema version must be 1.

Declare one to three named sizes: `compact`, `standard`, `wide`. Width and height are whole logical units from 120–1600; `defaultSize` must exist. Dimensions include Core's title frame, and content fills the remaining space. Arrangement controls add height while editing. Core applies Quattro's UI scale and clamps the surface to the available screen.

`defaults` is a JSON object. Core copies it into the initial placement. Widgets own semantic settings validation, migrations and good defaults; Core checks only that saved settings are objects of bounded size. Changing package defaults does not rewrite an existing placement.

Manifest limit: 16 KiB. Entry point: relative `.qml` path, within the package. No symlinks, special files or paths deeper than 12 components. Installation allows at most 256 files / 8 MiB, with an additional 32 KiB budget charge per directory, and skips `.git`. Keep build outputs outside the installable package. Core allows at most 64 installed packages. Package files are copied as non-executable files; executable helpers are outside API 1's packaging contract.

## Content entry point

```qml
import QtQuick
import qs.Commons

Item {
    required property var widgetContext
    Text {
        anchors.centerIn: parent
        text: widgetContext.settings.label || "Hello"
        textFormat: Text.PlainText
        color: Color.foreground
        font.family: Style.font.family
        font.pixelSize: Style.font.body
    }
}
```

Core passes `widgetContext` through Loader initial properties before component completion:

| Member | Meaning |
| --- | --- |
| `settings` | Read-only reference to the current saved JSON object; do not mutate it in place |
| `active` | Whether the host considers the surface visible and active |
| `sizeName` | Current supported size name |
| `saveSettings(object)` | Starts a bounded asynchronous registry write; returns whether it was accepted |

`saveSettings` returning true means accepted, not durable completion. Core reports write errors in the frame/manager and refreshes saved data after success. Busy operations return false; widgets must retain local edits and offer retry. The entire object is replaced. Maximum settings request: 8 KiB; complete layout: 64 KiB.

Do not assume a widget instance survives refreshes. Registry refresh may recreate its view; persistent state belongs in settings. Core unloads content when globally hidden or the monitor's workspace is fullscreen, and recreates it when active again. A normal overlapping window does not suspend the widget. Pause timers and producers when inactive and cancel them on destruction. Debounce settings saves; do not save on every timer tick.

Content should use `qs.Commons` semantic `Color` and `Style` tokens, plain text for external strings and Core's available content size. Do not create desktop windows, reserve screen edges or capture the whole desktop. Core owns those responsibilities. Text-input-heavy settings flows and a separate settings-page API are deferred; the foundation supports CLI configuration and widget-owned pointer controls.

Loading failures display an error within the card. There is no process isolation: a QML binding error may be contained, but a blocking loop or process crash in a widget can affect the host. The context does not prevent imports or privileged user-level operations.

## CLI contract

`omarchy-widget validate PATH` validates without loading QML. `install PATH` copies a reviewed snapshot without adding it. `add ID` enables its single placement, `hide ID` preserves it but disables it, `list` returns installed manifests/directories/placements and validation problems. `configure ID JSON` replaces settings; `place ID JSON` stores full position/size/monitor metadata. `remove ID` removes the package and placement. Refresh the host after external CLI changes.

An installed snapshot is treated as immutable. API 1 refuses replacement of an existing ID. To experiment with a new version, hide it, refresh Core, back up its settings, remove it and install/add the new snapshot. A settings-preserving upgrade transaction is future work.
