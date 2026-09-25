# Updates, settings and package faults

A package may declare `settingsVersion` (positive integer, default 1),
`settingsSchema` and `migrations`. Schema support is deliberately bounded:
object, array, string, number, integer, boolean; properties, required,
additionalProperties, items, maxItems, maxLength, minimum, maximum and enum.
Unknown schema keywords are rejected. This is a subset, not general JSON Schema.
Defaults, saved settings, migrated settings and rollback settings are validated.

Example version 2 migration:

```json
{"settingsVersion":2,"migrations":[{"from":1,"to":2,"rename":{"city":"label"},"defaults":{"format":"24h"}}]}
```

Each step advances exactly one version. Operations rename top-level fields,
add absent defaults or remove explicitly listed keys. Rename collisions fail;
Core never runs package migration scripts. Missing steps or invalid results
leave the active version and every instance unchanged. Source changes during
staging also fail before pointer activation.

The atomic registry commit includes the immutable code pointer, migrated settings
and before/after settings checkpoints. Identity, placement, workspace and hidden
state are preserved. Settings revisions advance when migration changes data or
schema, invalidating old editors. One previous code/settings checkpoint is kept;
the existing 1-MiB registry cap still applies, so a large checkpoint may cause an
update to be rejected without changing the old registry. Orphan version files
are harmless and not automatically deleted.

`rollback PACKAGE` (also in Widgets → Available) checks family compatibility and
restores the matching settings checkpoint when the schema changed. Later moves,
hides and instance removals survive. Later settings edits or newly added instances
with an incompatible schema cause rollback to fail with an explanation; it never
silently overwrites them. Export the current `list` response and install a forward
compatible package in that case. A same-schema rollback keeps compatible edits.
Rollback can be reversed again using the swapped checkpoint. Weather permission
is revoked; code rollback does not revive an old grant.

## Fault controls

The Available tab reports running, retrying, failed, disabled or idle package
runners. Health expires after five seconds without the supervisor heartbeat.
`package-control PACKAGE restart|disable|enable` is trusted-manager-only. Restart
resets that package's retry budget; disable stops its runner while preserving all
instances and their enabled preferences. New code generations reset failed starts.
Three failures stop automatic retries, with five seconds between attempts.

Containment remains **per package**, including all its instances, proxy and child
processes. The existing systemd cgroup enforces CPU, memory and task limits.
A bad instance can disrupt siblings in its package; unrelated package runners
are separate. The trusted manager, compositor and OS remain shared dependencies.
A QML load error is shown within the widget; runner health is process health,
not a guarantee that every content component loaded successfully.

Portable tests inject missing migrations, rename conflicts, schema-invalid saves,
and rollback-after-edit conflicts, and verify unchanged registry state. They also
exercise checkpoint reversal, visibility preservation and package control serials.
Actual runaway-process resource containment remains covered by the existing
resource CI job. Live desktop crash/restart presentation remains unverified.


Final integration: every gear/manager settings request registers with Core before
loading. Updates and rollback refuse while that package has an open editor.
An explicit package restart/disable clears a stuck request and discards volatile
drafts; use Save/Cancel first when the process is healthy.
