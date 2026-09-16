# Public weather service

API 2 optionally declares `capabilities:["weather"]` and `refresh:"weather"`.
The Widgets manager presents an Allow/Revoke action explaining that coordinates
are sent to Open-Meteo. CLI: `weather-permission PACKAGE allow|deny`. Grants bind
to the immutable installed generation and expire on update; the renderer cannot
grant itself permission. Only that package's instance can request weather.

`widgetContext.requestWeather(latitude,longitude)` updates `widgetContext.weather`:
`{state,data,updatedAt,refreshing,error}`. States are loading, ready, stale or
unavailable; data contains temperatureC, weatherCode, observedAt and attribution.
The bundled `examples/weather` exercises the contract with independent settings.

Core calls only [Open-Meteo's current weather endpoint](https://open-meteo.com/en/docs),
using fixed fields and validated coordinates rounded to 0.01 degrees. There is
no URL, header, credential, cookie or redirect input. IPv4 DNS results must all
be public; the chosen address is pinned for the TLS request, preserving hostname
verification and preventing DNS rebinding. IPv6-only resolution fails closed.
Curl configuration and environment are cleared; proxies and redirects are unused.
This adds curl and getent as host dependencies. Provider terms apply to deployment;
the public example is not an account-linked or commercial weather service.

A supervisor-wide public cache shares identical coordinates only after each
request passes its own generation grant. It holds at most 128 locations, refreshes
after 15 minutes, deduplicates in-flight requests and starts at most one fetch
per 10 seconds. DNS has a bounded process timeout; HTTPS has a 64-KiB limit and
six-second watchdog. Failure retains data and backs off for one minute. Cached
data is volatile across Core restart. The cache-full response is explicit.

Hidden/workspace-inactive instances cannot start new fetches. Grant revocation,
removal and generation changes deny subsequent reads; an already-started bounded
public request may finish into the shared cache. Revocation does not promise to
recall bytes already delivered. Renderers retain network denial.

Portable tests cover deduplication, hidden gating, backoff, stale preservation,
coordinate validation, private-address rejection and cache bounds. The production
HTTP path was not exercised against the live provider in this environment.
