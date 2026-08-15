## Context

See `proposal.md` for motivation and `specs/operator-recovery/spec.md` for the observable contract.

`src/operator.rs::observe` currently treats policy/topology agreement plus recognized members as `members_current`, reports “current parity member(s),” and can advertise read/write start. Those observations establish identity composition, not post-gap custody or a current range basis. `RecoverySnapshot` has no canonical custody-continuity or protection-basis representation. Adding one here would require the unresolved epoch, currentization, durability, and migration semantics deliberately excluded from C0a.

The current operator result is rendered from one `OperatorResult` by JSON serialization and `src/cli.rs::print_human`. The change must keep that single semantic-result boundary.

## Goals / Non-Goals

**Goals:**

- Make the current read-only assessment truthful with the evidence already available.
- Represent every C0a authority dimension with typed, versioned output.
- Keep range aggregation deterministic and bounded.
- Leave an implementation agent no need to consult architecture prose.

**Non-Goals:**

- Persisting custody, epoch, or protection-basis state.
- Teaching existing recovery writers to create current or prior basis evidence.
- Changing start, write, repair, rebuild, recovery-apply, or publication authorization.
- Adding a continuity profile, payload scan, retry, migration, compatibility shim, or second result model.

## Decisions

### 1. Extend the existing operator result; do not create a new capability or persistence model

Add one typed `authority` field to `OperatorResult`. It contains:

- a lineage disposition and reason;
- a custody disposition and reason;
- bounded protection-basis coverage entries keyed by configured parity role and basis, with range count and byte total;
- bounded exact exception ranges when one role has mixed classifications;
- an explicit non-authorization statement.

Use enums for the closed dispositions from the delta spec. Keep reasons as bounded operator explanations, not as authority. Reuse the existing topology, member observations, recovery inspection, report limits, result construction, and renderers.

Alternative: add custody and basis records to `RecoverySnapshot`. Rejected because C0a cannot define who durably writes them or the epoch/currentization transitions that make them authoritative.

Alternative: create a separate post-gap command or result envelope. Rejected because status and related observation commands already own this composition.

### 2. Classify only what current evidence proves

Rename the local `members_current` concept to lineage acceptance and use it only for lineage. Accepted lineage requires a supported recovery manifest whose exact topology matches policy plus one recognized, non-aliased current identity for every assignment. Duplicate, conflicting, or mismatched observations are ambiguous; missing authority is unproved.

The current direct-file profile has no certified continuity evidence. When observation can establish that no enforceable writer owned the stopped raw-member interval, report `gap-observed`; otherwise report `continuity-unproved`. Do not emit `continuity-proved` until an existing canonical continuity owner supplies accepted evidence.

Current persisted recovery state does not encode an exact range-local protection basis. Therefore:

- never infer `current` or `prior` from recognized members, topology, clean shutdown, parity agreement, or checksum state;
- never infer `unprotected` merely from missing evidence;
- report `indeterminate` only for an exact range whose current recovery evidence identifies an unresolved or uncertain transition that prevents one basis from being named;
- report `not-yet-interpretable` for every remaining configured parity-role range, including legacy, absent, corrupt, unsupported, migration-required, or incomplete basis evidence.

This conservative result is complete C0a behavior, not a placeholder. Later C0b/C2 work may supply authoritative basis observations through a separate canonical change; it must not reinterpret these C0a results in place.

### 3. Derive aggregation from one normalized range partition

Build role-local coverage from the configured protected geometry and the exact indeterminate ranges, if any. Split only at existing range boundaries, merge adjacent entries with the same basis, and compute counts and byte totals with checked arithmetic. Apply the existing bounded report limit to exact exceptions. If the limit would be exceeded, retain exact totals and report that exact detail requires the existing bounded export path; do not truncate away the stronger or exceptional state.

Do not add a configuration knob or generalized range framework. The current single-parity profile should produce one whole-range `not-yet-interpretable` entry unless exact indeterminate evidence requires a split.

Alternative: reuse verification `RegionAssessment`. Rejected because verification disposition and protection basis are independent claims.

### 4. Remove identity-derived protection and start advice from observation

`redundancy`, `parity`, `start`, `reason`, and `next_action` must be derived after authority classification. A complete recognized array with unproved custody or no current basis must not say “current parity,” “read-write available,” or recommend `start` as though assessment authorized it. It should report the evidence blocker and name an independently gated authority/reconciliation action. This changes assessment output only; `start` keeps its existing canonical authorization path until later normalization changes it.

### 5. Make a clean structured-output cutover

Change the structured schema identifier from `dwv.operator.v1` to `dwv.operator.v2`, add the required authority object, and update every command result and fixture in one cutover. Human output prints the same authority dimensions, totals, blockers, and non-authorization statement. Do not emit v1 aliases or dual schemas.

No persisted migration is required. Rolling back the code restores the v1 output shape; operators and strict consumers must update atomically with this change.

## Risks / Trade-offs

- Current production evidence will usually yield `not-yet-interpretable` basis after a gap. → Report the exact missing authority and keep future epoch/currentization work separately gated rather than inventing current protection.
- `start` can remain separately callable while status no longer recommends it. → State explicitly that assessment grants no authorization; do not broaden C0a into unresolved start semantics.
- Adding required v2 fields breaks strict v1 consumers. → Use a clean schema-version cutover and update all repository consumers and fixtures in the same implementation change.
- A bounded summary can hide a rare exceptional range. → Preserve exact counts and strongest dispositions, and use the existing bounded detail/export path rather than silent truncation.

## Migration Plan

1. Add the typed authority model and conservative classifier behind the current result construction.
2. Update all operator commands and human/JSON rendering to `dwv.operator.v2` in one change.
3. Update focused CLI fixtures and assertions, then run the complete operator CLI contract test.
4. Roll back the implementation commit if consumers cannot move to v2; no recovery or payload migration is needed.
