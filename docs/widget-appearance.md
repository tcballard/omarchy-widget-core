# Widget appearance · v0.0.2

Core's standalone host reads the active Omarchy `colors.toml` foundational palette from `$XDG_STATE_HOME/omarchy/current/theme`, using `~/.local/state` by default. It accepts flat quoted hex values for foreground, background, accent and red. Unsupported or missing values use safe defaults. This intentionally consumes published theme data without importing the main shell engine.

The optional `widgets.json` alongside it is a **Core-owned extension**, not an official Omarchy theme API:

```json
{
  "borderAlpha": 0.08,
  "backgroundAlpha": 1,
  "padding": 12,
  "fontFamily": "Inter",
  "scale": 1,
  "showTitle": false
}
```

Core inherits border size and rounding from resolved global Hyprland settings, overriding legacy radius/borderWidth appearance tokens. Border alpha is bounded 0–1, background alpha 0.6–1, padding 8–24 and scale 0.75–2. Theme scale affects content typography and padding, not cell sizes or desktop gaps. Per-instance appearance settings cannot override the shared frame. Legacy values remain stored but only the desktop theme/ricing controls exterior presentation. The font must exist on the machine; Qt supplies a fallback otherwise.

The idle frame follows desktop rounding/borders and has no title by default. Arrange exposes controls inside the same outer dimensions. GPU rendering masks the whole surface to its rounded outline. Software rendering uses the rounded background with content inset far enough to remain inside the corners; it does not depend on unsupported shader effects. Core owns content padding; widget authors should avoid painting a second card background.

Trusted Core supplies theme values through broker snapshots; theme switches and ricing changes refresh within about one second without remounting the sandbox. Geometry, controls and fonts are Core's own components; this release does not implement the shell's entire `shell.toml` styling vocabulary.

Store theme sources in the theme repository. The active directory is generated/staged by Omarchy; verify how a custom `widgets.json` is carried into that directory on the installed Omarchy revision. Missing files are optional and use defaults.
