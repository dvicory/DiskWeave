# Milestone 8 canonical semantic ownership — external review follow-up 1

Date: 2026-08-09
Change: `canonical-semantic-ownership`
Starting parent: `pkxqzkzy 62219652 chore: milestone 8 external-review checkpoint 1`
Review state: proposal corrections complete; proposal remains unapplied and blocked on renewed external semantic review.

## A. Corrections made

Only the three requested review corrections were made. No delta was applied, no relationship/fingerprint tooling was implemented, no reviewed fingerprint or evidence mapping was changed, no change was archived, and no successor OpenSpec was started.

### A.1 Tightened `requires` versus `refines`

Final definitions:

- **`requires`**: the source requirement's own semantics depend on an independently owned semantic fact.
- **`refines`**: the target requirement owns the same underlying detailed operational policy and the source defines a narrower specialization or adapter realization of that policy.
- Composition alone is not refinement. A source-target pair may have at most one relation kind.

Reclassified 17 composition/dependency edges from `refines` to `requires`:

- 15 edges from four `healthy-portable-io` requirements to independently owned topology, normalized-request, operation-slot, transaction, recovery, dirty/integrity, store, fence, and lifecycle policies. The XOR dependency was already `requires`.
- 2 edges from `req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback` to the independently owned dirty/integrity and recovery-fence policies. The automatic-repair dependencies on checksum validity and mismatch classification were already `requires`.

The graph still contains eight `refines` edges. Each now represents a genuine specialization or adapter realization; every one is explained in section C.

Files changed for this correction:

- `openspec/changes/canonical-semantic-ownership/specs/healthy-portable-io/spec.md`
- `openspec/changes/canonical-semantic-ownership/specs/checksum-scrub-verified-repair/spec.md`
- `openspec/changes/canonical-semantic-ownership/specs/documentation-knowledge-architecture/spec.md`
- `openspec/changes/canonical-semantic-ownership/design.md`
- `openspec/changes/canonical-semantic-ownership/proposal.md`
- `docs/milestones/m8.md`
- `openspec/changes/canonical-semantic-ownership/tasks.md`

### A.2 Made consequential semantic relationships conditionally mandatory

The proposal now requires authors to declare every relationship that satisfies the normative definitions:

- every consequential independently owned semantic prerequisite requires `requires`;
- every narrower specialization or adapter realization of the same detailed operational policy requires `refines`;
- zero relationships is valid only when no consequential prerequisite, specialization, or adapter realization exists;
- semantic reconciliation authors the edges;
- deterministic tooling validates the authored representation and does not infer missing edges from arbitrary prose.

The requirement now contains positive `requires`, positive `refines`, negative/no-edge, extraction, invalid-graph, transitive-invalidation, and formatting-stability scenarios.

Files changed for this correction:

- `openspec/changes/canonical-semantic-ownership/specs/documentation-knowledge-architecture/spec.md`
- `openspec/changes/canonical-semantic-ownership/design.md`
- `openspec/changes/canonical-semantic-ownership/proposal.md`
- `docs/milestones/m8.md`
- `openspec/changes/canonical-semantic-ownership/tasks.md`

### A.3 Anchored Linux support authority in a canonical requirement

Canonical support authority is now explicit:

- Canonical owner of the initial supported publication profile: `req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow`.
- Evidence owner: `req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate`.
- The evidence requirement now `requires` both the canonical publication-profile owner and `req.evidence-boundaries.evidence-scope-is-explicit`.
- A successful run records environment-specific facts only. It cannot create or broaden a canonically supported profile.

Files changed for this correction:

- `openspec/changes/canonical-semantic-ownership/specs/linux-ublk-frontend/spec.md`
- `openspec/changes/canonical-semantic-ownership/design.md`
- `openspec/changes/canonical-semantic-ownership/proposal.md`
- `docs/milestones/m8.md`
- `openspec/changes/canonical-semantic-ownership/tasks.md`

## B. Complete final relationship graph

Graph validation result:

```json
{
  "total": 49,
  "requires": 41,
  "refines": 8,
  "nodes": 45,
  "unknown": 0,
  "self": 0,
  "duplicate": 0,
  "mixed": 0,
  "acyclic": true,
  "cycle": null
}
```

Exact proposed forward graph, sorted by source, kind, and target:

```text
req.checksum-plane.invalidation-precedes-protected-mutation --refines--> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution --requires--> req.checksum-plane.validity-is-generation-bound
req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution --requires--> req.parity-verification-repair.mismatch-classification-requires-independent-evidence
req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback --requires--> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback --requires--> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed --requires--> req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed --requires--> req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout
req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed --requires--> req.xor-reference-model.every-single-known-erasure-reconstructs-exact-bytes
req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target --requires--> req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed
req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target --requires--> req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority
req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence --refines--> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence --requires--> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
req.dirty-integrity-invalidation.failures-and-restart-are-conservative --requires--> req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible --requires--> req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived
req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived --requires--> req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry
req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded --requires--> req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived
req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence --refines--> req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence
req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence --requires--> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence --requires--> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation --refines--> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative --requires--> req.dirty-integrity-invalidation.failures-and-restart-are-conservative
req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative --requires--> req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative --requires--> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety --requires--> req.dirty-integrity-invalidation.failures-and-restart-are-conservative
req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety --requires--> req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative
req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety --requires--> req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety --requires--> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe --requires--> req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe --requires--> req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe --requires--> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences --requires--> req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence
req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences --requires--> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences --requires--> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity --requires--> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity --requires--> req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity --requires--> req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity --requires--> req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity --requires--> req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity --requires--> req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent
req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics --refines--> req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened
req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics --refines--> req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics --requires--> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate --requires--> req.evidence-boundaries.evidence-scope-is-explicit
req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate --requires--> req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow
req.macos-bridge-feasibility.the-outcome-is-a-bounded-portable-demo-decision --requires--> req.normalized-block-semantics.adapters-expose-bounded-deterministic-conformance-behavior
req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence --requires--> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
req.recovery-state-semantics.evaluation-fixtures-cover-candidate-durability-and-reset-boundaries --requires--> req.volatile-media-simulator.media-state-separates-durable-and-process-visible-effects
req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent --refines--> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.volatile-media-simulator.operations-use-exact-normalized-ranges-and-structured-evidence --refines--> req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence
```

## C. Why every remaining `refines` edge is refinement

1. `req.checksum-plane.invalidation-precedes-protected-mutation` → `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`: the checksum owner specializes the generic invalidation-before-mutation policy into the checksum `VALID` → `STALE` transition.
2. `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence` → `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`: dirty-region checkpoint/clear is the dirty-protocol specialization of the general clean/valid typed-authority policy.
3. `req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence` → `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`: the transaction action specializes the dirty protocol's checkpoint/clear state transition.
4. `req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation` → `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`: transaction action ordering is a state-machine specialization of the canonical invalidation-before-mutation policy.
5. `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` → `req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened`: the Linux adapter realizes the normalized ordering/durability policy at the kernel request boundary.
6. `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` → `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics`: the Linux adapter realizes the canonical frontend-neutral request validation/mapping policy.
7. `req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent` → `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`: the recovery-store transaction is the persistence realization of the canonical durable-intent-before-home-mutation policy.
8. `req.volatile-media-simulator.operations-use-exact-normalized-ranges-and-structured-evidence` → `req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence`: the simulator is an adapter realization of the canonical store outcome/persistence-evidence contract.

No remaining `refines` edge merely means that the source calls, composes, or consumes another owner. Those associations now use `requires` or remain implicit when non-consequential.

## D. Exact mandatory relationship requirement

```markdown
### Requirement: Canonical semantic relationships are colocated and derived
<!-- dwv:req req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry -->

Current canonical requirements SHALL declare a forward `requires` relationship for every consequential independently owned semantic prerequisite needed to interpret or implement the local requirement, and SHALL declare a forward `refines` relationship when the local requirement is a narrower specialization or adapter realization of another requirement's detailed operational policy. Requirements with no such consequential relationship SHALL declare none. `requires` means the source requirement's own semantics depend on an independently owned semantic fact. `refines` means the target requirement owns the same underlying detailed operational policy and the source defines a narrower specialization or adapter realization of that policy. Composition alone SHALL NOT qualify as refinement.

Semantic reconciliation SHALL determine and author the consequential relationships required by the local semantics. Deterministic tooling SHALL validate the authored representation and SHALL NOT infer a missing semantic relationship from arbitrary English prose.

Relationship markers SHALL be contiguous immediately after the intrinsic identity. A source-target pair SHALL use at most one relation kind, and every target SHALL resolve to a current canonical requirement ID. The tool SHALL derive backlinks, capability aggregation, bounded ownership views, and owner-before-dependent reading order from forward markers without a second registry or manual backlink list. Unknown, non-current, self, duplicate, mixed-kind, misplaced, or cyclic relationships SHALL fail deterministically with stable source and cycle diagnostics.

Universal constitutional, security, and evidence constraints SHALL NOT require repeated edges on every detailed requirement. Lexical or terminology similarity, shared tests or evidence, implementation call graphs, and optional or future work SHALL NOT create semantic edges. Reverse relationships SHALL remain derived.

Each requirement SHALL have a local semantic fingerprint that is a formatting-stable digest of its local normative prose and scenarios, excluding intrinsic and relationship marker comments and non-semantic formatting. After validating that the combined semantic graph is acyclic, the tool SHALL compute each effective semantic/review fingerprint prerequisite-first as a deterministic digest of the local semantic fingerprint, sorted outgoing `(relation kind, target ID)` pairs, and each target's effective semantic/review fingerprint. Reviewed state SHALL store and compare the effective semantic/review fingerprint. Formatting-only changes and changes only to review outcome or reason SHALL preserve semantic fingerprints. A local semantic or outgoing-edge change SHALL make every direct and transitive semantic dependent review-suspect. Every suspect requirement SHALL require an individual semantic review and concrete reason; bulk acceptance SHALL remain forbidden.

#### Scenario: A consequential independently owned prerequisite exists

- **WHEN** semantic reconciliation establishes that a local requirement needs an independently owned semantic fact for interpretation or implementation
- **THEN** the local requirement declares a forward `requires` relationship to that fact before semantic review may approve it

#### Scenario: A narrower specialization or adapter realization exists

- **WHEN** semantic reconciliation establishes that the target owns the same detailed operational policy and the local requirement narrows or realizes that policy
- **THEN** the local requirement declares a forward `refines` relationship and does not also declare `requires` for the same source-target pair

#### Scenario: No consequential semantic relationship exists

- **WHEN** a requirement has no consequential prerequisite, specialization, or adapter realization beyond universal constraints, lexical similarity, shared evidence, implementation calls, or optional future work
- **THEN** it declares no relationship for those non-semantic associations

#### Scenario: Authored relationships are extracted

- **WHEN** required forward markers are present and structurally valid
- **THEN** extraction exposes the forward edges, derived backlinks, bounded owner-before-dependent order, local fingerprints, and effective semantic/review fingerprints without a manual backlink record

#### Scenario: The relationship graph is invalid

- **WHEN** an edge has an unknown or non-current target, is self-referential, duplicates or conflicts with another edge, is misplaced, or participates in a cycle
- **THEN** readiness fails with source path, line, source ID, relation kind, target ID, and the smallest deterministic discovered cycle when applicable

#### Scenario: An owner's local semantics change

- **WHEN** requirements B and C transitively depend on owner A, unrelated requirement D has no path to A, and A's normalized local prose or scenarios change while B, C, and D remain locally unchanged
- **THEN** B and C receive changed effective semantic/review fingerprints and `semantic-prerequisite-changed` diagnostics until individually reviewed while D's effective semantic/review fingerprint remains stable

#### Scenario: Only owner formatting changes

- **WHEN** an owner's semantics and relationships are unchanged but Markdown formatting changes
- **THEN** local and dependent effective semantic/review fingerprints remain unchanged
```

## E. Exact Linux support authority

Canonical profile owner, unchanged current requirement:

```markdown
### Requirement: The initial Linux publication profile is complete and narrow
<!-- dwv:req req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

The initial Linux profile SHALL publish exactly one writable endpoint only when the validated topology contains exactly one data slot, one parity slot, and all required recovery authority. The endpoint SHALL target that explicit stable data slot while the portable topology remains variable-width. A wider or otherwise unsupported valid topology SHALL be reported as unsupported before publication or mutation; the adapter SHALL NOT publish a subset, classify the topology itself as invalid, or encode the profile limit in portable APIs or persistent state.
```

Corrected acceptance/evidence requirement:

```markdown
### Requirement: Live ext4 acceptance evidence is bounded and scope-accurate
<!-- dwv:req req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate -->
<!-- dwv:requires req.evidence-boundaries.evidence-scope-is-explicit -->
<!-- dwv:requires req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

A Linux acceptance profile already declared supported by current canonical Linux requirements SHALL run a reproducible bounded workflow that creates the disposable fixture, publishes a real DiskWeave-owned ublk endpoint, formats and mounts ext4, performs bounded create/overwrite/rename/fsync/read/delete operations, verifies exact content, unmounts and cleanly stops, restarts against the same fixture, remounts and verifies retained content, inspects parity/recovery/integrity disposition, and mounts the ordinary data member read-only after final shutdown. Evidence SHALL record the exact environment, configured bounds, source digest, and correlated kernel submission, normalized request, semantic result, and completion without payload bytes, raw pointers, or private host paths. A successful run in any environment SHALL NOT create or broaden a canonically supported profile.

#### Scenario: A canonically supported profile passes

- **WHEN** every required operation and lifecycle transition succeeds for a profile already supported by current canonical Linux requirements
- **THEN** evidence may claim functional file-backed Linux ublk/ext4 behavior only for that canonical profile in the recorded environment and does not establish another supported profile

#### Scenario: Live execution is unavailable

- **WHEN** the required architecture, VM, kernel capability, permission, or tool is unavailable
- **THEN** deterministic adapter tests may pass but Linux frontend and ext4 acceptance remain explicitly unmet rather than skipped as success

#### Scenario: The canonical profile passes in another environment

- **WHEN** the same canonically supported profile completes successfully in an additional environment
- **THEN** evidence records that environment but the successful run does not create or broaden canonical Linux support

#### Scenario: A stronger claim is requested

- **WHEN** evidence is used to infer production concurrency, daemon recovery, raw-device durability, FUA, broader filesystems, deployment correctness, online topology mutation, or hardware safety
- **THEN** the report identifies those claims as unsupported
```

Authority split:

- Canonical requirement owns whether a publication/acceptance profile is supported.
- Verification record owns the exact architecture, guest, kernel, endpoint, queue/transfer bounds, source digest, trace counts, tool versions, and observed pass/fail facts for a run.
- A run in another environment is additional evidence for the same canonical profile, not a new profile and not a support-policy mutation.

## F. Recovery uncertainty contract

The requested conservative distinctions remain explicit and unchanged except for clarifying the `refines` marker target:

```markdown
### Requirement: Home mutation requires durable dirty and integrity invalidation intent
<!-- dwv:req req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent -->
<!-- dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation -->

The semantic store SHALL atomically persist the affected dirty regions and stale integrity extents at one generation before a caller may rely on that transaction as the owner's durable invalidation intent. Dirty and stale generations SHALL survive in-memory process loss until a later explicit recovery transaction changes them. A rejected or generation-mismatched transaction leaves the prior snapshot authoritative. A lost, corrupt, or indeterminate commit observation establishes no authoritative resulting snapshot for the caller, provides no permission for protected home mutation, and requires reconciliation through the recovery commit-observation contract.

#### Scenario: Recovery intent commit is rejected before authoritative commit

- **WHEN** the transaction is rejected or generation-mismatched before authoritative commit
- **THEN** the proposed transaction does not become authoritative, the prior snapshot remains authoritative, and the caller receives no permission for home mutation

#### Scenario: Recovery intent commit observation is lost, corrupt, or indeterminate

- **WHEN** commitment may have occurred but its acknowledgement is lost, corrupt, or indeterminate
- **THEN** neither the prior nor proposed resulting snapshot may be assumed authoritative for protected mutation, no home-mutation permission exists, and reconciliation through the recovery commit-observation contract is required
```

Resulting cases:

- Rejected or generation-mismatched before authoritative commit: the prior snapshot remains authoritative.
- Lost/corrupt/indeterminate commit observation: neither prior nor proposed resulting snapshot may be assumed authoritative for protected mutation.
- In both cases: no protected home-mutation permission; the uncertain case additionally requires recovery commit-observation reconciliation.

## G. Dependency-closure fingerprint contract

Exact final design:

```markdown
**Local semantic fingerprint:** a formatting-stable digest of the local requirement's normative prose and scenarios, excluding intrinsic and relationship marker comments and non-semantic formatting.

**Effective semantic/review fingerprint:** after validating the combined semantic graph is acyclic, compute prerequisite-first a deterministic digest of the local semantic fingerprint, sorted outgoing `(relation kind, target ID)` pairs, and each target's effective semantic/review fingerprint.

Reviewed state stores and compares the effective semantic/review fingerprint under a bumped schema. Each stale requirement requires an individual review and concrete reason; bulk acceptance remains forbidden. Changing only a review outcome or reason does not affect either semantic fingerprint.

For `A → B → C` where B imports A and C imports B:

- changing A's local semantics changes A's effective semantic/review fingerprint, then B's, then C's;
- a formatting-only change to A preserves the effective semantic/review fingerprints of A, B, and C;
- unrelated D remains stable;
- changing B's outgoing semantic edge set changes B and C, but not unrelated A or D unless the new edge itself imports them.

`knowledge affected`, readiness, and `docs check` report `semantic-prerequisite-changed` for every direct and transitive stale dependent.
```

This remains dependency-closure invalidation, not direct-dependent-only invalidation. The effective digest includes the relation kind, target ID, and target effective digest; a relation-only change therefore propagates transitively even when local prose is unchanged.

## H. Ownership and scenario review

Observed review results after the corrections:

1. All five source requirements whose 17 edge kinds changed were reviewed across their 12 complete local scenarios:
   - four `healthy-portable-io` requirements with 15 reclassified edges;
   - one `checksum-scrub-verified-repair` requirement with two reclassified edges.
2. Their normative prose and scenarios did not move or change. Only their relationship markers changed. The scenarios continue to test local composition behavior while the independently owned policies they consume are now `requires` targets.
3. All eight remaining `refines` edges were reviewed individually; section C gives the operational policy and narrower realization for each.
4. The mandatory-relationship requirement owns seven local scenarios covering positive `requires`, positive `refines`, no-edge, extraction, invalid graph, transitive semantic invalidation, and formatting-only stability.
5. Linux acceptance scenarios remain under the evidence requirement. Canonical profile selection remains under the initial publication-profile requirement. The new `requires` edge makes that authority dependency explicit without moving environment facts into the canonical owner.
6. Recovery rejection and uncertain-observation scenarios remain distinct and colocated with the recovery-store owner.
7. Proposed graph checks found zero unknown targets, self edges, duplicate edges, mixed-kind pairs, or cycles.
8. No scenario needs migration solely because `refines` became `requires`; relationship kind changes ownership/import semantics, not the local behavior being exercised.

## I. Exact validation evidence

### I.1 Strict all-change OpenSpec validation

Command:

```text
openspec validate --strict --all
```

Output:

```text
- Validating...
✓ spec/anchorless-topology-identity
✓ spec/architecture-contract
✓ change/canonical-semantic-ownership
✓ spec/checksum-plane
✓ spec/checksum-scrub-verified-repair
✓ spec/degraded-read-offline-rebuild
✓ spec/dirty-integrity-invalidation
✓ spec/documentation-knowledge-architecture
✓ spec/evidence-boundaries
✓ spec/explicit-transaction-machine
✓ spec/file-backed-stores
✓ spec/healthy-portable-io
✓ spec/linux-ublk-frontend
✓ spec/macos-bridge-feasibility
✓ spec/macos-demo-cli
✓ spec/metadata-loss-recovery
✓ spec/normalized-block-semantics
✓ spec/normalized-trace-replay
✓ spec/parity-envelope-profiles
✓ spec/parity-verification-repair
✓ spec/recovery-state-semantics
✓ spec/security-boundaries
✓ spec/store-operation-contracts
✓ spec/transaction-engine-evidence-comparison
✓ spec/volatile-media-simulator
✓ spec/xor-reference-model
Totals: 26 passed, 0 failed (26 items)
```

### I.2 Unchanged current-state knowledge readiness

Command:

```text
cargo xtask docs knowledge readiness
```

Output payload:

```json
{
  "command": "knowledge",
  "diagnostics": [],
  "ok": true,
  "outcome": "success",
  "result": {
    "diagnostics": [],
    "gate_counts": {
      "fingerprint-suspect": 0,
      "historical-reference": 0,
      "mapping-collision": 0,
      "orphaned-state": 0,
      "uncovered": 0,
      "unknown-reference": 0
    },
    "next_actions": [],
    "ready": true,
    "requirements": 152,
    "schema": "dwv.knowledge.readiness.v1"
  },
  "schema": "dwv.docs.cli.v1"
}
```

This validates the unchanged current canonical state only. It is not represented as validation of the still-unimplemented relationship/fingerprint proposal semantics.

### I.3 OpenSpec artifact status

Command:

```text
openspec status --change canonical-semantic-ownership --json
```

Exact artifact-state fields:

```json
{
  "changeName": "canonical-semantic-ownership",
  "schemaName": "spec-driven",
  "artifacts": [
    {"id": "proposal", "outputPath": "proposal.md", "status": "done", "requires": []},
    {"id": "specs", "outputPath": "specs/**/*.md", "status": "done", "requires": ["proposal"]},
    {"id": "design", "outputPath": "design.md", "status": "done", "requires": ["proposal"]},
    {"id": "tasks", "outputPath": "tasks.md", "status": "done", "requires": ["specs", "design"]}
  ]
}
```

`status: done` means all planning artifact files exist. It does not override task `0.4` or authorize application.

## J. Working-copy evidence

Final `jj status`:

```text
Working copy changes:
M docs/milestones/m8.md
M openspec/changes/canonical-semantic-ownership/design.md
M openspec/changes/canonical-semantic-ownership/proposal.md
M openspec/changes/canonical-semantic-ownership/specs/checksum-scrub-verified-repair/spec.md
M openspec/changes/canonical-semantic-ownership/specs/documentation-knowledge-architecture/spec.md
M openspec/changes/canonical-semantic-ownership/specs/healthy-portable-io/spec.md
M openspec/changes/canonical-semantic-ownership/specs/linux-ublk-frontend/spec.md
M openspec/changes/canonical-semantic-ownership/tasks.md
Working copy  (@) : xnvkkyxm 73d8031e (no description set)
Parent commit (@-): pkxqzkzy 62219652 chore: milestone 8 external-review checkpoint 1
```

Final `jj diff --summary`:

```text
M docs/milestones/m8.md
M openspec/changes/canonical-semantic-ownership/design.md
M openspec/changes/canonical-semantic-ownership/proposal.md
M openspec/changes/canonical-semantic-ownership/specs/checksum-scrub-verified-repair/spec.md
M openspec/changes/canonical-semantic-ownership/specs/documentation-knowledge-architecture/spec.md
M openspec/changes/canonical-semantic-ownership/specs/healthy-portable-io/spec.md
M openspec/changes/canonical-semantic-ownership/specs/linux-ublk-frontend/spec.md
M openspec/changes/canonical-semantic-ownership/tasks.md
```

Final `jj diff --stat`:

```text
docs/milestones/m8.md                               | 37 ++++-----
...c/changes/canonical-semantic-ownership/design.md | 91 +++++++++++++----------
...changes/canonical-semantic-ownership/proposal.md | 14 +--
...hip/specs/checksum-scrub-verified-repair/spec.md |  4 +-
...ecs/documentation-knowledge-architecture/spec.md | 49 ++++++++----
...ntic-ownership/specs/healthy-portable-io/spec.md | 30 +++----
...ntic-ownership/specs/linux-ublk-frontend/spec.md | 14 +--
...ec/changes/canonical-semantic-ownership/tasks.md | 14 +--
8 files changed, 153 insertions(+), 100 deletions(-)
```

The complete final `jj diff` was run. Captured byte count and digest:

```text
bytes: 79094
sha256: 49a27496d0f93329ac4bd9e117be7f98f61045a0a31d9d32b750bb7f3ad159d9
harness capture: artifact://244
```

The diff contains only the eight files above. No implementation, reviewed-state, evidence-mapping, current canonical spec, archive, or successor-change file is modified.

## K. Remaining blocker

No unresolved semantic ownership decision remains in this correction package.

Exact remaining blocker:

> Awaiting external semantic review approval; implementation is explicitly prohibited before approval.

Task `0.4` remains unchecked. The change must remain unapplied until renewed external approval.