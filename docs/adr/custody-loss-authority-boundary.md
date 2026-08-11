# ADR: Custody-loss authority remains fail-closed in v0.8

**Status:** Accepted
**Date:** 2026-08-10
**Scope:** Metadata-loss recovery, operator recovery, and writable publication after DiskWeave loses custody of member state

## Context

The original M9 implementation used declarative member policy plus exhaustive parity agreement to create fresh recovery state for a policy-described array. Adversarial review established that those facts prove only a selected current XOR equation. They do not prove historical array identity, topology epoch, assignment instances, coding-position history, ownership continuity, or absence of out-of-band writes while DiskWeave lacked custody.

The archived `2026-08-10-operator-controlled-recovery-safe-start` change records that attempted implementation. The archived `2026-08-10-close-m9-authority-boundaries` change corrected the canonical boundary and removed the production transition. Those archives remain historical evidence, not current authority.

## Decisions

1. DiskWeave v0.8 SHALL NOT define or implement authority re-establishment after custody loss.
2. Policy, expected identities, locators, surviving payload readability, and parity/checksum agreement may support bounded observation and verification, but SHALL NOT manufacture prior-lineage authority or authorize writable publication.
3. The current all-metadata-lost/all-data-present production path is non-executable and fails closed before fresh recovery state or payload/parity mutation.
4. A future v0.9 architecture change must select and specify one coherent authority model before implementation. Candidate models are:
   - explicit adoption into a new array lineage, without claiming historical continuity;
   - recovery of a prior lineage from independent authority-bearing evidence;
   - a deliberately specified combination with disjoint authorization rules.
5. This ADR selects none of those candidates. It also does not define historical checksum retention, stale-parity reuse, safety under external writes, post-custody verification failure semantics, or fast writable re-entry.
6. Existing safe behavior remains available where sufficient current authority already exists: observation, exhaustive read-only verification, exact baseline/admission checks, and publication only after current admission plus actual frontend publication.

## Consequences

- M9 can complete only as a fail-closed operator safety boundary; it cannot claim a successful lost-custody recovery-to-start path.
- `recover preview` may explain the missing authority and future operation class. `recover apply` must refuse the current lost-custody case.
- No current recovery certificate, receipt, adoption token, or lineage reconstruction mechanism is implied.
- v0.9 owns the architectural question rather than inheriting a temporary matrix case or implementation shortcut.

## Reversal evidence

Replace this decision only through a canonical OpenSpec and architecture change that defines the chosen authority source, external-write threat model, lineage semantics, checksum/parity treatment, failure transitions, admission boundary, and adversarial evidence. Passing XOR verification alone is insufficient reversal evidence.
