# Widget appearance — Core 0.1.1

Core owns the card frame; widgets receive the same resolved appearance through
`widgetContext.appearance`. Palette colours and scale remain live bindings to
Omarchy's `Color` and `Style`. This is a Core extension, not an upstream
`shell.toml` section. No global theme changes are required.

Theme authors can ship a root **widgets.json** alongside colors.toml:

```json
{
  "radius": 16,
  "borderWidth": 1,
  "borderAlpha": 0.08,
  "backgroundAlpha": 0.98,
  "fontFamily": "sans-serif",
  "separatorAlpha": 0.05,
  "showTitle": false
}
```

This is a starting point for a soft Familiar-style treatment. It is not an
exact reproduction of that theme. No shadow or compositor blur is added.
Transparency changes only the background, not text opacity.

Core reads the staged file at `$XDG_STATE_HOME/omarchy/current/theme/widgets.json`
(default `~/.local/state/omarchy/current/theme/widgets.json`) when refreshing.
Edit the source theme, reapply it, then run:

```bash
omarchy-shell io.github.tcballard.widget-core refresh
```

If your theme installer does not stage this additional file, use the per-widget
override below. Colours/scale update through normal shell bindings; JSON
appearance overrides are refreshed explicitly, not by a background watcher.

The optional `appearance` object in each widget's saved settings overrides the
theme file. Its other settings must be retained when configuring it through CLI.
World Clock's city editor preserves this object when saving cities.

Supported values:

| Key | Range / default | Owner |
| --- | --- | --- |
| radius | 0–40 screen logical pixels; default max(theme rounding, scaled 12) | Core frame |
| borderWidth | 0–3 pixels; default 1 | Core frame |
| borderAlpha | 0–1; default 0.12 | Core frame |
| backgroundAlpha | 0.6–1; default 1 | Core frame |
| showTitle | true to retain idle title; default false | Core frame |
| fontFamily | fontconfig family; default shell family | Frame / participating widget labels |
| separatorAlpha | 0–1; default 0.07 | World Clock rows |

Use radius 0 and showTitle true for the original angular presentation. Arrange
mode always restores a visible accented frame, header and movement controls.
Open the manager or use the `arrange` IPC command to enter it. Core limits the
theme file to an 8 KiB JSON object and falls back with an error if malformed.

Widgets should support shared appearance keys rather than interpreting theme
names. Clock digits retain the shell's fixed-width font for alignment.
