# Network widgets: proposed next API

Status: design only. This change does not grant widgets network access.

Weather, calendars, RSS and markets are natural widget uses. Keep the renderer sandboxed and let a trusted Core data service fetch declared resources. The widget should receive bounded data and timestamps, not credentials or a general-purpose proxy.

Proposed first scope:

- Read-only public HTTPS JSON/text feeds, with manifest-declared exact origins and user-visible permission review. Start with one weather provider to prove the full path.
- Core-owned request scheduling, minimum refresh intervals, per-package concurrency and byte/time limits, caching and backoff. A stale cached result carries fetched-at and error metadata.
- Broker authority tied to package identity and code generation. A widget cannot claim another package's grants or cache entries.
- Revalidate every redirect and resolved address. Deny loopback, private/link-local/metadata destinations by default, including IPv6 and DNS rebinding paths. Never inherit ambient proxy credentials, browser cookies or arbitrary headers.
- Cache by package, grant and resource identity. A widget update that changes origins requires a new grant; disabling/removing a widget cancels work and revokes its access.
- Public feeds first. Private calendars and account-linked services need a separate credential and OAuth design; secret tokens stay outside widget settings and renderer snapshots.

The candidate QML-facing API should expose named data resources (for example `weather.current`) and statuses such as loading, fresh, stale and denied. Do not publish a concrete API number or promise signatures until a working provider exercises permissions, cancellation, offline behaviour and isolation.

This design preserves the current renderer network denial and does not turn the existing state broker into an arbitrary URL fetch endpoint.
