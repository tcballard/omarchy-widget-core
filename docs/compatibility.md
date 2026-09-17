# Compatibility boundary

API 3 is the first declared contract for the reviewed settings-schema, migration,
acknowledged display-action, shared-weather, command-dependency and common-gear
features. It remains experimental pending target-desktop acceptance.

| Package | This Core | Earlier Core API 2 hosts |
| --- | --- | --- |
| Schema 1 / API 1 | Removed; rejected by validation, installation, update, loading and rollback | Historical implementation |
| Schema 2 / API 2 | Legacy loading and legacy package-owned settings gear | Optional features differ across revisions; not an author target |
| Schema 2 / API 3 | Supported development contract | Rejected at manifest validation before install |
| Future API or unknown `requires` feature | Rejected before install | Unsupported |

New widgets use `coreApi:3`. The manifest's optional `requires` array accepts
`settings-schema`, `settings-migrations`, `acknowledged-actions`, `shared-weather`,
`frame-settings`, and `command-dependencies`; unknown features fail closed. This
array is only legal with API 3. API 3 itself is the boundary old hosts enforce.
The registry JSON wire response remains `api:2` for existing CLI consumers;
`widgetContext.api` is 3. These are separate interfaces.

API 1's manifest conversion and display-context facade have been deleted. Port
development packages to `schemaVersion:2`, `coreApi:3`, `families` and
`defaultFamily`; replace `sizes`/`defaultSize` and use `small`, `medium`, `large`.
Use `widgetContext.family` instead of `sizeName`. Remove `requestInput`,
`inputRequested` and `draftRevision`; settings editors use `settingsContext`,
and display actions save against `settingsRevision`. Previews reject API 1 too.
Already-installed API 1 code is reported as an invalid package and is not run.
Update it with a ported package or uninstall it. Existing layout-file recovery
preserves saved data independently of package API support.

| Runtime layer | Declared target | Evidence |
| --- | --- | --- |
| Core | This remediation branch, API 3 | Rust and offscreen integration tests |
| Qt | Qt 6; component CI pinned to PySide6/Qt 6.11.2 | Offscreen tests only; other Qt versions unverified |
| Desktop | Omarchy 4 Quattro / Hyprland / Quickshell | No installed revision accepted yet |
| Isolation | Linux, Bubblewrap, systemd 254+ and cgroup v2 | Existing dedicated CI gates; separate from desktop acceptance |

`dependencies.commands` is enforced against executable files under `/usr/bin`.
It is not an execution allowlist. Additional QML imports must be packaged through
Arch dependencies and exercised on the target runtime. There is no claim of
Rainmeter skin support, a generic Linux backend, or a tested Qt version range.
Authors must identify their exact tested revisions in their own release notes.
