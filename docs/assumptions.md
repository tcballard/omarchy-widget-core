# Core v0.0.2 review and assumptions

The previous experimental architecture allowed arbitrary package dimensions, discarded commands while the helper was busy, recreated widget windows after snapshots, had no per-editor save acknowledgement, used crash-stale directory locks, and delegated replacement to individual package installers.

v0.0.2 addresses these with fixed families, logical snap coordinates, stable instance identities, serialized/coalesced operations, revisioned durable settings, kernel writer locks, and immutable package versions switched through an atomic registry document. It also moves widget execution into a separate supervised Quickshell process.

Assumptions:

- Widgets remain separate packages/repositories; Core is the shared runtime and manager.
- One definition per package is sufficient now; the explicit definition ID is `main`.
- Small, medium and large cover the initial product. No arbitrary free resizing.
- A 16-point grid aligns placement; overlap is allowed. Automatic collision resolution is future work.
- Settings objects are small (8 KiB); the registry is bounded to 1 MiB / 128 instances / 64 active packages.
- Package code runs in separate Bubblewrap islands with scoped state access and filtered Wayland connections. This development boundary still requires security review and desktop acceptance; resources are limited as a shared service group.
- World Clock remains a single multi-city widget. Its own API 2 layout/editor migration is separate from this Core change.
- A version rollback changes code, not user settings. Widget authors must version their own settings migrations.
- Complete version directories may remain after interruption or removal. Automatic garbage collection is deferred to avoid deleting code used by a running host.
- Display positions use monitor names and a fallback screen. Live compositor behaviour, work-area boundaries and fractional scaling must pass desktop acceptance.
- Core consumes active theme files and uses its own components; no official new Omarchy theme API is claimed.

The initial 0.1.x tags do not imply stability. This development line is 0.0.2. Before 0.1.0, require successful desktop acceptance, a World Clock migration exercising the contract, reliable upgrade/recovery evidence, and an explicit decision about whether supported packages remain trusted or require sandbox enforcement.
