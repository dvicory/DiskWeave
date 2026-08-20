## Context

Current `openspec/specs/*/spec.md` requirements own product behavior. The parameterized `RecoveryProtocol` module owns only the exact relation delegated by `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent`. Active changes are target behavior until synced. This migration therefore changes names and maintained projections without changing semantic ownership.

The design uses three source-derived facts:

1. the recovery protocol keeps write-recovery state, data/parity-write state, recovery state, lifecycle state, operation outcomes, attempted/known-applied regions, persistence-evidence coverage, and explicit release as separate dimensions;
2. the current specs distinguish request durability intent, low-level fence mechanisms, rebuild/SQLite checkpoints, and genuine terminal child results from the overloaded recovery/write uses being renamed here;
3. the knowledge system fingerprints current requirements and stores reviewed outcomes/reasons by semantic ID, so ID changes need a fail-closed rekey rather than a fake semantic re-review.

## Goals and non-goals

The goal is a current repository whose product vocabulary can be understood without a private DiskWeave decoder ring while preserving every correctness-relevant distinction.

Product/runtime semantics remain unchanged. The only intentional new normative behavior is a documentation-maintenance rule for Concepts and Terminology: it is maintained incrementally with provenance instead of being regenerated wholesale. That rule is reviewed as new documentation behavior rather than disguised as a semantics-preserving rename.

The migration must preserve:

- write completion versus proven durability;
- delegated-model write durability versus concrete persistence evidence;
- recovery `DIRTY`, `CLEAN`, and `INDETERMINATE`;
- unknown write state versus indeterminate write-effect observation;
- normal processing versus interrupted processing;
- reconciliation as a separate operation/condition;
- completed/aborted write state versus released ownership;
- request delivery interest versus continuing correctness work;
- exact evidence scope and generation/incarnation/order/watermark boundaries.

The migration does not generalize this protocol for a hypothetical future filesystem. Names may be specific to the current block-parity write protocol.

## 1. Product vocabulary

### Write-recovery record

Before DiskWeave writes data or parity, durable recovery state must already cover the affected dirty regions and integrity invalidation. If current durable `DIRTY`/`STALE` state already provides that coverage under the captured topology and generation, an equivalent record need not be committed again. Otherwise DiskWeave commits the required **write-recovery record** first.

A write-recovery record is a logical durable recovery fact. It is not necessarily one new physical database row per request. It does not mean:

- the data/parity write occurred;
- the data/parity write is durable;
- recovery state is `CLEAN`;
- the range is released;
- the data is parity-protected in some broader product sense.

### Data/parity writes and persistence

Use **data/parity write** for the protected block writes represented by this protocol. Use **data/parity change** only when a requirement genuinely covers a broader operation category.

`DataParityWritesDurable` is a bounded delegated-model fact: all represented regions are attempted and known applied and the relation has accepted its abstract durable-write outcome. It does not carry the full concrete store evidence needed for a product claim.

**Persistence evidence** is the owner-scoped evidence used by current requirements to support later claims. Its admissibility remains defined by the owning specs and can include store/incarnation, ordering domain, accepted and synchronized-through watermarks, topology, range/region, capability, and generation.

Neither fact alone releases the range. Recovery state becomes `CLEAN` only when the owning current evidence predicate permits **commit recovery state `CLEAN`**.

### Write lifecycle and reconciliation

The write lifecycle is:

- `Unowned`
- `NormalProcessing`
- `InterruptedProcessing`
- `CompletedAwaitingRelease`
- `AbortedAwaitingRelease`

`InterruptedProcessing` means the ordinary processing path was disrupted, but DiskWeave still owns the correctness work. It may remain true after reconciliation completes while persistence, recovery `CLEAN`, completion, or release work remains.

**Reconciliation** examines the allowed evidence for an uncertain operation and determines what DiskWeave may safely claim next. It is not retry, rollback, repair, or the entire interrupted lifecycle. In prose prefer “requires reconciliation” over noun-heavy forms such as “reconciliation-required condition.”

`WriteRecoveryRecordUnknown` and `DataParityWritesUnknown` are state values: the relation cannot safely classify the corresponding current fact as a known durable record or as no-write/awaiting-durability/durable data-parity state. Observation values are separate. `NoDataParityWriteObservation` is the baseline value when no data/parity effect observation has been accepted. `DataParityWriteEffectIndeterminate` records that a data/parity effect may have occurred but cannot be classified more definitely. `dataParityWriteObservationFinalized` separately records whether that observation classification is final, so an indeterminate observation can exist before finalization.

### Secondary terms

Keep these terms, but qualify them where needed:

| Term | Current use |
|---|---|
| authority | distinguish semantic authority from runtime/recovery/operator authority |
| claim | distinguish a statement/evidence claim from a writable ownership claim |
| ownership | distinguish semantic, range/write, resource, endpoint/adapter, and retention ownership |
| disposition | keep where it is a precise technical classification; prefer `result`, `classification`, or `required action` in explanatory prose when one is more specific |

Keep `currentization`, checksum baseline, lineage, store incarnation, topology epoch, ordering domain, watermark, and similar independent concepts unless a later change has a clearly better semantics-preserving name.

### Reader tests

A normal write should be explainable as:

1. DiskWeave owns the range and establishes required write-recovery coverage.
2. It performs the required data/parity writes.
3. The delegated relation can establish its bounded write-durability fact.
4. Current owners establish the required persistence evidence.
5. DiskWeave commits the affected recovery state `CLEAN`.
6. The write is completed but its range is still owned.
7. `releaseRange` ends ownership and permits reuse.

An interrupted write should be explainable as:

1. ordinary processing stops or an operation outcome becomes unknown;
2. the write enters `InterruptedProcessing` and remains owned;
3. if an outcome cannot yet be classified, it requires reconciliation;
4. reconciliation examines only allowed evidence and may produce a definite or indeterminate result;
5. interrupted processing can continue after reconciliation while other correctness work remains;
6. the write can complete or abort only under its existing predicates;
7. reuse remains forbidden until explicit release.

## 2. Delegated model rename map

This map is mechanical. It changes labels only. State cardinality, guards, transitions, invariants, finite parameters, release behavior, and delegated scope must remain unchanged.

### State and observation types

| Current | Target |
|---|---|
| `IntentState` | `WriteRecoveryRecordState` |
| `NoIntent` | `NoWriteRecoveryRecord` |
| `IntentPending` | `WriteRecoveryRecordPending` |
| `IntentDurable` | `WriteRecoveryRecordDurable` |
| `IntentUnknown` | `WriteRecoveryRecordUnknown` |
| `IntentObservation` | `WriteRecoveryRecordCommitObservation` |
| `IntentCommitUnknown` | `CommitUnknown` |
| `IntentCommitRejected` | `CommitRejected` |
| `IntentCommitDurable` | `CommitDurable` |
| `HomeState` | `DataParityWriteState` |
| `HomeUnmodified` | `NoDataParityWrites` |
| `HomeVolatile` | `DataParityWritesAwaitingDurability` |
| `HomeDurable` | `DataParityWritesDurable` |
| `HomeUnknown` | `DataParityWritesUnknown` |
| `HomeObservation` | `DataParityWriteObservation` |
| `HomeEffectUnknown` | `NoDataParityWriteObservation` |
| `HomeEffectIndeterminate` | `DataParityWriteEffectIndeterminate` |
| `HomeEffectDurable` | `DataParityWriteEffectDurable` |
| `ObligationState` | `WriteLifecycleState` |
| `InFlight` | `NormalProcessing` |
| `Handoff` | `InterruptedProcessing` |
| `Terminal` | `CompletedAwaitingRelease` |
| `Aborted` | `AbortedAwaitingRelease` |

`RecoveryState` and `RecoveryClean`/`RecoveryDirty`/`RecoveryIndeterminate` keep their semantic values; the state field becomes `recoveryState`.

### Fields and actions

| Current | Target |
|---|---|
| `intent` | `writeRecoveryRecord` |
| `intentObservation` | `writeRecoveryRecordCommitObservation` |
| `home` | `dataParityWriteState` |
| `homeObservation` | `dataParityWriteObservation` |
| `homeReconciled` | `dataParityWriteObservationFinalized` |
| `recovery` | `recoveryState` |
| `obligation` | `writeLifecycle` |
| `invalidated` | `integrityClaimsInvalidated` |
| `attempted` | `writeAttemptedRegions` |
| `mutated` | `writeKnownAppliedRegions` |
| `fenced` | `storesWithPersistenceEvidence` |
| `checkpointed` | `recoveryCleanRegions` |
| `rangeHeld` | `rangeOwned` |
| `terminalPendingRelease` | `releasePending` |
| `begin` | `startWrite` |
| `acceptIntent` | `confirmWriteRecoveryRecordDurable` |
| `loseIntent` | `observeWriteRecoveryRecordCommitUnknown` |
| `reconcileIntent` | `reconcileWriteRecoveryRecordCommitObservation` |
| `mutate` | `attemptDataParityWrite` |
| `reconcileMutation` | `reconcileDataParityWriteCoverage` |
| `makeHomeDurable` | `confirmDataParityWritesDurable` |
| `fence` | `observePersistenceEvidence` |
| `checkpoint` | `commitRecoveryClean` |
| `release` | `releaseRange` |
| `loseHome` | `observeDataParityWriteEffectIndeterminate` |
| `reconcileHomeIndeterminate` | `reconcileDataParityWriteEffectIndeterminate` |
| `reconcileHomeDurable` | `reconcileDataParityWriteEffectDurable` |
| `abandon` | `enterInterruptedProcessing` |

Keep `init` and `step`. Rename helper predicates/invariants mechanically so they use the target concepts, for example `TerminalRequiresRelease` → `CompletedOrAbortedRequiresRelease`, `DurableHomeRequiresCoverage` → `DurableDataParityWriteRequiresCoverage`, `MutationRequiresIntent` → `DataParityWriteRequiresWriteRecoveryRecord`, and `FenceAndCheckpointCoverage` → `PersistenceEvidenceAndRecoveryCleanCoverage`.

The model action names describe abstract state transitions, not concrete I/O commands.

### Rust/API label completion

The same vocabulary applies to existing Rust/API labels when the mapping is one-to-one and behavior-preserving:

- `WriteRecoveryRecordRequirement::FirstWrite` → `WriteRecoveryRecordRequirement::CommitRequired`;
- `WriteRecoveryRecordRequirement::AlreadyDirty` → `WriteRecoveryRecordRequirement::AlreadyCovered`;
- `ActionResult::WriteRecoveryRecordCommitted(WriteRecoveryRecordEvidence)` → `ActionResult::WriteRecoveryRecordDurableWithEvidence(WriteRecoveryRecordEvidence)`;
- a recovery-specific internal `checkpoint` module/file that owns the recovery-`CLEAN` operation → `recovery_clean`.

These are label-only changes. They SHALL NOT change enum cardinality, payloads, guards, result-kind classification, ordering, persistence predicates, or release behavior. Maintenance, rebuild-progress, SQLite, and other genuine checkpoint concepts retain `checkpoint`.

## 3. Identity policy and migration maps

A current identity is either descriptive or unnecessary.

- If stable cross-artifact identity is needed, use an existing descriptive semantic ID or create a concise one.
- If stable identity is not needed, use a descriptive name without another ID.
- Keep generated Bead IDs and real version/schema identifiers.
- Retired opaque codes appear only in migration history or historical provenance.

The exact requirement, verification, and evidence mappings live in `migration/semantic-id-renames.toml`. Historical planning-code-to-name mappings live in `migration/planning-names.toml`. Both are migration data, not permanent semantic registries.

This change is an explicit exception to the repository rule that unchanged semantics normally keep their `req.*` IDs. Gate #1 may authorize only the requirement rekeys listed in the approved migration map because retaining those IDs would preserve terminology this change retires. Review outcomes/reasons and graph identity are preserved as described below; ordinary work returns to stable-ID preservation after this migration.

### Current planning records

Preserve meaningful categories as structure, not opaque prefixes:

- `OS-*` → **Roadmap item**
- `C*` → **Capability**
- `U*` → **Semantic boundary**
- `R*` → **Semantic review result**
- `A*` → **Architecture invariant**, using the existing `arch.diskweave.*` identity
- `D-*` → **Accepted decision**
- `P-*` → **Provisional implementation choice**
- `V-*` → **Validation question**
- `F-*` → **Permanent-format candidate**

Within a typed collection, descriptive names must be unique. Cross-category references use the category structurally or an existing durable semantic identity such as a Bead, `req.*`, `arch.*`, verification, or evidence ID.

Do not invent `roadmap.work.*`, `outcome.*`, `capability.*`, or `reconciliation.*` replacement taxonomies merely to preserve the old codes.

### Verification claim names

Use descriptive names. In particular:

- `VP-005` → `uncertainty-remains-explicit`
- `VP-016` → `no-speculative-production-abstractions`

The full positive criteria remain in their owning verification definitions.

## 4. Canonical OpenSpec migration

The delta specs in this change are the complete canonical requirement/relationship surface selected by the current audit. They cover recovery/write vocabulary, claim-level persistence evidence, affected documentation terms, and broader `protected mutation`/`payload mutation` wording where the current meaning is a concrete data/parity or protected-state change. Capability directory slugs remain unchanged.

Requirement-body changes belong in OpenSpec. Requirement title/ID changes use the normal OpenSpec requirement-rename mechanism plus the ID map in `migration/semantic-id-renames.toml`.

OpenSpec 1.8 does not provide a lossless scenario-heading rename. Therefore scenario bodies remain in the delta under their current headings, while `migration/scenario-renames.toml` defines the approved final headings. Exact explanatory text outside requirement blocks that OpenSpec cannot express is listed in `migration/canonical-text-renames.toml`.

A scenario heading in a proposed delta that contains retired terminology must either appear in that migration map or intentionally retain a different precise technical meaning.

Apply ISO 24495-1 and Zinsser-style plain technical prose to the changed requirements and scenarios: prefer concrete nouns and verbs, avoid noun piles, and preserve all correctness conditions.

## 5. DiskWeave Guide and Concepts

Rename the documentation projection from **Human Guide** / `human_guide` to **DiskWeave Guide** / `diskweave_guide`. Do not move the Guide source directory as part of this migration.

Concepts and Terminology is persistent checked-in explanatory Markdown under the existing Guide source structure. It is maintained incrementally:

- each concept points to its current semantic owners;
- owner changes trigger targeted review of affected entries;
- unaffected entries are not wholesale regenerated or rewritten;
- the Guide explains canonical semantics but does not become semantic authority.

The documentation-knowledge delta in this change owns this maintenance contract.

After Gate #1, root repository guidance may add one concise rule:

> Use descriptive names for current concepts. When a stable identifier is needed, use a descriptive semantic ID instead of an opaque code. Keep generated Bead IDs and version identifiers as-is. Use legacy IDs only when referring to historical material.

Do not edit root `AGENTS.md` during Gate #1 design.

## 6. One-time migration mechanics

One-time mechanics under `migration/` were used to materialize the Gate #2 candidate and are archived with this change as migration evidence. They are not permanent product/tooling architecture and are not rerun after the final candidate already contains their approved result.

`migration/migrate.py` was limited to four responsibilities while constructing the candidate:

1. rename only the scenario headings listed in `scenario-renames.toml`, after verifying the post-delta requirement ID, occurrence, old heading, and approved body digest;
2. apply only the exact canonical explanatory-text replacements listed in `canonical-text-renames.toml` for text outside OpenSpec requirement blocks;
3. rekey reviewed outcomes/reasons for renamed `req.*` IDs without inventing review decisions;
4. fail if any old/current mapping is missing, duplicated, ambiguous, or already partially applied.

The final Gate #2 candidate carries the approved scenario headings directly in both canonical specs and the active delta. The migration files remain provenance for how that target was constructed; do not run the one-time script again during final cutover. Permanent `xtask`/knowledge tooling remains the source of truth for current fingerprints, graph integrity, readiness, and orphan detection.

The script must not rewrite scenario bodies, infer semantic equivalence, perform free-form prose rewriting, or create permissive aliases.

## 7. Active changes and current guidance

Before any still-active OpenSpec change resumes, normalize its complete proposal, design, tasks, delta specs, references, and current product vocabulary to the approved names. Rediscover the active set at execution time.

Current active architecture/campaign guidance is also a teaching surface: migrate current vocabulary and remove opaque current codes while preserving its categories and meaning. Remove or replace current configuration/rules that require retired `OS-*` identities. Historical/superseded architecture and archived changes may retain historical names.

Current actionable Beads are also teaching surfaces. Preserve generated `dwv-*` identities, dependency edges, status, scope, and acceptance meaning, but update the titles, descriptions, acceptance criteria, and agent context of open/current Beads that will remain actionable after cutover when they use retired vocabulary or opaque planning codes. Do not mass-rewrite closed Beads merely to make search clean.

Because Beads are shared workflow state rather than part of the isolated candidate tree, prepare the exact proposed Bead edits as Gate #2 evidence without applying them. Apply those reviewed edits only during the post-Gate-2 cutover.

Do not mass-rewrite archives merely to make grep clean.

## 8. Review-state and compatibility

The migration must preserve semantic review rather than manufacture it.

Before rekeying reviewed state:

- materialize old and new requirement graphs;
- canonicalize IDs through the approved requirement map;
- require a bijection over renamed/current identities;
- require the same owner capability, relationship kinds/multiplicity, and scenario/body semantics after approved terminology edits;
- compare the delegated model structurally after applying the label map;
- fail closed on any semantic mismatch.

Preserve each renamed requirement's review outcome and reason. Recompute local/effective fingerprints using existing tooling after the new IDs and relationships are current.
The new `req.documentation-knowledge-architecture.concepts-and-terminology-is-maintained-incrementally` requirement receives its own review; it is not covered by the rename equivalence witness.

If a renamed term appears in a serialized/persisted/external schema, use an explicit versioned migration or actionable refusal. Do not silently reinterpret old persisted values or keep permissive aliases solely for convenience.

## 9. Gates and cutover

### Gate #1

Gate #1 approves:

- product vocabulary and non-implications;
- delegated model label map;
- identity and planning-category policy;
- requirement/verification/evidence migration maps;
- complete canonical delta set;
- scenario-heading map;
- Guide/Concepts maintenance contract;
- active-change/current-guidance policy;
- one-time migration and compatibility strategy.

No current authority changes before Gate #1.

### Candidate target and Gate #2

After Gate #1, build the complete final target in a non-authoritative candidate/scratch tree:

- apply all approved OpenSpec bodies, titles, IDs, and relationships;
- apply the delegated model/API/trace/documentation/planning renames required by downstream work;
- normalize every active change;
- materialize the approved final scenario headings, canonical explanatory text, and reviewed-state rekeys represented by the one-time migration evidence;
- run OpenSpec, knowledge/readiness, model, Rust/trace, docs, and compatibility checks.

Gate #2 reviews that exact target and its equivalence evidence.

### Final cutover

Only after Gate #2, perform the real authority transition from the exact reviewed candidate:

1. land or apply the exact Gate #2-reviewed candidate revision, including its canonical specs, model/API/trace/docs/planning target, final scenario headings, and reviewed-state migration;
2. apply the separately reviewed shared-Bead edits;
3. run the complete validation/equivalence suite against that exact state;
4. verify that the canonical specs already contain the approved delta result, then archive this change without another spec sync.

Do not reconstruct the reviewed candidate by rerunning OpenSpec sync or the disposable migration during cutover. A failed final check blocks the cutover; do not land or archive a half-migrated state.

After cutover, current maintained surfaces use only current names. Old names remain only in migration history and historical provenance.

## 10. Risks and exclusions

The primary risk is semantic compression hidden inside a rename. The model-equivalence, graph-equivalence, scenario-body, evidence-scope, and explicit-gate checks exist to prevent that.

The change intentionally leaves unrelated narrow uses alone: request durability/ordering intent, low-level fences, SQLite/rebuild checkpoints, true terminal child/results, payload as a byte/boundary noun, generation, baseline, lineage, currentization, and historical terminology.

Capability and Guide source-directory renames are deferred. A future change may address them only if their current names become materially misleading.
