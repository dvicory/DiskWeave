# ADR: OS-017 checksum scrub and verified repair

- **Status:** Accepted as a portable evidence slice; production scheduler and
  stable checksum-format decisions deferred
- **Date:** 2026-08-08
- **Scope:** Single-parity regular-file scrub and separate-target repair

## Interim decision boundary

This change is accepted and archived as the portable OS-017 evidence slice.
Use the existing exhaustive verifier, checksum evidence, and separate-target
repair seams as the portable OS-017 path. Keep scrub read-only, require one
unique current evidence solution, bind plans to identities and generations,
and require target readback, digest verification, parity verification, and a
durable replacement fence before reporting a successful repair.

This archive does not authorize in-place source-member mutation, online scrub
or rebuild scheduling, P/Q repair, degraded writes, or a stable checksum
format. It also does not publish an authoritative integrity record for a
replacement target until replacement assignment and protected-member mapping
exist.

## Evidence

The disposable CLI classifies clean data/parity, uniquely corrupted data,
uniquely corrupted parity, and simultaneous data/parity corruption. Unique
cases produce a confirmation-bound plan and a verified separate-target result;
simultaneous corruption remains ambiguous and produces no plan.

The representative workflow preserves the source members and emits a
`StoreFenceRef` for the replacement target. The full boundary and commands are
recorded in `docs/verification/os-017-checksum-scrub-verified-repair.md`.

## Consequences

- The portable core now has an executable scrub/repair orchestration seam.
- The replacement target is durable and verified but is not silently promoted
  to an array member.
- Scheduler policy, production format selection, and topology-changing repair
  remain explicit future decisions rather than inferred from this evidence.
