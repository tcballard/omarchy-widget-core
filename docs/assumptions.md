# Architecture assumptions

1. Widgets are their own product/package category. Each widget has its own repository, version and `widget.json`. Core owns the common runtime. No individual widgets ship here.
2. Start with actual native desktop surfaces. QML content runs within Quattro's existing Quickshell process. Core uses the current service loader as a replaceable bootstrap adapter; this does not establish official upstream support.
3. The host and storefront have different jobs. This repo implements hosting plus a manager of already-installed packages. A future catalogue supplies discovery and verified distribution metadata; the host remains usable without it.
4. Install and add are distinct actions. Copying a local package does not execute QML. Adding explicitly loads trusted code. There is no sandbox in this version.
5. Core owns surfaces, theme scaling, placement, monitor fallback, lifecycle and saved settings. Widgets own domain content, data sources, settings semantics and error/empty states. Provider credentials are not part of Core's settings design.
6. One package gets one desktop placement initially. World Clock is one widget showing multiple city/timezone rows, inspired by Bloomberg Launchpad. Its eventual repo should handle timezone/DST logic internally.
7. Local package snapshots are enough to prove the contract. Automatic remote installation, upgrades, signed artifacts, marketplace submissions and a skills bundle come after the first real widgets expose the gaps.
8. User-level XDG directories hold the widget registry. Core removal does not imply deleting widget packages. A single writer lock and atomic JSON replacement protect ordinary settings writes. Package removal and layout update are separate filesystem operations; power-loss recovery across both is not transactional.
9. Core API 1 is provisional. Keep the boundary small, document changes and use real desktop checks before declaring compatibility. Current targets are Quattro's service facade, theme tokens and Wayland layer-shell APIs.

## What this proves next

Install Core on the target Omarchy machine, verify its empty manager, then build a separate World Clock against the package contract. Exercise multiple timezone rows, sizes, restart persistence and monitor switching. A Sports widget can then test asynchronous producers and stale-data behavior without embedding a sports provider in Core.

## Sources for the integration shape

- [Omarchy Quattro shell](https://github.com/omacom/omarchy/tree/quattro/shell): service injection, semantic theme tokens and existing shell runtime. The inspected tree was `f2b419d9a9d7e7821de2ddf9c42991e32cf06cdf`; this identifies a tree, not a compatibility release.
- [Initial desktop-widget reference](https://github.com/cyelis1224/omarchy-desktop-widgets): the desktop-widget idea shared at the start of this exploration. This repo is a separate experiment.
