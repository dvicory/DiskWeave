# Prompt: reconcile DiskWeave architecture v0.8, update goal v3, and create/execute autonomous implementation goal v4

You are continuing an existing DiskWeave implementation. The repository may contain completed, archived, corrective, in-progress, partially implemented, blocked, or superseded OpenSpecs and verification work. Do not assume the project is starting from scratch, and do not infer implementation state from architecture or goal documents alone.

## Inputs and authority

Read, in this order:

1. `diskweave-refined-architecture-v0.8.md` — the current, self-contained normative architecture.
2. The current autonomous implementation goal v3 and its status, conformance, and evidence artifacts.
3. The repository implementation, crate/module dependency graph, OpenSpecs, ADRs, archived changes, verification records, tests, commits, persistent fixtures, and actual `dwv demo` behavior.
4. `diskweave-refined-architecture-v0.7.md` and older goals only when needed to understand the contract or rationale under which existing work began.

Use this precedence when sources disagree:

```text
accepted v0.8 architecture invariants and decisions
    > newer accepted ADRs that explicitly amend v0.8
    > executable repository evidence
    > current/in-progress OpenSpecs and autonomous goal text
    > historical architecture/goal rationale
    > assumptions, comments, or names not backed by evidence
```

Executable evidence may reveal that an architectural assumption is wrong; it does not silently repeal an accepted invariant. Preserve the reproduction, record the conflict, and use the repository's ADR/architecture-amendment process.

## Mission

Create an evidence-based **DiskWeave autonomous implementation goal v4** that:

- reflects the repository's actual state rather than restating architecture prose;
- preserves and completes valid goal-v3 correctness, verification, deterministic-testing, and portable-product work;
- applies only those v0.8 clarifications that current code or evidence actually needs;
- keeps conventional independently readable data members and the portable block-parity product as the implementation focus;
- preserves clean responsibility boundaries among physical stores, topology, codecs, transaction protocol, executor ownership, simulator layers, formats, and namespace placement;
- keeps the existing `dwv demo` CLI as the continuous operator-facing acceptance spine;
- continues the next dependency-ready implementation work after goal v4 and its reconciliation artifacts are coherent.

Do not stop after producing a plan or conformance matrix.
Before changing implementation, verify the handoff baseline: no active OpenSpec change is left incomplete, main specs validate strictly, Goal v3 status/evidence artifacts exist, and the current focused and integrated portable checks pass. Resolve any handoff inconsistency before starting v0.8 remediation.

# 1. Inspect before deciding

Inspect the repository before claiming that a seam, type, dependency, test, format field, model, command, or defect is present or absent.

At minimum inspect:

- every unfinished, recently completed, or evidence-sensitive goal-v3 item;
- OS-007, OS-009, OS-017, and OS-024 and their actual acceptance evidence;
- VE-001 through VE-005 and the VP property/evidence index;
- `dwv demo` commands and the integrated portable workflow;
- `StoreId`, physical identity, logical slot/member identity, role/coding-position bindings, topology snapshots, and assignment generations;
- `RandomAccessStore`, backend capabilities, operation slots, executor ownership, buffer/token lifecycle, completion uncertainty, and shutdown/drain behavior;
- codec APIs, dependency direction, golden/reference vectors, and topology independence;
- `dwv-sim` low-level media/fault state and its parity/recovery/integrity fixtures;
- normalized trace, scenario, reproducer, and evidence types;
- parity-envelope, recovery-manifest, SQLite semantic-schema, and format-family identification;
- namespace rule matching, member selection, mover/tier logic, and protection-status reporting where implemented;
- crate/module dependency direction and any public semantic types that leak implementation libraries.

For every relevant item, classify its actual state as:

- complete and evidenced;
- behavior complete but evidence incomplete;
- partially implemented;
- in progress;
- dependency blocked;
- platform gated;
- not started;
- superseded by an accepted decision;
- non-conformant;
- unknown pending the smallest focused probe.

Do not trust a milestone label, archive state, commit message, or passing unit test by itself.

# 2. Preserve goal v3 rather than restarting it

Goal v3 is complete for its declared scope. Architecture v0.8 does not invalidate coherent work merely because it sharpens responsibility boundaries. Verify that completion status against repository evidence and carry forward only real unfinished work, evidence gaps, or gated claims.

Apply these rules:

- Preserve completed goal-v3 behavior and evidence when it satisfies v0.8.
- Carry coherent in-progress OpenSpecs forward under their existing identifiers and history.
- Verify every status claim in goal v3 against repository evidence before copying it into goal v4.
- Do not restart OS-007, OS-009, OS-017, OS-024, VE-001, VE-002, VE-003, VE-004, or VE-005 solely because v0.8 restates or clarifies their architectural seams.
- Do not replace `dwv-sim`, the explicit transaction oracle, a selected transaction implementation, `array.sqlite3`, normalized trace semantics, or the `dwv demo` command family merely to make an abstraction cleaner.
- Do not reorder dependency-ready portable work unless inspection finds a real correctness dependency, broken acceptance claim, or implementation blocker.
- Add a v0.8 acceptance clarification to an unfinished OpenSpec only when it belongs naturally to that OpenSpec and does not inflate its scope.
- Use a small successor/corrective OpenSpec for completed work only when current code violates an accepted v0.8 invariant or lacks evidence needed by dependent work.
- Preserve historical OpenSpec, ADR, commit, and verification artifacts; do not rewrite history as if earlier work never happened.
- Keep `docs/handoffs/goal-v3.md` and its evidence artifacts as historical records. Produce a status/carryover map instead of rewriting Goal v3 to make it appear newly compliant.
- An empty carryover or correction queue is a valid result when inspection finds no remaining work; do not invent implementation to populate it.
- When a concrete defect is found, retain a minimized deterministic reproducer before or alongside the fix whenever practicable.

# 3. Perform a bounded v0.8 conformance audit

This audit exists to verify the current product architecture, not to start a generalized storage framework. Build a repository-grounded matrix for the following properties.

For every Section 3 property, record the exact inspected source paths, public symbols or dependency edges, focused executable check or test, observed result, claim boundary, and action. A `CONFORMANT` result SHALL NOT rely only on architecture prose, archived OpenSpecs, commit messages, type names, or an existing passing unit test. If the code and evidence were not directly inspected, classify the row as `UNKNOWN`.

## 3.1 Physical store versus logical topology

Verify that:

- a physical `RandomAccessStore`/backend endpoint is not inherently a logical slot, conventional filesystem, data member, parity member, or coding position;
- topology/assignment code owns the binding from physical identity to logical slot, role, payload window, and coding position;
- capability and physical identity evidence are store properties, while role, protection, availability, and topology generation are protocol properties;
- file-backed and simulated stores can serve data or parity roles without role-specific backend semantics.

Do not demand renaming, wrappers, newtypes, or new crates when existing code already preserves the distinction clearly. Prefer a focused dependency/type/property test over churn.

## 3.2 Codec independence

Verify that codec primitives consume only coding facts such as:

- codec profile and semantic parameters;
- explicit shard/coding positions;
- input/output byte shards or ranges;
- tail and zero-extension rules.

They should not discover devices, inspect array topology, choose placement, open stores, persist assignments, or depend on frontend/recovery modules. If the boundary is already sound, add only the missing evidence. If it is not, make the smallest facade or dependency correction; do not redesign parity layout.

## 3.3 Executor and consequential-operation ownership

Verify that operation slots/executor code owns generic I/O lifecycle concerns:

- actual buffers and generational tokens;
- child submissions and completions;
- permits and bounded admission;
- cancellation requests, abandonment, drain, and reclamation;
- uncertain completion and exactly when a slot becomes reusable;
- terminal evidence or durable transfer of unresolved obligations to recovery state.

The executor must not own namespace placement, parity policy, recovery-schema interpretation, or a permanent store role. The transaction representation must not be generalized merely to make the executor reusable.

## 3.4 Simulator layering

Verify that the low-level deterministic media model can represent durable bytes, volatile acknowledged writes, pending operations, completion delivery, flush/fence behavior, torn/short/error writes, crashes, power loss, reset, and disappearance without depending on parity equations, dirty regions, checksum records, or SQLite types.

Parity, integrity, envelope, and recovery fixtures may depend on that lower model. If this dependency direction already holds, no module or crate split is required. If not, extract only the smallest seam that improves current deterministic tests.

## 3.5 Persistent and trace format identity

Verify that parity envelopes, recovery manifests, SQLite semantic exports, normalized traces, and reproducer bundles are unambiguously interpreted under the conventional-member block-parity semantics through their existing magic/profile/features/version or a minimal family discriminator.

Rules:

- Do not add a redundant field if existing bytes already discriminate the semantic family safely.
- If an experimental format is ambiguous, fix it in the owning format OpenSpec with bounded decoding, golden fixtures, migration behavior, and hostile-input tests.
- If compatibility has been promised, do not silently rewrite bytes; use an ADR and explicit dual-reader/migration plan.
- Do not reserve unused filesystem, allocator, object, extent, per-object protection, or coding-group fields.
- OS-024 remains a normalized DiskWeave block-parity semantic trace; producer-specific schedule/model/fuzz witnesses remain separate retained evidence.

## 3.6 Durability versus array protection

Verify that request-level ordering and durability semantics remain distinct from the array's redundancy profile and namespace placement.

Do not add `ProtectionIntent`, `ProtectionClass`, `Mirror2`, `EC4+2`, `RaidZ2`, or equivalent per-object concepts to current `BlockRequest`, transaction actions, operation slots, recovery state, trace events, or CLI merely for generality. The current product has an array-level protection profile.

## 3.7 Namespace placement scope

Where namespace placement exists, verify that it remains a deterministic whole-file logical-slot selection policy with explicit inputs, explanations, conflicts, and failure behavior.

It may separate rule matching from slot selection when that improves current correctness and tests. It must not become an extent allocator, object graph, coding-group planner, or general protection-policy engine.

## 3.8 Current protocol is allowed to remain specific

Verify that these layers remain intentionally specific to DiskWeave conventional-member block parity:

- normalized block/frontend request semantics;
- stable logical slots and positional parity mapping;
- parity-address range coordination;
- dirty-region and integrity-invalidation protocol;
- `array.sqlite3` recovery semantics;
- explicit and/or `procmachines` transaction implementations;
- degraded-read, verification, repair, and rebuild policy;
- current semantic trace event vocabulary.

Do not generalize these layers to support an unspecified second product.

## 3.9 No speculative production machinery

Search for unused abstractions whose only rationale is an unspecified filesystem/allocation product, including:

- placeholder filesystem or allocator crates;
- unused inode/object/extent IDs in current public APIs;
- allocation or arbitrary coding-group managers with no current consumer;
- per-object protection-policy hierarchies;
- universal transaction/effect frameworks;
- additional data-member metadata regions;
- current-format fields reserved for unspecified structures;
- CLI commands or feature flags with no accepted current behavior.

Do not create such machinery. Remove existing speculative code only when it is truly unused and safe to remove without disrupting valid work; otherwise contain it through the smallest corrective plan.

# 4. Conformance outcomes and actions

For each audit row assign one outcome:

| Outcome | Meaning | Required action |
|---|---|---|
| **CONFORMANT** | Existing implementation and evidence satisfy v0.8 | Link evidence; do not reimplement |
| **LOCAL CLARIFICATION** | Semantics are correct but naming/docs/tests permit confusion | Make the smallest compatible clarification or dependency test |
| **EVIDENCE GAP** | Behavior appears correct but a consequential claim lacks executable evidence | Add a focused test to the owning OpenSpec or a small verification successor |
| **NON-CONFORMANT** | Existing behavior violates an accepted v0.8 invariant | Preserve a reproducer and remediate before depending on it |
| **UNKNOWN** | Inspection cannot determine the property | Add the smallest probe/test needed to resolve it |
| **NOT YET APPLICABLE** | Relevant component does not yet exist | Record the boundary in its future owning OpenSpec; add no placeholder code |
| **PLATFORM-DEFERRED** | Evidence genuinely requires macOS bridge, Linux, or hardware | Preserve the claim boundary and platform gate |

Do not classify a portable correctness, recovery, integrity, ownership, or dependency-direction defect as platform deferred.
Goal completion requires every implemented portable row to be `CONFORMANT`, a completed `LOCAL CLARIFICATION`, or supported by completed corrective evidence. Unresolved `UNKNOWN`, `EVIDENCE GAP`, or `NON-CONFORMANT` rows block completion. `NOT YET APPLICABLE` and `PLATFORM-DEFERRED` are valid only for genuinely absent or gated subsystems and SHALL name their owner and entry condition.

# 5. Rewrite goal v3 into goal v4

Rewrite the current goal from actual repository state. Do not merely append a v0.8 section, and do not discard useful goal-v3 detail, status, evidence, or dependencies.

Goal v4 must contain these queues:

Queues A-E contain only actual remaining work. An empty queue is valid and SHALL NOT trigger invented implementation.

## A. Immediate correctness and conformance blockers

Only demonstrated defects, unsafe claims, or dependency violations that block safe dependent work. Include inherited goal-v3 blockers and any v0.8 issue proven by inspection.

## B. Coherent goal-v3 carryover

All unfinished or under-evidenced correctness, verification, OS-007/009/017/024, `dwv demo`, and portable workflow work that remains valid. Preserve identifiers, dependencies, acceptance evidence, and history.

Completed and evidenced milestones belong in the status/carryover map, not this implementation queue.

This queue remains the default execution priority unless Queue A contains a true blocker.

## C. Narrow v0.8 corrections or evidence

Only small work justified by the audit, such as:

- a store/slot/role semantic clarification or dependency guard;
- topology-owned role binding evidence;
- a topology-free codec facade/test;
- lower simulator dependency inversion;
- an experimental format/trace family discriminator;
- a deterministic whole-file placement seam or test.

Do not add an item merely because the architecture names the property. If the repository is conformant, link evidence and omit implementation work. Merge a correction into Queue B only when it is naturally coherent with an unfinished owner; otherwise use the smallest successor item.

## D. Additive verification evidence

Carry VE-001 through VE-005 according to their actual status. Preserve the VP property index, finite bounds, non-claims, producer-specific witnesses, and normalized regressions. Move a verification item earlier only when it resolves a concrete high-consequence uncertainty in current behavior.

Finite completed evidence is sufficient for its declared bounds. Do not open broader fuzzing, schedule exploration, or proof campaigns unless they resolve a concrete high-consequence uncertainty in current behavior.

## E. Capability-gated platform evidence

Keep macOS bridge, Linux ublk/io_uring/filesystem, and hardware work behind actual entry gates. Their absence constrains claims but does not block portable work.

For every queue item, state prerequisites, exact executable completion evidence, claim boundary, retained artifacts, and next work unlocked.

Before creating a recovery-manifest or rebaseline successor, compare the proposed gap with OS-015 acceptance and evidence. If only CLI composition or demo evidence is missing, use a narrow successor; if the recovery contract itself is incomplete, amend or succeed OS-015 explicitly.

# 6. Preserve `dwv demo` and the failure-artifact pipeline

Keep the existing `dwv demo` CLI as the continuous acceptance surface. Preserve valid command names and machine-readable outcome distinctions. Do not create a replacement CLI generation.

Before broad remediation, record a baseline of relevant working scenarios. After each change affecting portable behavior, run:

1. the smallest focused scenario proving the change;
2. neighboring regression scenarios;
3. the integrated portable workflow whenever prerequisites exist.

The integrated workflow remains:

```text
init
  -> healthy read/write/flush/reopen
  -> baseline and read-only scrub
  -> induced fault and evidence classification
  -> verified repair or conservative refusal
  -> degraded read and resumable rebuild
  -> recovery-state-loss handling
  -> trace export/render/replay
```

This is an evidence sequence, not a requirement that `demo run` alone implement every stage. For each stage, state the exact command or command sequence, outcome, and current evidence. Do not claim integrated recovery-state-loss handling unless it is actually exercised; if that stage is missing, classify it and queue the smallest coherent successor.

When a demo, deterministic test, property test, fuzzer, scheduler, bounded proof, or abstract model finds a defect:

```text
preserve the original producer witness
    -> reproduce deterministically
    -> minimize at the producing layer
    -> create a normalized DiskWeave scenario/trace when representable
    -> retain expected terminal payload/parity/integrity/recovery/ownership state
    -> add a permanent regression
    -> fix without weakening the invariant
    -> rerun the producing layer and cheaper downstream semantic layers
    -> rerun focused and integrated demo evidence
```

Do not discard producer-specific scheduler witnesses, fuzz inputs, Kani harness counterexamples, or model paths merely because a normalized trace exists.

# 7. Verification continuity

Preserve goal v3's property-first verification program:

- VE-001 remains narrowly bounded verification of small high-consequence pure or unsafe components, not application-wide proof.
- VE-002 maintains one independent abstract transaction/recovery model by default, with the accepted PlusCal/TLA+/TLC versus Stateright decision and documented bounds, fairness, mutations, counterexamples, and non-claims.
- VE-003 uses broad schedule exploration only where an existing concurrency seam makes it valuable; Loom remains for tiny custom synchronization models.
- VE-004 is a later test-only complement to `dwv-sim` for production-adjacent deterministic I/O, never a replacement or Linux/hardware proof.
- VE-005 expands structured property/fuzz generation and promotes minimized failures into the regression corpus.

Do not add a verifier-shaped runtime, effect system, actor model, synchronization API, transaction representation, or persistent schema merely to use a tool. Equivalent tools are acceptable when they prove the same property and retain the same assumptions, bounds, witnesses, and non-claims.

Completed finite verification remains complete for its declared bounds. Broader campaigns are additive unless they resolve a concrete high-consequence uncertainty.

# 8. Prohibited goal-v4 work

Goal v4 SHALL NOT:

- implement or prototype a DiskWeave-owned filesystem or allocator;
- design a new native on-disk filesystem format;
- add DiskWeave metadata to conventional data members;
- implement inode/extent trees, arbitrary coding groups, per-file erasure coding, snapshots, reflinks, or CoW-root publication;
- add unused per-object protection-policy APIs;
- replace current positional parity, dirty/integrity protocol, recovery-state semantics, or `dwv-sim` for abstract generality;
- turn current transaction actions into a universal storage/effect language;
- create placeholder crates, feature flags, CLI commands, tables, or format fields for unspecified products;
- replace ublk or the current block engine merely to pursue abstraction generality;
- delay portable correctness work for speculative optionality;
- claim that conventional-member recovery guarantees apply to another product family.

# 9. Autonomous decision policy

Make routine reversible choices without asking the user, including:

- module/crate placement and internal Rust organization;
- test and evidence organization;
- dependency-lint mechanism;
- bounded queue/worker/cache/fixture defaults;
- SQLite settings within accepted durability semantics;
- trace/parser limits and experimental schema migration mechanics;
- property/fuzz strategies and equivalent verification tools;
- small naming or facade corrections behind accepted seams.

Escalate only when evidence reveals:

- a product-level semantic or compatibility decision;
- a destructive or irreversible format migration;
- a materially different recoverability or availability promise;
- a safe correction that would substantially undermine the current product goal;
- genuinely inconclusive alternatives with different user-visible consequences;
- a proposal to open a separate product family with its own namespace, allocation, format, or recovery contract.

# 10. Goal-v4 deliverables

Produce and maintain, using repository conventions:

1. **Autonomous implementation goal v4**, rewritten from actual repository state.
2. **Goal-v3 status and carryover map**, showing completed, retained, minimally amended, superseded, blocked, platform-gated, and still-in-progress work.
3. **Bounded v0.8 conformance matrix**, linking each Section 3 property to code, tests, OpenSpecs, evidence, or a minimal corrective item.
4. **Minimal corrective/successor OpenSpecs or amendments** only where current implementation genuinely violates v0.8 or lacks evidence required by dependent work.
5. **Updated or explicitly reaffirmed `dwv demo` evidence log and permanent regression corpus**, adding new artifacts only when changed claims or new evidence require them.
6. **Updated verification status**, retaining VP mappings, bounds, assumptions, tool decisions, and producer witnesses.
7. **Dependency/status index** naming the exact next dependency-ready action.

After these artifacts are coherent, continue executing the next dependency-ready goal-v4 item. Do not stop at reconciliation.

# 11. Completion standard

Goal v4 is complete only when:

- actual goal-v3 work has been completed, retained, or carried forward with truthful status and executable dependencies;
- all applicable v0.8 current-product boundary rows are conformant or have completed corrective evidence;
- no portable correctness, integrity, recovery, lifecycle, or ownership defect is hidden as optionality or platform work;
- OS-007, OS-009, OS-017, OS-024, and VE work retain honest status and are not gratuitously restarted;
- `dwv demo` exercises real portable implementation paths and truthful clean/mismatch/refusal/blocked/unsupported/uncertain outcomes;
- for implemented seams, physical stores, logical slots, roles, coding positions, codecs, executor ownership, and simulator dependency direction are unambiguous; absent or gated seams have explicit `NOT YET APPLICABLE` or `PLATFORM-DEFERRED` owners and entry conditions;
- supported persistent and trace decoders cannot misinterpret incompatible semantic families; unknown or incompatible inputs are rejected, without implying that experimental formats are stable;
- no speculative allocator, per-object protection API, universal transaction framework, format, crate, or CLI has been introduced;
- macOS bridge, Linux, and hardware claims remain gated to their actual evidence;
- the exact next dependency-ready action is named and execution continues; if no dependency-ready portable item remains, an evidenced dependency or platform block is a valid endpoint and its entry condition is recorded.


## 12. Current execution status (2026-08-08)

### 12.1 Bounded v0.8 conformance matrix

| Section 3 property | Inspected implementation and evidence | Outcome | Action and claim boundary |
|---|---|---|---|
| 3.1 Physical store versus logical topology | `crates/dwv-store/src/lib.rs` (`RandomAccessStore`, `StoreCapabilities`); `crates/dwv-core/src/topology.rs` (`TopologySnapshot`, assignments); `crates/dwv-store-file/src/file_store.rs`; `crates/dwv-sim/src/media.rs` (`MediaConfig`/`MediaSimulator`) | CONFORMANT | Store identity/capabilities and logical role bindings remain separate. File and simulated stores do not encode data/parity roles. |
| 3.2 Codec independence | `crates/dwv-codec/src/lib.rs` (`ParityCodec`, `XorReference`); codec bounded and golden-vector tests | CONFORMANT | Codec consumes geometry, coding position, ranges, and bytes only. No store, topology, recovery, or frontend dependency is introduced. |
| 3.3 Executor ownership | `crates/dwv-store/src/lib.rs` (`OperationSlotTable`, `RandomAccessStore`); `crates/dwv-service/src/admission.rs`; operation-slot and service lifecycle tests | CONFORMANT | Slot generations, buffers, permits, completion evidence, abandonment, and drain remain executor/service concerns. |
| 3.4 Simulator layering | `crates/dwv-sim/src/media.rs` (`MediaSimulator`, `MediaSchedule`, `MediaTrace`); parent protocol adapter in `crates/dwv-sim/src/lib.rs` (`Simulator`); `media_model_replays_without_protocol_state` and existing schedule/replay tests | LOCAL CLARIFICATION completed | Low-level media state now has a role-neutral module with no parity, dirty-region, checksum, envelope, recovery-schema, or SQLite dependency. The parent fixture owns recovery/envelope state and adapts protocol schedules. Evidence is deterministic and portable; it does not claim a production executor or physical durability. |
| 3.5 Persistent and trace format identity | `crates/dwv-format/src/lib.rs`; `crates/dwv-trace/src/lib.rs`; `crates/dwv-recovery-sqlite/src/lib.rs`; hostile envelope and normalized-trace tests | CONFORMANT | Existing magic/profile/features/version and schema checks remain the family boundary. No reserved filesystem or allocator fields were added. |
| 3.6 Durability versus array protection | `crates/dwv-core/src/lib.rs` (`DurabilityIntent`); `crates/dwv-transaction-ref/src/action.rs`; topology/profile and service planning code | CONFORMANT | Request ordering/durability remains distinct from array-level parity. No per-object protection API was added. |
| 3.7 Namespace placement scope | No namespace placement or allocator implementation is present in the inspected portable crates; architecture Section 3.7 remains the future namespace owner | NOT YET APPLICABLE | No placeholder allocator or filesystem machinery is added. Entry condition: an accepted namespace/frontend OpenSpec with deterministic whole-file placement semantics. |
| 3.8 Current protocol specificity | `crates/dwv-service/src/service.rs`; `crates/dwv-recovery/src/*`; `crates/dwv-transaction-ref/src/*`; `src/demo.rs`; existing portable demo and recovery evidence | CONFORMANT | Block parity, recovery state, integrity invalidation, rebuild, verification, and CLI semantics remain DiskWeave-specific. |
| 3.9 No speculative production machinery | Scoped source audit across `crates/`, `src/`, and `docs`; no unused object/protection/allocator API was found | CONFORMANT | No new speculative crate, feature, CLI, schema, or universal transaction/effect framework was introduced. |

The audit inspected code and tests directly. `cargo check --workspace` passed
after the simulator seam change; `cargo test -p dwv-sim` passed 28 tests,
including the new role-neutral media test.

### 12.2 Goal-v3 status and carryover

- **Completed and retained:** OS-007, OS-009, OS-017, OS-024, VE-001 through
  VE-005 declared evidence, the bounded portable `dwv demo` path, and prior
  recovery/verification artifacts. Their historical documents remain unchanged.
- **Corrective work completed:** the only demonstrated v0.8 implementation gap
  found in this pass was simulator layering. `crates/dwv-sim/src/media.rs`
  now owns role-neutral media/fault state; `Simulator` remains the
  DiskWeave-specific protocol fixture.
- **Platform-gated:** live macOS bridge, Linux ublk/io_uring, and physical
  power-loss durability remain gated by their existing evidence entry
  conditions. The portable media model does not widen those claims.
- **Not started but valid future work:** namespace placement is absent and
  remains gated behind an accepted namespace/frontend consumer; no production
  placeholder is permitted.

### 12.3 Dependency/status index

The focused and integrated acceptance run is complete: workspace tests,
Clippy, formatting, metadata, strict OpenSpec validation, and the portable
`dwv demo` workflow all passed. No additional portable implementation was
dependency-ready in this goal; the remaining entry conditions are the existing
platform gates. Final artifact review is complete, and the implementation and
evidence work is committed as `c7f3d734`.