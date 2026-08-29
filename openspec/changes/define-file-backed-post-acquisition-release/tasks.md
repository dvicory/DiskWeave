## 1. Reconcile the file-backed owner target

- [x] 1.1 Review the added post-acquisition release requirement against the canonical single-writer/partial-acquisition rule and confirm that the existing definite partial cleanup semantics are unchanged.
- [x] 1.2 Review single owner-tracked accepted-set ownership, non-extractable store use, exact claim/acquisition/incarnation/resource/alias/geometry/topology binding, same-store-ID substitution refusal, bounded owner-managed accepted/unresolved quarantine and retry, per-claim `released`/`held`/`uncertain` meaning, aggregate `released-all`/`residual`/`uncertain` derivation, and process-scoped identity/process-loss behavior against affected store-operation, recovery, and startup owners.
- [x] 1.3 Confirm that release owns no startup abandonment policy, writable-recovery disposition, session/epoch rollback, recovery `CLEAN`, checksum, transaction, shutdown, or frontend policy.

## 2. Reconcile dependent startup wording

- [x] 2.1 Replace the active scan-independent startup target's unresolved full-set/later-gate file-backed prerequisite with retention and consumption of this exact accepted ownership value and owner result, while keeping startup as the owner of when abandonment requests release.
- [x] 2.2 Preserve conservative outcomes: `released-all` permits only fresh later acquisition; dropping accepted ownership or receiving `residual` or `uncertain` transfers or retains exact owner custody and blocks same-process inferred retry/reacquisition; process loss uses the existing crash-release rule; durable session/epoch and other owner state remain unchanged by file-backed release.
- [x] 2.3 Keep known partial acquisition under its existing definite release-all rule and keep writable-recovery authority under its separate owner.

## 3. Implement file-backed accepted ownership

- [x] 3.1 Replace the split accepted binding and caller-owned stores with one `AcceptedFileWriterSet` that owns the exact live stores, exposes store operations by non-extracting borrow, and consumes itself for release.
- [x] 3.2 Transfer dropped accepted or unresolved ownership into bounded process-local file-owner quarantine with exact retry and no descriptor forgetting or intentional leaking.
- [x] 3.3 Retain lightweight exact-resource process reservations for definitely released members until aggregate `released-all`, while closing released descriptors normally and keeping retry bookkeeping proportional to live claims rather than retry history.
- [x] 3.4 Preserve exact claim identity, complete payload-before-marker release, partial-failure custody, process-death cleanup, genuine replacement separation, and the existing definite partial-acquisition cleanup boundary.
- [x] 3.5 Add focused regressions for accepted Drop, mixed release, released-descriptor closure, blocked member reacquisition, bounded retry records, final reservation removal, repeated-cycle cleanup, same-store-ID replacement separation, both descriptor locks, and failure outcomes.

## 4. Validate and review

- [x] 4.1 Run strict OpenSpec validation for this change and the affected active startup target; repair only defects within this semantic scope.
- [x] 4.2 Run focused crate tests, workspace tests, strict lint, documentation checks/build, and Rust diagnostics.
- [ ] 4.3 Obtain fresh affected-owner external completion review before canonical synchronization or Bead closure.
- [ ] 4.4 After completion approval, synchronize canonical semantics and archive the change through their normal workflows.
