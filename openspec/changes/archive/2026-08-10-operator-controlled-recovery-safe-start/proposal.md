## Why

DiskWeave has portable identity, verification, recovery, checksum, admission, and Linux publication semantics, but no production operator workflow that composes them from read-only inspection through safely published service. The current SQLite open path can mutate during observation, uncertain commits lack durable reopen reconciliation, and mandatory post-recovery checksum work is not yet an end-to-end writable-admission gate.

## What Changes

- Add one production `dwv` operator workflow for `status`, `members`, `scrub`, `damage`, recovery preview/apply, checksum-baseline continuation, and `start`.
- Replace the ad hoc hand-written parser with the established `clap` parser, keep `dwv` as the executable boundary, and route retained demo/acceptance commands through the same typed command, result, outcome, and rendering infrastructure rather than preserving a parallel low-quality CLI.
- Produce human output by default and explicit versioned JSON from one bounded semantic result, preserving success, usage error, refusal, blockage, unsupported capability, definite failure, and reconciliation-required outcomes.
- Add one bounded versioned `array.json` declarative-policy input so repeated offline commands can locate recovery state and candidate members without treating paths, desired bindings, or configuration as current array authority.
- Add non-mutating recovery inspection with distinct absent, supported, corrupt/unreadable, unsupported, migration-required, and reconciliation-required classifications.
- Reconcile uncertain recovery commits after release and reopen against exact validated prior and proposed semantic states.
- Persist and rehydrate current checksum baseline identity, coverage, generation, validity, and completion; require complete current coverage before read/write admission when metadata-loss recovery created that obligation.
- Revalidate proposal, identity, topology, recovery state, and exhaustive payload verification at recovery apply time; preserve zero data/parity writes for the supported all-data/single-P matching case.
- Report online/read-write only after portable admission and actual publication through the existing narrow Linux ublk profile; report unsupported publication truthfully elsewhere.
- Preserve current whole-known-missing degraded reads, repair/rebuild scope, SQLite support claim, and Linux publication profile without broadening them.

## Capabilities

### New Capabilities
- `operator-recovery`: Production operator command composition, bounded assessment/action results, stable outcomes, rendering, recovery-plan freshness, and admission-versus-publication reporting.

### Modified Capabilities
- `recovery-state-semantics`: Add observational inspection and exact prior/proposed reconciliation after uncertain commit observation.
- `checksum-plane`: Define persisted, current, complete checksum-baseline evidence and conservative partial/reopen behavior.
- `healthy-portable-io`: Make an outstanding mandatory checksum baseline a read/write assembly and request-admission barrier without changing unrelated healthy admission.
- `linux-ublk-frontend`: Bind the online/read-write product result to successful publication within the existing narrow supported profile.

## Impact

Affected areas include the root `dwv` command application, its versioned declarative array-policy input, `dwv-recovery`, `dwv-recovery-sqlite`, `dwv-service`, `dwv-verify`, `dwv-frontend-ublk`, retained demo/acceptance command routing, focused adapter/application/CLI evidence, canonical requirement links, and verification manifests. `clap` is added as the sole new third-party dependency; no custom CLI library, generic daemon/job framework, management catalog, force mode, persistent compatibility shim, or platform profile is introduced.
