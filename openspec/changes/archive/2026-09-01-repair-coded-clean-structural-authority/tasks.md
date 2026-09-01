## 1. Durable Semantic Transition Boundary

- [x] 1.1 Replace separable public coded-capture updates/removals for `CLEAN`, later-cut, refused cleanup, inherited abandonment, and live `CleanKnown` cleanup with phase-specific opaque complete transition groups; verify ordinary production callers cannot extract or commit a partial capture successor.
- [x] 1.2 Make the recovery store validate each complete group against the exact durable predecessor, topology/generation, phase, selected geometry, aggregate evidence, every owner-required dirty-region/checksum-extent/integrity-state effect, and proposed successor before applying any member; reject a coded-group transaction containing an ordinary recovery mutation, a same-capture group pair, or incompatible selected-state postconditions on overlapping geometry before candidate application.
- [x] 1.3 Give `CLEAN` and later-cut the same exact predecessor-bound prepare → commit → install and unknown-acknowledgement semantics as release, compaction, and cleanup; verify known noncommit retains the predecessor and may-have-committed outcomes install neither candidate or next live authority before reopen accepts only the exact complete predecessor or successor.
- [x] 1.4 Preserve the permanent incomplete-group regressions and add before/after ordinary co-mutation, same-capture group, and incompatible overlapping-group negative controls for every applicable coded transition kind; add positive compatible distinct-capture group composition coverage.

## 2. Non-Bypassable Authority Producers

- [x] 2.1 Make production coded admission consume an opaque complete claim issued by the canonical geometry owner and bound to the exact admitted member/range/request, operation identity/generation where applicable, topology epoch, and coding profile; isolate arbitrary mapped-unit construction to component tests and add stale/cross-request/generation/topology negative coverage proving a valid claim for another mutation cannot be replayed.
- [x] 2.2 Make final coded authority removal consume the canonical full exact-generation lifecycle release capability; keep `OperationReleasePermit` internal as one prerequisite, migrate every caller, remove the public permit-only release path, and add a compile/export negative control proving a permit alone cannot remove a coded claim.
- [x] 2.3 Make Included lifecycle evidence obtainable only from the production lifecycle owner after exact live finalization or persisted release-receipt validation; remove public issuer-returning/raw-positive issuance paths and add a compile/export negative control proving capture identity, coordinator access, operation token, or fence certificate alone cannot mint positive evidence.
- [x] 2.4 Run an exported-authority audit under ordinary Cargo feature unification and compile/runtime negative controls; verify no deprecated alias, test-support feature, deserialized DTO, scalar, raw issuer, or lower-level prerequisite restores any removed production authority path.

## 3. Production Correspondence and Capacity Progress

- [x] 3.1 Migrate accepted Connect paths to the same production geometry, lifecycle, full release, and complete recovery semantic-transition producers used by `HealthyPortableService`; specifically commit later cuts with their newer dirty/write-recovery boundary and verify model choices never become implementation authority.
- [x] 3.2 Implement `max_coded_captures` exhaustion through existing retained-work backpressure rather than global `Recovering`; with capacity one, verify the second write remains nonterminal and non-filesystem-visible without evidence loss, duplicate admission, or terminal failure, then progresses from its existing admission state after the first capture durably retires and retained work is driven again.
- [x] 3.3 Preserve and rerun the independent coded-geometry oracle, capture-wide aggregate-fence regression, compacted-history quarantine, inherited abandonment, last-write clean closure, long sequential retirement, and lost-acknowledgement cases; repair any conformance break at its owner rather than weakening the checks.
- [x] 3.4 Correct Connect, manifest, and maintained evidence claims to name only executable production correspondence and explicit finite/sample bounds; verify every claimed fault has a maintained command that fails on the injected defect and passes on production.

## 4. Verification and External Completion Gate

- [x] 4.1 Update the bounded Quint/Rust/Connect trace comparison and verification manifest so every claimed conflict, persistence, uncertainty, restart, and capacity fault has a maintained command and exact scoped evidence result; keep bounded non-claims explicit.
- [x] 4.2 Run formatting, affected-crate Clippy with warnings denied, workspace all-target/all-feature check, single-threaded full workspace tests, strict all-OpenSpec validation, knowledge readiness/affected-owner review, documentation check, and documentation build; update bounded non-claims without overstating physical or exhaustive proof.
- [x] 4.3 Prepare a fresh exact-snapshot external handoff after every corrective
  round, including the preceding external `NO-GO`, the complete repair diff,
  authority matrix, permanent regressions, verification output, current
  OpenSpec state, and current Bead state. Treat every returned `NO-GO` as
  required input to the next round.
- [x] 4.4 Apply the returned external packet mechanically. Mark this task complete only after a fresh external `GO` has no open P1/P2 finding; otherwise repair every concrete finding and repeat task 4.3.
- [x] 4.5 Only after task 4.4 is complete, synchronize and archive this corrective change, rerun post-archive documentation/strict validation, and close `dwv-3vz`, `dwv-nto.9.3`, `dwv-nto.9.4`, and `dwv-nto.9`.
