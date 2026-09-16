# Widget starter

Develop against Widget Core API 2, using the SDK described in the Core repository.
`View.qml` receives `widgetContext`; `Settings.qml` receives `settingsContext`.
Core owns placement, exterior frame, theme, Save/Cancel and durable persistence.

Validate: `omarchy-widget validate /path/to/this/package`.
Install: `omarchy-widget install /path/to/this/package`.
Open Widgets and add two instances, then configure them differently.

Declare external `/usr/bin` commands in `dependencies.commands`; no command is
run by validation. This declaration does not grant filesystem, network or IPC
access. Use the Core weather capability for the supported public data service.
Use absolute timestamps for timed tasks and stop visual timers when inactive.

Before distributing, change ID/name, include your license, document dependencies,
check all supported families and make deterministic PNG previews. Do not claim
live desktop compatibility based only on manifest validation.
