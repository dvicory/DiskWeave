# Milestone 8 canonical-semantic-ownership external review checkpoint

> **Purpose:** Review the complete `canonical-semantic-ownership` OpenSpec proposal before any delta is applied. The requested output path says “milestone-9”, but the material under review is Milestone 8 WP-8.1/WP-8.2.
>
> **Hard stop:** Do not apply the deltas, edit implementation or knowledge tooling, update reviewed fingerprints or evidence mappings, archive the change, create a successor change, or begin metadata/request/planning work. This checkpoint is semantic review only.

## 1. Repository state and review target

- Repository: `/Users/daniel.vicory/src/DiskWeave`
- Active milestone plan: `docs/milestones/m8.md`
- Active change: `openspec/changes/canonical-semantic-ownership/`
- Current canonical authority: `openspec/specs/*/spec.md`
- Coherence-only roadmap: `docs/handoffs/diskweave-refined-architecture-v0.8.md`
- Review package: `proposal.md`, `design.md`, `tasks.md`, and 12 capability deltas under the active change
- No proposal delta has been applied to `openspec/specs/`.
- No implementation, knowledge tooling, reviewed-state, manifest, curriculum, retained evidence, or archive has been changed by this proposal checkpoint.
- `metadata-certificate-authority-gate`, `normalized-service-request-boundary`, and `milestone-planning-cutover` have not been created.

Review the complete working-copy diff from the repository root:

```bash
jj diff
jj diff --summary
openspec status --change canonical-semantic-ownership --json
```

The expected diff summary contains only these added files:

```text
docs/milestones/m8.md
docs/verification/m8.md
openspec/changes/canonical-semantic-ownership/.openspec.yaml
openspec/changes/canonical-semantic-ownership/proposal.md
openspec/changes/canonical-semantic-ownership/design.md
openspec/changes/canonical-semantic-ownership/tasks.md
openspec/changes/canonical-semantic-ownership/specs/checksum-plane/spec.md
openspec/changes/canonical-semantic-ownership/specs/checksum-scrub-verified-repair/spec.md
openspec/changes/canonical-semantic-ownership/specs/degraded-read-offline-rebuild/spec.md
openspec/changes/canonical-semantic-ownership/specs/dirty-integrity-invalidation/spec.md
openspec/changes/canonical-semantic-ownership/specs/documentation-knowledge-architecture/spec.md
openspec/changes/canonical-semantic-ownership/specs/explicit-transaction-machine/spec.md
openspec/changes/canonical-semantic-ownership/specs/healthy-portable-io/spec.md
openspec/changes/canonical-semantic-ownership/specs/linux-ublk-frontend/spec.md
openspec/changes/canonical-semantic-ownership/specs/macos-bridge-feasibility/spec.md
openspec/changes/canonical-semantic-ownership/specs/normalized-block-semantics/spec.md
openspec/changes/canonical-semantic-ownership/specs/recovery-state-semantics/spec.md
openspec/changes/canonical-semantic-ownership/specs/volatile-media-simulator/spec.md
```

## 2. Required reviewer decision

Return one of:

1. **Approved for application**, explicitly approving owner selection, scenario dispositions, relationship kinds/edges, stable-ID decisions, recovery uncertainty semantics, and dependency-closure fingerprint semantics; or
2. **Rejected pending correction**, listing each exact requirement/scenario/edge and the required semantic correction.

Do not approve only the prose shape. Challenge whether each owner can support two independent conforming implementations and whether every failure/crash/uncertainty case remains fail-closed.

## 3. Audit scope and observed inventory

The proposal audited all current canonical material before drafting:

- 25 capability specifications
- 152 stable current requirement IDs
- 349 current scenarios
- 10 consequential clusters:
  1. protected mutation;
  2. watermark/fence/checkpoint;
  3. abandonment/failure/release;
  4. topology/request identity;
  5. verification/repair/rebuild;
  6. constitutional constraints;
  7. historical-authority leakage;
  8. Linux correction-program retirement;
  9. Linux acceptance scope;
  10. documentation relationships/fingerprints.

Classifications are intentionally non-exclusive:

| Classification | Count | Proposed disposition |
|---|---:|---|
| owner + valid refinement | 6 | Preserve owners; narrow recovery, transaction, checksum, dirty-clear, and repair consumers |
| owner + composition | 4 | Healthy admission, write, durable-completion, and failure requirements retain service-only composition |
| owner + adapter conformance | 3 | Linux mapping, volatile simulator, and macOS bridge remain adapter-local |
| constitutional constraint + detailed owner | 3 | Architecture, security, and evidence constraints remain high-level |
| duplicate ownership | 3 | Consolidate protected intent, complete fence predicates, and generic abandonment/failure wording |
| contradiction | 0 | The draft lost-acknowledgement contradiction was corrected before this checkpoint |
| missing canonical behavior | 1 | Add typed relationships and dependency-closure effective-fingerprint freshness |
| historical-authority leakage | 7 | Replace historical authority in checksum, healthy I/O, normalized requests, recovery, simulator, macOS bridge, and Linux wording |
| implementation nonconformance | 0 | No new implementation finding belongs to this proposal-only checkpoint |
| shadow architecture | 0 | No implementation or historical source supersedes current canonical semantics |
| confirmed no-problem | 17 | Thirteen capability audits need no delta; four cross-capability boundaries are explicitly retained |

Confirmed cross-capability non-problems:

1. `architecture-contract`, `security-boundaries`, and `evidence-boundaries` constrain detailed capabilities without owning their state transitions.
2. Parity verification, checksum-authorized repair, degraded rebuild, and metadata-loss recovery consume related evidence but make distinct decisions.
3. Shared tests and traces may verify multiple independent requirements without creating semantic co-ownership.
4. Parity-envelope session evidence and dirty recovery state are related layers; envelope evidence cannot authorize recovery state.

## 4. Final ownership reconciliation

| Semantic policy | Selected owner | Local refiners/composers/adapters | Scenario disposition | Classification |
|---|---|---|---|---|
| Protected home mutation requires durable dirty and integrity intent | `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation` | Recovery owns one-generation persistence/observation; transaction owns action emission; checksum owns `VALID`→`STALE`; healthy I/O composes; repair owns repair-local action | Keep owner success/failure; split recovery reject/mismatch from lost/corrupt/indeterminate; retain transaction/checksum/repair/RMW/full-overwrite local cases | duplicate ownership → owner + refinements/composition |
| Exact range outcome and persistence evidence | `req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence` | Volatile simulator specializes store behavior; healthy I/O maps exact writes | Preserve short/uncertain outcomes and service-local exact-write composition | owner + adapter/composition |
| Raw store watermark truth | `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` | Recovery decides authority; dirty protocol selects exact clear subset; transaction emits actions; healthy I/O composes | Remove repeated complete fence predicates; preserve local accepted/rejected cases | duplicate ownership → owner + refinements/composition |
| Typed authority for clean/valid/session claims | `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence` | Dirty exact-region clear, transaction action/release, healthy completion, and repair-local acceptance consume it | Preserve valid and invalid evidence cases with owner/local boundaries explicit | owner + refinements/composition |
| Exact dirty-region clear subset and generation checks | `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence` | Transaction maps accepted decision to release; healthy I/O composes | Preserve covering and omitted/stale evidence cases under dirty owner | owner + refinements/composition |
| Frontend abandonment/delivery-interest semantics | `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | Transaction owns state; dirty owns durable consequence; healthy I/O composes; Linux maps | Preserve abandonment before/after irreversible work and reconciliation cases | duplicate ownership → owner + composition |
| Backend operation/tag/buffer lifetime and generation reuse | `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | Transaction/healthy compose; Linux maps tags/resources | Preserve exhaustion, stale/duplicate completion, abandonment, and delayed release | owner + composition/adapter |
| Transaction action order, terminal state, and release | `req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions` plus its three local state requirements | Local transaction requirements refine imported durable-intent, checkpoint, lifetime, and dirty consequences | Rename generic scenarios to accepted/unavailable/authority/reconciliation outcomes | transaction-local owner + valid refinement |
| Durable dirty/restart consequence | `req.dirty-integrity-invalidation.failures-and-restart-are-conservative` | Transaction and healthy compose; Linux reacquires | Preserve failed home write; rename daemon crash to process loss; never infer clean from missing completion | owner + composition |
| Topology and request identity | `anchorless-topology-identity` owns slot/role/coding-position/assignment/generation/epoch; normalized request requirement owns request-carried fields | Healthy admission resolves against captured topology; Linux maps kernel request | Preserve healthy/stale assembly; add reordered collection and positional-mismatch cases | clarification + composition/adapter |
| Normalized request validation, ordering, durability intent | the two normalized request/ordering requirements | Healthy admission composes; Linux maps | Preserve supported/unsupported/range/mapping/replay cases; remove historical gate language | owner + composition/adapter |
| XOR update and known-erasure reconstruction | corresponding `xor-reference-model` requirements | Healthy write and degraded read consume exact math | Preserve RMW/full overwrite, single erasure, and beyond-tolerance cases | confirmed no-problem |
| Read-only mismatch classification and automatic repair authority | parity verifier owns read-only classification; checksum scrub owns unique independent-evidence authorization | Repair plan combines independent facts | Preserve unique parity/data target and ambiguous/beyond-tolerance cases | confirmed distinct owners |
| Separate-target repair durability and verified readback | `req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback` for repair-local behavior | Refines durable-intent and typed-fence owners | Preserve success and target-write/readback failure | owner + valid refinement |
| Degraded-read eligibility and offline rebuild/promotion | the two `degraded-read-offline-rebuild` requirements | Consume topology, recovery export/cursor, and XOR | Preserve known erasure, stale evidence, ambiguity, chunk, unknown durability, and alias refusal | confirmed distinct owner + prerequisites |
| Metadata-loss recovery and envelope evidence | `metadata-loss-recovery` owns decisions; `parity-envelope-profiles` owns non-authorizing envelope facts | Dedicated metadata authority change is deferred | Preserve all cases in this proposal | confirmed no-problem |
| Constitutional product/security/evidence boundaries | each of `architecture-contract`, `security-boundaries`, and `evidence-boundaries` owns its high-level constraint | Detailed capabilities remain constrained | No scenario movement | constitutional constraint + detailed owner |
| Canonical relationship graph and semantic freshness | new `req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived` | Change-impact/context requirements depend on it | Add valid/invalid graph, A→B→C propagation, unrelated-D stability, formatting stability | new durable behavior |
| Platform adapter conformance and evidence scope | portable semantics remain with normalized/store owners; each platform requirement owns translation/evidence | Simulator, macOS bridge, Linux adapter specialize or depend on owners | Preserve platform refusals/non-claims; separate Linux declared profile from exact environment | owner + adapter conformance |
| Linux correction evidence program | no product owner; program is complete | Evidence maps directly to durable requirements | Delete the two program-completion scenarios with the retired requirement | obsolete requirement retirement |

## 5. Exact relationship contract

Marker placement:

```text
<!-- dwv:req req.capability.local-policy -->
<!-- dwv:requires req.other.prerequisite -->
<!-- dwv:refines req.owner.complete-policy -->
```

- `requires`: the source depends on another independently owned semantic fact.
- `refines`: the target owns the underlying detailed operational policy; the source defines a narrower local specialization, composition mapping, or adapter conformance.
- One source-target pair may use only one kind.
- Only forward edges persist. Backlinks, aggregation, and owner-before-dependent order are derived.
- Unknown/non-current targets, self edges, duplicate edges, mixed-kind duplicates, misplaced markers, and cycles fail deterministically.
- Diagnostics must include source path, line, source ID, kind, target, and the smallest deterministic cycle when applicable.

Proposed graph facts:

- 48 total edges
- 23 `requires`
- 25 `refines`
- 44 nodes participating in the graph
- no unknown target
- no self edge
- no duplicate or mixed-kind source-target pair
- acyclic

Exact graph, grouped by source:

```text
req.checksum-plane.invalidation-precedes-protected-mutation
  refines -> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution
  requires -> req.checksum-plane.validity-is-generation-bound
  requires -> req.parity-verification-repair.mismatch-classification-requires-independent-evidence
req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback
  refines -> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
  refines -> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed
  requires -> req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
  requires -> req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout
  requires -> req.xor-reference-model.every-single-known-erasure-reconstructs-exact-bytes
req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target
  requires -> req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed
  requires -> req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority
req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence
  requires -> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
  refines -> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
req.dirty-integrity-invalidation.failures-and-restart-are-conservative
  requires -> req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
req.documentation-knowledge-architecture.agent-context-is-graph-selected-and-reproducible
  requires -> req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived
req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived
  requires -> req.documentation-knowledge-architecture.canonical-requirements-are-discovered-without-a-duplicate-semantic-registry
req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded
  requires -> req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived
req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence
  requires -> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
  requires -> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
  refines -> req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence
req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation
  refines -> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative
  requires -> req.dirty-integrity-invalidation.failures-and-restart-are-conservative
  requires -> req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
  requires -> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety
  refines -> req.dirty-integrity-invalidation.failures-and-restart-are-conservative
  refines -> req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative
  refines -> req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics
  refines -> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe
  refines -> req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
  refines -> req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
  refines -> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences
  refines -> req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence
  refines -> req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
  refines -> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity
  requires -> req.xor-reference-model.incremental-updates-and-full-recomputation-are-equivalent
  refines -> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
  refines -> req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions
  refines -> req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent
  refines -> req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic
  refines -> req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence
req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics
  requires -> req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations
  refines -> req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened
  refines -> req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics
req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate
  requires -> req.evidence-boundaries.evidence-scope-is-explicit
req.macos-bridge-feasibility.the-outcome-is-a-bounded-portable-demo-decision
  requires -> req.normalized-block-semantics.adapters-expose-bounded-deterministic-conformance-behavior
req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence
  requires -> req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence
req.recovery-state-semantics.evaluation-fixtures-cover-candidate-durability-and-reset-boundaries
  requires -> req.volatile-media-simulator.media-state-separates-durable-and-process-visible-effects
req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent
  refines -> req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation
req.volatile-media-simulator.operations-use-exact-normalized-ranges-and-structured-evidence
  refines -> req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence
```

Deliberately implicit:

- universal architecture/security/evidence constraints are not repeated on every detailed requirement;
- metadata-loss decision edges wait for the dedicated metadata authority change;
- reverse edges are derived;
- implementation call graphs, co-testing, lexical similarity, and optional future work are not semantic edges;
- offline rebuild relies transitively on topology/XOR through degraded eligibility;
- healthy failure relies on the dirty/restart owner rather than redundantly linking every recovery observation.

## 6. Requirement identity decisions

Proposed inventory: 26 modified IDs, one new ID, one retired ID; total current IDs remain 152 after application.

### New

- `req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived`
  - Reason: typed canonical relationships and dependency-closure effective-semantic freshness are new durable behavior.

### Retired

- `req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries`
  - Classification: requirement deletion because behavior is obsolete.
  - Reason: it describes a completed correction/evidence program, not durable product behavior.
  - Its two scenarios are deleted only with the requirement.
  - Existing proof/regression/evidence artifacts remain and must be mapped directly to durable requirements.

### Preserved with ownership relocation/narrowing

- dirty durable intent, checkpoint/clear, and failure/restart IDs;
- recovery home-mutation and typed-fence IDs;
- explicit transaction durable-intent, checkpoint, and failure IDs;
- checksum invalidation ID;
- healthy write, durable-completion, and abandonment IDs;
- separate-target repair ID.

### Preserved with clarification or relationship-only change

- healthy assembly/admission;
- Linux kernel request mapping and live acceptance;
- normalized portable-evidence boundary;
- simulator operation/safety requirements;
- recovery SQLite/evaluation requirements;
- macOS bridge outcome;
- checksum automatic repair;
- degraded-read eligibility and rebuild;
- documentation change-impact and context IDs.

No other current ID changes.

## 7. Scenario migration review

Every scenario in every changed requirement has an explicit disposition. Material name/content changes:

- Linux request mapping: preserve five scenarios; generalize unsupported intent without weakening; keep unrepresentable completion and trace replay.
- Linux live acceptance: replace “complete VM workflow passes” with “declared supported profile passes”; move exact guest/kernel/queue/trace/source facts to evidence; preserve unavailable-environment and stronger-claim refusal.
- Linux correction program: delete its two scenarios with the obsolete requirement; preserve their evidence under durable owners.
- New relationship requirement: add valid extraction, invalid graph, A→B→C propagation/unrelated-D stability, and formatting-only stability.
- Documentation change impact: replace generic relationship-change wording with dependency-closure semantic-prerequisite invalidation; preserve unavailable-baseline disclosure.
- Documentation context: preserve current-selection and context-bound failure while adding relationships/fingerprints/reading order.
- Recovery home mutation: preserve valid mutation; split generic commit failure into rejected/generation-mismatched-before-authoritative-commit and lost/corrupt/indeterminate observation.
- Recovery typed fence: rename both cases around typed recovery authority, preserving success and conservative volatile-evidence rejection.
- Recovery SQLite/evaluation: preserve cases; remove `OS-005` authority; retain evidence-driven selection and conservative reconciliation.
- Degraded read/rebuild and checksum repair: preserve their local scenarios unchanged except wording needed for exact owner relationships.
- macOS bridge: preserve candidate-pass/no-candidate behavior; remove `OS-021` and “live goal” authority.
- Simulator: preserve short write; rename uncertain completion to uncertain effect; preserve power-loss replay while removing historical ownership claims.
- Portable evidence: rename cases around portable-only evidence/later adapter; preserve non-certification.
- Checksum invalidation: preserve both cases; narrow the valid-extent case to checksum-local `VALID`→`STALE` through the durable-intent owner.
- Healthy assembly: preserve healthy/stale cases; add reordered collection/assignment resolution and positional mismatch refusal.
- Healthy write: rename RMW explicitly; preserve full overwrite.
- Healthy durable completion: rename cases around owner acceptance/rejection; preserve durable success/conservative failure.
- Healthy abandonment/restart: preserve both as service composition.
- Transaction intent/checkpoint/failure: rename generic cases to local accepted/unavailable, authority accepted/rejected, delivery-interest abandonment, and process loss.
- Dirty clear/restart: rename cases around selected-region authority and process loss; preserve exact subset/generation and durable dirty discovery.

Reviewer must reject any retained scenario that still independently restates a complete imported owner predicate, and any deleted scenario that loses an observable local behavior.

## 8. Recovery uncertainty contract requiring explicit approval

Final proposed distinction:

1. **Rejected or generation-mismatched before authoritative commit**
   - proposed transaction does not become authoritative;
   - prior snapshot remains authoritative;
   - caller receives no permission for protected home mutation.
2. **Lost, corrupt, or indeterminate commit observation where commitment may have occurred**
   - neither prior nor proposed resulting snapshot may be assumed authoritative for protected mutation;
   - no home-mutation permission exists;
   - reconciliation through the recovery commit-observation contract is required.

Exact proposed owner wording is in:

`openspec/changes/canonical-semantic-ownership/specs/recovery-state-semantics/spec.md`

This proposal deliberately does not redesign SQLite or solve the broader lost-acknowledgement adapter problem. The review question is whether the canonical fail-closed distinction is correct and sufficient without falsely making the prior snapshot authoritative after an indeterminate observation.

## 9. Dependency-closure fingerprint contract

- **Local semantic fingerprint:** normalized local requirement prose and scenarios, excluding intrinsic/relationship comments and formatting-only changes.
- **Effective semantic fingerprint:** after acyclicity validation, deterministic prerequisite-first digest of:
  1. local semantic fingerprint;
  2. sorted outgoing `(relation kind, target ID)` pairs;
  3. each target’s effective semantic fingerprint.
- Reviewed state stores and compares the effective fingerprint under a bumped schema.
- A local semantic change changes that requirement and every direct/transitive semantic dependent.
- An edge change changes its source and every dependent of that source.
- Formatting-only changes do not propagate.
- Review outcome/reason changes do not affect either fingerprint.
- Every stale requirement requires an individual review and concrete reason; bulk acceptance remains forbidden.
- `knowledge affected`, readiness, and `docs check` report `semantic-prerequisite-changed` for every direct/transitive stale dependent.

Required regression shape:

```text
A <- B <- C      D
```

B imports A; C imports B; D is unrelated. Changing A’s local semantics changes A, B, and C effective fingerprints, but not D. Formatting-only A changes preserve all four. Changing B’s edge set changes B and C, not unrelated A or D unless the new edge imports them.

Reviewer must explicitly approve transitive dependency-closure invalidation, not a direct-dependent-only substitute.

## 10. Linux correction evidence migration

When the retired Linux correction-program ID is removed, its current references must be removed or remapped. Existing TLA+, Kani, deterministic regression, fence-model, process-death, trace, and Linux evidence is retained and mapped directly, as applicable, to these exact durable targets:

1. `req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked`
2. `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`
3. `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`
4. `req.dirty-integrity-invalidation.failures-and-restart-are-conservative`
5. `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`
6. `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence`
7. `req.store-operation-contracts.store-failures-are-conservative-and-testable`
8. `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`
9. `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`
10. `req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing`
11. `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics`
12. `req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit`
13. `req.evidence-boundaries.evidence-scope-is-explicit`
14. `req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed`
15. `req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded`
16. `req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe`
17. `req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority`
18. `req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate`

These are migration targets, not a claim that every artifact proves every target. Application must inspect and map each artifact only to the exact target it actually verifies.

## 11. Linux acceptance scope clarification

The durable requirement keeps the bounded workflow and requires a declared supported Linux acceptance profile. The retained verification record—not the canonical requirement—owns exact ARM64 guest, kernel, endpoint, queue depth, transfer bounds, trace counts, and source digest.

The accepted claim remains functional file-backed Linux ublk/ext4 behavior only for the recorded supported profile/environment. The proposal still denies production concurrency, daemon recovery, raw-device durability, physical power-loss durability, FUA, broader filesystem support, deployment correctness, online topology mutation, and hardware safety.

Review must confirm this separates contract from evidence environment without broadening the demonstrated claim.

## 12. Historical-authority replacements in this change

| Current leakage | Proposed replacement |
|---|---|
| checksum `OS-010` intent | refinement to durable dirty/integrity owner plus checksum-local `VALID`→`STALE` prose |
| healthy purpose “required by the handoff” | direct portable owners and service-composition wording |
| healthy `OS-008/OS-010` ordering | typed dirty/recovery/transaction/XOR/store relationships plus direct orchestration prose |
| normalized `OS-001` and successor gate | durable portable-evidence-does-not-certify-platform behavior |
| recovery `OS-005` selection/fixtures | current SQLite-decision and evaluation-fixture requirements stated directly |
| simulator “handoff Phase 0”, `OS-002`, future OpenSpec, `OS-004` | standalone media model, store refinement, and explicit owner boundaries |
| macOS “live goal” and `OS-021` | evidence-backed macOS functional reference and current normalized frontend seam |
| Linux architecture-v0.8 correction mapping | retire completed program requirement and map evidence directly |

Documentation history-isolation scenarios may continue to mention old handoffs/goals as deliberately excluded test inputs. Metadata-loss historical references are intentionally deferred to `metadata-certificate-authority-gate` to avoid overlapping deltas.

## 13. Delta file inventory

OpenSpec reports all planning artifacts complete and ready for review. Deltas exist only for:

1. `checksum-plane`
2. `checksum-scrub-verified-repair`
3. `degraded-read-offline-rebuild`
4. `dirty-integrity-invalidation`
5. `documentation-knowledge-architecture`
6. `explicit-transaction-machine`
7. `healthy-portable-io`
8. `linux-ublk-frontend`
9. `macos-bridge-feasibility`
10. `normalized-block-semantics`
11. `recovery-state-semantics`
12. `volatile-media-simulator`

`proposal.md` lists exactly these 12 modified capabilities. No new capability is created.

## 14. Current validation evidence

Observed after the final proposal corrections:

```text
openspec validate --strict --all
26 passed, 0 failed
```

This includes `change/canonical-semantic-ownership` plus all 25 current specs.

```text
cargo xtask docs knowledge readiness
ready: true
requirements: 152
fingerprint-suspect: 0
historical-reference: 0
mapping-collision: 0
orphaned-state: 0
uncovered: 0
unknown-reference: 0
```

The readiness result describes the unchanged current canonical state. It does not validate the not-yet-implemented relationship model or proposed post-application reviewed state.

Independent proposal graph inspection observed:

```text
proposed requirement IDs after one add/one retirement: 152
edges: 48
requires: 23
refines: 25
unknown/self/duplicate/mixed issues: 0
acyclic: true
```

`openspec status --change canonical-semantic-ownership --json` reports proposal/specs/design/tasks all `done` and the planning artifact set complete. This means ready for review, not ready to archive.

No Rust tests, docs build, clean-room reconstruction, model, Kani, or live Linux workflow was rerun for this proposal-only checkpoint because no implementation/current semantic delta was applied. Those remain mandatory after approval and implementation.

## 15. Blocking decisions and expected response

No unresolved semantic decision is known inside the draft. The only active blocker is external semantic approval.

Review these questions explicitly:

1. Does every affected detailed operational policy have exactly one complete owner without hollowing out constitutional/security/evidence constraints?
2. Are all non-owner requirements true refinements, compositions, or adapter mappings rather than duplicate owners?
3. Is each `requires` versus `refines` choice correct under the exact definitions?
4. Is the 48-edge graph minimal but sufficient, including deliberately implicit transitive edges?
5. Does every scenario retain a distinct local behavior, and are deleted scenarios fully accounted for?
6. Are stable IDs preserved only where semantics remain materially the same?
7. Is the Linux correction-program ID correctly retired with adequate direct migration targets?
8. Does the Linux acceptance rewrite avoid both environment lock-in and claim broadening?
9. Is the recovery rejected/mismatched versus lost/corrupt/indeterminate distinction fail-closed and non-contradictory?
10. Is dependency-closure effective-fingerprint invalidation correct and implementable without a second semantic registry?

If approved, say so explicitly. Implementation may then begin with applying the canonical deltas and building the relationship/fingerprint tooling. If rejected, name exact file, requirement ID, scenario or edge, current proposed outcome, and required correction. Do not make implementation edits during review.
