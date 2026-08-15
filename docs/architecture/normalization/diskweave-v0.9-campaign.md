# DiskWeave v0.9 architecture-normalization campaign

**Status:** Temporary, non-authoritative execution state.  
**Authority:** This file does not define product behavior, architecture, OpenSpec ownership, implementation, evidence, milestones, or release scope. Current `openspec/specs/*/spec.md` files remain the only detailed product authority.  
**Campaign source:** Repaired from the F2 report dated 2026-08-12.  
**Retirement rule:** Delete this file when the completion gate in Section 8 passes. Do not preserve it as a parallel requirements system.

## 1. Reproducible posture

| Item | Observed value |
|---|---|
| Repository working copy | Current jj working copy; revision identity is session-local and not semantic authority |
| Active architecture | `arch.diskweave.v0.8`, `docs/architecture/diskweave-architecture-roadmap-v0.8.md` |
| Active source SHA-256 | `9bb42d1b857491d5fe4d1425e07d563312adfe112cb7b1368c38508c12cf3522` |
| Settled target input | `arch.diskweave.v0.9`, content-final non-authoritative candidate |
| Target source SHA-256 | `18051e58d9386d25015ef574f178953735b389d45f17c7e6fe681ca5ebe1618a` |
| Imported F2 SHA-256 | `092875c01e73d7908be8e7f3f8da08d690349a81b0f7c4eac6268182d53cd559` |
| Current canonical requirements | 168 |
| Readiness after C8b archive | Green; no diagnostics |
| Candidate impact | 21 marked v0.9 invariants; 53 directly affected current requirements |
| Durable transition posture | C0a, C8a, and C8b canonicalized and evidenced; U11 blocked planning; U26b remainder, U27, and U28 remain separate; renewable Linux trace retention planned |
| Source-exclusion proof | Premature and not attempted |

Authority posture:

- v0.8 remains the active architecture direction. Its unmarked prose is architecture input, not detailed current product authority.
- v0.9 is the intended cumulative successor after reconciliation and activation. Later accepted meaning supersedes v0.8 where the change is explicit and genuine.
- An omission from v0.9 is not proof that v0.8 meaning was removed.
- F2 is prior analysis only. This campaign imports its useful map but does not inherit its terminal conclusions without the current method's required reconciliation.
- Implementation and evidence may confirm, falsify, or bound a claim. They do not fill a missing semantic owner.

## 2. Material repairs to F2

1. **Demoted unsupported terminal conclusions.** F2 used `current-owned` for many marked invariants after deterministic context retrieval. Retrieval established review surfaces, not semantic closure. Those rows are now `impact-mapped` unless a bounded reconciliation below establishes a narrower conclusion.
2. **Withdrew the global “no current debt” conclusion.** The supported statement is: no current debt was positively established among the reconciled scopes. Unreconciled scopes remain open.
3. **Added shared coded-range coordination.** v0.8 §11.6 and v0.9 §8.6 both require operations on corresponding offsets to coordinate by the shared coded parity range, including operations aimed at different data members. F2 omitted this independently advancing obligation.
4. **Added checkpoint admission closure.** v0.8 §11.7 and v0.9 §9.11 require a checkpoint to capture a closed mutation set so a later write cannot be marked clean. F2 treated nearby generation and fence rules too broadly.
5. **Recovered a stranded v0.8 lifecycle rule.** v0.8 §19.4 requires stable identities, resumable/cancellable semantics, and persisted correctness cursors for long-running jobs. v0.9 retains operation-specific cursors but does not repeat the complete cross-operation rule. The rule remains visible as unresolved retained intent rather than being silently removed.
6. **Split semantics that can advance independently.** Foreground currentization, indeterminate reconciliation, destructive present-data rebaseline, claim-lifetime prohibition, positive retention policy, deployment ordering, optional encryption, namespace placement, mover, staging, independent tools, and stable-format graduation now have separate rows.
7. **Narrowed the next capability.** F2's C2 restart capability is valuable but not yet semantically closed: it depends on unresolved mutation coordination, checkpoint closure, immediate-write, claim-lifetime, and authorization questions. The next slice is now read-only post-gap authority assessment, which is independently useful and does not authorize mutation.

## 3. Bounded semantic-reconciliation results

These are compact results, not persisted claim cards.

### R1 — Post-gap observational authority

**Current closure inspected:**

- `req.operator-recovery.production-assessment-is-observational-and-multidimensional`
- `req.operator-recovery.declarative-array-policy-locates-but-does-not-authorize`
- `req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred`
- `req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative`
- their deterministic prerequisite and direct-dependent closure

**Result:** Current canonical semantics have one coherent observational composition: policy locates, identity/recovery/evidence owners decide, and assessment reports separate dimensions without mutation or authorization. They do not define custody continuity, protection epochs, or `Current`/`Prior`/`Unprotected`/`Indeterminate` protection basis. The v0.9 post-gap classification is therefore a **target-delta**, not a current contradiction and not current debt established by this review. The existing observational owner is the candidate evolution surface; no new owner shape is preselected.

### R2 — Shared coded-range coordination

**Current closure inspected:**

- `req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity`
- `req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed`
- `req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation`
- the returned write, transaction, recovery, store, and XOR closure

**Result:** Current canonical semantics require a range-acquisition action and correct parity orchestration, but they do not define the conflict domain of that authority. They do not state that writes to different data members at corresponding offsets conflict through the same parity address. v0.8 §11.6 and v0.9 §8.6 agree on the missing rule. This is an **unresolved retained target semantic** and a possible current semantic gap, but current debt is not positively established: current requirements do not promise concurrent write progress, and the current service serializes through one mutable service path while supplying a synthetic range token. Implementation shape is not durable authority.

Two conforming implementations could otherwise choose per-member versus coded-range coordination and behave differently under concurrent same-codeword writes. Terminal closure therefore requires canonical semantics before implementation concurrency may rely on the rule.

### R3 — Checkpoint concurrency

**Current closure inspected:**

- `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`
- `req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences`
- `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`
- their transaction, watermark, checksum, and repair closure

**Result:** Current owners define exact typed fence evidence, region subsets, generations, action order, and stale-transaction rejection. They do not independently close the race in which an already-dirty overlapping mutation can be admitted while a checkpoint clears the same region; closure depends on the unresolved range/admission conflict domain. v0.8 §11.7 and v0.9 §9.11 retain the requirement that checkpoint captures a closed mutation set. Keep this as a separate **ownership-or-scope-unresolved** row rather than declaring it entailed by generic generations.

### R4 — Long-running job lifecycle

**Current closure inspected:**

- `req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority`
- `req.degraded-read-offline-rebuild.rebuild-resumes-and-completes-only-after-full-verification`
- `req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission`

**Result:** Current canonical semantics own stable, generation-checked rebuild identity and progress, and they bound jobs as a resource. They do not define one generic stable identity, resume, cancellation, and persisted-cursor contract for rollover, scrub, checksum, rebuild, mover, and other long-running work. v0.9 has operation-specific resumable cursors for rollover, rebuild, and mover but omits v0.8's complete general statement. Preserve the rule as **unresolved retained intent**. Later reconciliation may establish valid per-operation composition; it must not create an umbrella owner merely for symmetry.

### R5 — Independent recovery-state inspection

**Current closure inspected:**

- `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`
- `req.recovery-state-semantics.semantic-schema-migrations-and-exports-are-versioned-independently-of-sqlite`
- `req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms`
- `req.architecture-contract.recovery-and-repair-never-promote-algebraic-possibility-to-authority`
- `req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission`
- their deterministic prerequisite and direct-dependent closure

**Result:** Current canonical owners coherently define the portable manifest, six conservative inspection dispositions, format/schema versioning, bounded hostile-input behavior, and non-mutating interpretation. They do not require an independently invocable executable or prove that inspection works without production service, operator, or frontend private state. The missing C8 slice is therefore a **target-delta** limited to tool availability, shared rendering, process-status mapping, explicit experimental claim scope, and dependency isolation. A new operator-surface capability may compose the existing semantic owners without restating their recovery policy.

### R6 — Offline-rebuild record capacity and retirement

**Current closure inspected:**

- Bounded-resource owners: `req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission` and `req.store-operation-contracts.resource-admission-and-identity-remain-bounded-and-explicit`.
- Rebuild progress, verification, and promotion: `req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority`, `req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target`, `req.degraded-read-offline-rebuild.rebuild-resumes-and-completes-only-after-full-verification`, `req.degraded-read-offline-rebuild.replacement-promotion-preserves-logical-identity`, `req.anchorless-topology-identity.topology-transition-plans-bind-source-target-and-recovery`, and `req.anchorless-topology-identity.topology-transitions-are-staged-and-recoverable`.
- Recovery uncertainty and ownership: `req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic`, `req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent`, `req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred`, `req.explicit-transaction-machine.failure-abandonment-and-crash-states-are-conservative`, and `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`.
- Disposable control history: `req.file-backed-stores.disposable-control-sqlite-state-is-separate-from-recovery-authority`.
- Their deterministic prerequisite and direct-dependent closure.

**Result:** Current canonical authority defines bounded resource admission/export, generation-bound rebuild identity and cursor, exact per-range and final verification, a separate replacement and prepared topology, explicit durable topology commit/publication, conservative uncertainty, operation-slot ownership through terminal reconciliation, and disposable control history that cannot authorize recovery. It does not define when a detailed rebuild record stops being correctness-relevant or how record retirement interacts with claims, restart, or capacity exhaustion. Within this bounded closure, no current canonical rule makes successful rebuild count a lifetime allowance.

The accepted product direction is a **target-delta**: successful rebuilds do not consume a hidden lifetime allowance; configured rebuild-record capacity bounds records that remain correctness-relevant. The exact capacity, retirement predicate, retained audit/history, identifier reuse, migration, and mechanism remain unresolved. The operation-specific retirement boundary has an **ownership-or-scope-unresolved** disposition; it is not current product authority.

**Evidence boundary:** `crates/dwv-recovery/src/lib.rs` sets `RecoveryExportLimits::default().max_rebuilds` to 64 and rejects an exported snapshot with more records as `ExportLimitExceeded(OfflineRebuild)`; `crates/dwv-recovery/src/rebuild.rs::rebuild_export_is_bounded` exercises the fail-closed shape with a zero limit. The observed 65th-record refusal is bounded manifest-export implementation evidence, not current product authority or a lifetime rebuild allowance. The implementation's begin/advance/complete paths and absence of a retirement mutation are evidence only and do not settle the semantic boundary.

**Preserved boundaries:** A13 source evidence, A14/U14 claim-material rules in their existing C2/C3/C5–C7 scope, A15 evidence ordering, A16 operation ownership, and A17 bounded/observable resources remain separate owners. U36 does not redefine those owners, R4/U12 generic job composition, or U15 positive retention/GC policy; disposable control history remains reconstructible and non-authoritative.

**Next safe gate:** Determine the exact point after all ranges pass per-range and final verification, a prepared topology exists, durable topology commit and request-visible publication complete, restart reconciles any committed generation after interrupted publication, and any explicitly selected claim dependencies have been preserved in a verified retained copy or narrowed/retired by their owner, when the detailed rebuild record ceases to be authoritative. Only then may a bounded OpenSpec change be proposed. This campaign does not choose automatic eviction, identifier reuse, a numeric bound, migration, generic lifecycle/GC/audit policy, or an implementation mechanism; source retirement remains gated by Section 8.

### R7 — Long-horizon finite-resource audit

**Current closure inspected:**

- Linux trace and live resource owners: `req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe`, `req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority`, `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics`, and `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`.
- Fence and recovery owners: `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`, `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`, `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`, and `req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic`.
- Cross-cutting bound: `req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission`.
- The implementation producers, consumers, export limits, and reset or retirement paths for frontend trace records, home-write fence certificates, rebuild records, recovery-manifest bytes, and finite generation/tag identities.

**Result:** Current canonical semantics require bounded admission and explicit exhaustion, but they do not state that every finite retained-state capacity is renewable across the advertised service lifetime. The audit separated four independently advancing boundaries rather than creating one generic lifecycle rule.

1. **Linux trace retention has a current lifetime defect.** The canonical Linux requirement explicitly refuses another request when the configured trace count is retained, and the implementation stops the queue at 4,096 records. The validated planning change `make-linux-trace-retention-renewable` modifies that same owner: reserve before admission, reuse only the oldest fully terminal and reconciled record, disclose partial-session retention and saturated accounting, and never reclaim an incomplete or live owner. This is planned target behavior, not current authority. Bead chain `dwv-x6y.1.1` -> `.2` -> `.3` owns implementation through canonicalization.
2. **Home-write fence capacity is not ready for canonical retirement semantics.** `RecoverySnapshot::fences` is append-only and the default export limit is 16,384, but no current rule makes successful writes a lifetime allowance or defines fence liveness, compacted coverage, pre-intent capacity reservation, or atomic retirement. A fence remains live while any clean, integrity, session, uncertain, or selected historical claim needs its exact typed coverage. Beads `dwv-x6y.2.1`–`.3` preserve the separate witness, admission, and session-close joins; `.2.3` depends on `dwv-hg0.4`.
3. **Finite reusable identities need owner-local reset domains.** Checked exhaustion is safe; silent wrap or same-process reuse is not. A reset requires a new disambiguating namespace/incarnation or proof that every old channel and reference is fenced and reconciled. This remains U39 under `dwv-x6y.4`, not a universal counter framework.
4. **Recovery-manifest size is adapter conformance after liveness.** The 16 MiB SQLite profile must represent every admitted supported semantic snapshot or select an explicit unsupported/migration outcome before dependent mutation. It does not authorize truncation or retirement. `dwv-x6y.5` depends on the fence and rebuild liveness results.

**Invariant boundary:** One narrowed A17 long-horizon rule may be governed only after these owner-local rules are coherent: ordinary supported operation must not consume an undeclared irreversible lifetime allowance, while mathematically finite identities may still fail explicitly at pre-mutation exhaustion. Bead `dwv-x6y.6` depends on the trace, fence, rebuild, and identity work. U12 remains a separate operation-by-operation lifecycle audit under `dwv-x6y.7`.

## 4. Corrected operator capability map

Labels are disposable campaign navigation.

| Capability | Operator outcome | Safe claim | Stronger claim refused | Main dependencies and open joins |
|---|---|---|---|---|
| **C0a — Assess post-gap authority without mutation** | Inspect a complete, degraded, ambiguous, or recovery-needed array and distinguish lineage, custody continuity, current protection, prior protection, unprotected/indeterminate ranges, integrity, and next safe action. | The report states only what current observations and durable evidence support; successful inspection changes nothing. | Inspection does not authorize publication, writes, repair, current protection, or historical continuity. | No target capability dependency. R1; U01. |
| **C0b — Explain full recovery and claim status** | Add historical coverage, unknown ranges, advertised claim lifetime, progress, and broader human/machine explanations. | Every claim and unknown is range- and evidence-bounded. | Status does not manufacture missing authority or material. | C0a; U02, U14–U17. |
| **C1 — Use ordinary members through one mediated service** | Expose stable virtual members while healthy stopped payloads remain ordinary images. | One current writer mediates every required writable store. | Not a filesystem, backup, snapshot, multi-host writer, or per-file redundancy engine. | C0a; existing identity/writer semantics; U11, U21–U25. |
| **C2 — Restart a complete array after a custody gap** | Publish admitted present data read/write after stabilization and durable new-epoch admission without a full payload scan. | Present bytes are served; only committed active-epoch ranges are currently protected; a successful first protected write establishes its complete affected range. | No historical continuity, semantic correctness, full integrity, or full-member recoverability claim. | C0a, C1; U03–U08, U13, minimum U02. Not yet semantically closed. |
| **C3 — Restore current protection in background** | Advance untouched ranges while service continues. | Only generation-checked, durable commits increase current coverage; partial progress remains truthful. | Rollover completion does not imply current integrity or historical equality. | C2; U09, U10, U12–U14. |
| **C4 — Read or rebuild after a known loss** | Reconstruct exact current ranges and keep historical candidates and unknown ranges separate. | Current-basis known-erasure results are current only under the declared topology, basis, and source model. | A parity equation alone does not identify corruption or prove a prior version. | C0a; current recovery owners; U02, U18, U20, U36. |
| **C5 — Repair or change an array safely** | Check, scrub, repair, replace, resize, or change profile through a generation-bound plan, separate target, verification, and promotion. | Good sources remain unchanged and promotion follows exact evidence. | Solvability or a partial target does not authorize promotion. | C0a; C4 where reconstruction is used; U08, U10, U13–U16, U20, U36. |
| **C6 — Adopt selected present data into a new lineage** | Create fresh identity and protection from a complete selected present-data set when old lineage cannot be proved. | Selected bytes are the new starting data; no prior continuity is claimed. | Policy, readable bytes, parity agreement, or operator confidence do not recover old lineage. | C0a, C1; U04–U08, U13, U16, U18. |
| **C7 — Retain, restore, or retire historical claims** | Preserve or retire supporting material and, if selected, restore one named point or export provenance-preserving salvage. | Historical output names one point and exact covered ranges. | Validators do not generate bytes; mixed/unknown output is not a normal member. | C0b; U13–U17, U31. |
| **C8 — Recover and verify with independent tools** | Inspect documented persistent structures without production-daemon private state; later tools may verify equations, build current parity from complete data, export candidates, or explain recovery plans. | Each tool and format claim names its evidence tier and compatibility bounds. | Experimental schemas are not stable formats; file-backed evidence is not hardware durability; inspection does not authorize recovery or mutation. | R5; U26a, U26b, U27, U35. |

Operator-closure result: C0a is canonicalized. Independent recovery-state inspection is closed as a separate read-only C8 slice because every unfavorable artifact state remains a useful conservative result and no follow-on action is authorized. C0b remains coupled to unresolved history, claim-lifetime, retention, and release semantics; C2 remains open until immediate writes coordinate by coded range, checkpoint admission is closed, inherited blockers survive, and last-material consequences have a non-destructive default or explicit refusal.

## 5. Marked invariant ledger

All 21 v0.9 marked invariants were inspected through `knowledge architecture-candidate`. Invariant governance is pending for every row until activation or another accepted governed home exists. `Impact-mapped` is intentionally non-terminal.

| ID | Marked invariant | Capabilities | Detailed posture and evidence anchor | Open disposition |
|---|---|---|---|---|
| A01 | `arch.diskweave.conventional-member-block-parity` | All | **impact-mapped** to architecture-contract and healthy-portable-io owners returned by candidate impact | Reconcile current composition; govern retained invariant. |
| A02 | `arch.diskweave.portable-semantics-use-role-neutral-substrates` | All | **impact-mapped** to architecture-contract, store-operation, and XOR owners | Reconcile adapter/substrate boundary; govern invariant. |
| A03 | `arch.diskweave.authority-dimensions-remain-distinct` | All | **target-delta** for custody, range basis, and recovery meaning; current dimensions only impact-mapped | R1 plus later basis/history reconciliation; avoid umbrella duplication. |
| A04 | `arch.diskweave.identity-ambiguity-fails-closed` | C0a–C7 | **impact-mapped** to architecture-contract and anchorless-topology identity owners | Reconcile inherited applicability to every new transition. |
| A05 | `arch.diskweave.writable-service-has-one-mediated-writer` | C1, C2, C5, C6 | **impact-mapped** to file-backed, normalized, Linux, macOS, and portable owners | Reconcile portable publication and claim-release lifecycle. |
| A06 | `arch.diskweave.custody-gaps-open-new-protection-epochs` | C0a, C2–C4, C6 | **target-delta**; R1 proves current closure lacks custody/epoch meaning | U01, U03; governance. |
| A07 | `arch.diskweave.unproved-lineage-requires-explicit-new-lineage` | C0a, C6, C7 | **target-delta**; current metadata-loss semantics fail closed and withhold this positive action | U18, U19; governance. |
| A08 | `arch.diskweave.protection-basis-is-range-local-and-role-specific` | C0a–C7 | **target-delta**; current recovery/dirty/degraded owners are candidate surfaces, not closure proof | U01–U09, U18, U20. |
| A09 | `arch.diskweave.current-protection-gates-cross-member-propagation` | C2–C6 | **target-delta**; current write and degraded-read owners do not define post-gap basis | U04, U07; governance. |
| A10 | `arch.diskweave.recovery-meaning-is-explicit-and-unmixed` | C0a, C0b, C4, C5, C7 | **target-delta** beyond current conservative recovery owners | U01, U02, U16–U18. |
| A11 | `arch.diskweave.historical-evidence-remains-generation-bound` | C0b, C2–C5, C7 | **target-delta** beyond current checksum generation validity | U13–U18. |
| A12 | `arch.diskweave.algebraic-possibility-is-not-authority` | C0a–C8 | **impact-mapped** to architecture, evidence, degraded-read, and parity-verification owners | Reconcile composition and inherited applicability. |
| A13 | `arch.diskweave.recovery-preserves-source-evidence` | C2–C7 | **target-delta** beyond current non-mutating reads and separate-target repair/rebuild | U13, U16, U18, U20. |
| A14 | `arch.diskweave.recovery-claims-do-not-outlive-required-material` | C2, C3, C5–C7 | **target-delta**; no current impact links; F2 correctly found no complete current owner | U13–U16; governance. |
| A15 | `arch.diskweave.crash-safety-is-evidence-ordered` | C1–C8 | **target-delta** for epochs, basis, and claim retirement; current dirty/fence core is only impact-mapped | Apply to each new transition; do not create parallel crash policy. |
| A16 | `arch.diskweave.abandonment-does-not-end-operation-ownership` | C1–C7 | **impact-mapped** to normalized lifecycle and operation-slot owners | Reconcile applicability to rollover, retention, migration, and adoption. |
| A17 | `arch.diskweave.correctness-resources-are-bounded-and-observable` | All | **impact-mapped** to security, store, adapter, Linux, and evidence owners | R6–R7 and U36–U40 separate rebuild, trace, fence, identity, and representation bounds. Govern only after owner-local rules close under `dwv-x6y.6`; U10 fairness remains separate. |
| A18 | `arch.diskweave.semantic-granularities-remain-independent` | All | **impact-mapped** to request, store, XOR, dirty, and checksum owners | Reconcile protection-range and role-basis applicability. |
| A19 | `arch.diskweave.claims-remain-evidence-tier-specific` | C0a, C0b, C2, C4, C5, C7, C8 | **impact-mapped** to evidence-boundaries owner | Reconcile new restart/history/tool claims. |
| A20 | `arch.diskweave.format-interpretation-fails-closed` | C0a, C1, C2, C5–C8 | **impact-mapped** to architecture, recovery schema, and envelope owners | Reconcile epoch/history format evolution and migration refusal. |
| A21 | `arch.diskweave.stable-format-claims-require-independent-recovery` | C8 and every persistent capability | **target-delta**; no current impact links; current envelopes remain experimental | U26a, U26b, U27; govern without premature format stability. |

## 6. Consequential open and retained semantic ledger

These temporary IDs are burn-down handles, not permanent product identifiers.

| ID | Semantic boundary | Capabilities | Posture | Evidence and required disposition |
|---|---|---|---|---|
| U01 | Read-only post-gap lineage/custody/basis assessment | C0a | **canonicalized** | R1; archived OpenSpec `2026-08-14-add-post-gap-authority-assessment`; canonical owner `req.operator-recovery.production-assessment-is-observational-and-multidimensional`; implementation `src/operator.rs`, `src/cli.rs`; evidence `tests/operator_cli.rs` and operator unit tests; Beads `dwv-roy.1`, `.2`, `.3`. |
| U02 | Broader epoch/basis/history/unknown/claim-lifetime status | C0b; minimum projections elsewhere | **target-delta; mapped** | Current assessment owner exists; new dimensions require target semantics without authorizing action. |
| U03 | Typed store-scoped stabilization, durable epoch admission, and scan-independent publication | C2 | **target-delta; mapped** | v0.9 §§5.7, 8.4, 10.2–10.4; current identity/writer/start owners are inherited. |
| U04 | Fresh-basis foreground write/currentization for `Prior` or `Unprotected` ranges | C2, C5, C6 | **target-delta; mapped** | v0.9 §§6.8–6.9, 9.4; must never incrementally trust prior parity. |
| U05 | Proof-based reconciliation of `Indeterminate` basis | C2, C5 | **target-delta; mapped** | Split from currentization because it may settle surviving basis without destructive rebaseline. |
| U06 | Separately authorized present-data rebaseline of `Indeterminate` basis | C2, C5, C6 | **ownership-or-scope-unresolved** | Split because authorization, discarded claims, audit, and destructive consequences advance independently. |
| U07 | Global coded-range coordination across different member operations | C2–C5 | **unresolved retained target semantic** | R2; v0.8 §11.6 and v0.9 §8.6. Conflict domain is shared parity address, not target member or queue. |
| U08 | Closed-mutation-set checkpoint concurrency | C1–C3, C5 | **ownership-or-scope-unresolved** | R3; v0.8 §11.7 and v0.9 §9.11. Keep separate from lock mechanism and generic fence evidence. |
| U09 | Background rollover lifecycle, progress, restart, failure, and completion | C3 | **target-delta; mapped** | v0.9 §§9.13, 10.6–10.9. |
| U10 | Cross-operation fairness and starvation limits | C3, C5, C7 | **target-delta; mapped** | v0.8 §11.10 and v0.9 §9.13; independent of resource admission. |
| U11 | Portable shutdown, endpoint withdrawal, and claim-release ordering | C1, C2 | **OpenSpec planned; implementation blocked** | Change `add-portable-shutdown-claim-release` defines the target ordering but is not current authority. Implementation inspection found no service-owned durable writable-session begin/close path, so exact close-session evidence is unavailable. Bead `dwv-hg0.4` owns prerequisite reconciliation and blocks `dwv-hg0.1`; U03, U08, and U25 are review inputs only, not declared unblockers. |
| U12 | Long-running job identity, resume/cancel, and correctness cursor | C3, C5, C7 | **unresolved retained intent** | R4; v0.8 §19.4. Determine whether operation-specific owners compose completely; `dwv-x6y.7` owns the later bounded audit and must not create an umbrella owner for symmetry. |
| U13 | Historical artifact roles and coherent named recovery points | C0b, C4, C7 | **target-delta; mapped** | v0.9 §§7.7–7.9, 11.6–11.7, 12.3–12.5. |
| U14 | Baseline rule that an advertised recovery claim cannot outlive required material | C2, C3, C5–C7 | **target-delta; mapped** | Marked A14 and v0.9 §11.10. Must remain even if positive history features are deferred. |
| U15 | Positive retention extent, replication, expiry, and garbage-collection defaults | C0b, C3, C7 | **ownership-or-scope-unresolved; product decision** | Split from U14; no default is selected by this campaign. |
| U16 | Authorization classes and exact plans for exceptional destructive authority changes | C5–C7 | **target-delta; mapped** | v0.9 §15.3. Standing policy and explicit exceptional confirmation stay distinct. |
| U17 | Historical restore, named views, and provenance-preserving salvage release scope | C7 | **ownership-or-scope-unresolved** | Positive surface may be deferred; mixed/unknown-output prohibitions remain in U13/A10. |
| U18 | Explicit new-lineage adoption | C6 | **target-delta; mapped** | Current metadata-loss semantics deliberately withhold executable authority. |
| U19 | Prior-lineage recovery authority after metadata loss | C6, C7 | **deferred positive profile** | Baseline has no executable profile. Retain complete producer/validator/freshness/conflict boundary. |
| U20 | Topology/profile transitions under range-local basis | C5 | **target-delta; mapped** | v0.9 §§5.9, 6.11, 12.11; transitions inherit dirty, indeterminate, preservation, and claim obligations. |
| U21 | Whole-file namespace and placement semantics | C1 | **ownership-or-scope-unresolved** | v0.8 §§14.1–14.3; v0.9 §§13.1–13.3. Baseline no-striping boundary is distinct from a selected namespace product surface. |
| U22 | Cross-member mover transaction | C1, C5 | **ownership-or-scope-unresolved** | v0.8 §14.4; v0.9 §13.4. Independent copy/commit/delete, crash, resume, and source-preservation lifecycle. |
| U23 | Staging/tier protection-status semantics | C0b, C1 | **ownership-or-scope-unresolved** | v0.8 §14.5; v0.9 §13.5. Must not be swallowed by namespace placement or imply parity protection. |
| U24 | Optional encryption integration surface | C1 | **ownership-or-scope-unresolved** | v0.8 §15.1; v0.9 §13.6. Optional and not a baseline publication blocker. |
| U25 | Baseline deployment/startup/shutdown dependency ordering | C1, C2 | **target-delta; mapped** | Split from optional encryption; v0.8 §§15.3–15.6 and v0.9 §§13.7–13.8. |
| U26a | Independent read-only recovery-state inspection | C8 | **canonicalized** | `req.independent-recovery-inspection.independent-recovery-state-inspection-is-bounded-and-non-authorizing`; implementation `crates/dwv-recovery-sqlite/src/recovery_inspect.rs`; evidence `crates/dwv-recovery-sqlite/tests/recovery_inspect.rs`; closed Bead chain `dwv-bno` -> `.1` -> `.2` -> `.3`. The command remains experimental and authorizes no U26b or U27 action. |
| U26b | Independent equation verification, current parity build, candidate export, recovery-plan explanation, migration, and damaged/unknown drills | C8 | **split: C8b canonicalized; remainder target-delta** | `openspec/specs/independent-parity-verification/spec.md` now owns only standalone bounded single-XOR equation verification under `req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing`; archived change `openspec/changes/archive/2026-08-14-add-bounded-parity-scrub/` preserves the transition record. Implementation `crates/dwv-verify/src/independent_parity.rs`, focused evidence `crates/dwv-verify/tests/independent_parity.rs`, evidence record `docs/verification/independent-parity-verification.md`, and closed Bead chain `dwv-ykl` -> `.1` -> `.2` -> `.3` establish only that bounded file-backed/model observation. Current parity build, export, plan explanation, migration, and damaged/unknown drills remain open U26b remainder; U26a, U11, U27, and U28 remain separate. |
| U27 | Stable-format graduation claim gate | C8 | **target-delta; mapped** | A21. Separate from building tools so a partial tool does not imply stable format. |
| U28 | P/Q production profile and exact codeword coherence | C4, C5 | **deferred positive profile** | No baseline P/Q fields or implementation duty; same-codeword rule remains at profile boundary. |
| U29 | Strict pre-publication verification/currentization | C2 | **deferred optional profile** | Full scan does not prove historical continuity. |
| U30 | Managed-member profile | C1, C2 | **deferred optional profile** | No baseline container, reserve, field, or API. |
| U31 | Certified custody-continuity profile | C0a, C2 | **deferred optional profile** | Evidence ceiling remains active; ordinary detectors are not proof. |
| U32 | Shadow-history profile | C7 | **deferred optional profile** | Historical material roles and claim lifetime remain active without it. |
| U33 | Mediated standalone read/write sessions | C1, C2 | **deferred optional profile** | Future profile needs starting basis, absent-role, crash, integrity, custody, and rejoin semantics. |
| U34 | Production degraded writes | C4 | **deferred; baseline prohibited** | Current complete-assignment and read-only degraded semantics continue to block the positive feature. |
| U35 | Live macOS bridge | C8 | **evidence-only gap, not selected dependency** | Current regular-file fixture/trace claim remains narrow; no portable semantic change follows. |
| U36 | Offline-rebuild record capacity, retirement, and lifetime exhaustion | C4, C5 | **ownership-or-scope-unresolved; not OpenSpec-ready** | R6–R7; accepted direction is no hidden lifetime allowance: configured capacity bounds correctness-relevant records. Current authority does not bind rebuild identity to prepared topology/committed publication, define block/cancel/rollback outcomes, or close claim-material handoff. `dwv-ee6` closed the initial campaign result; `dwv-x6y.3.1`–`.3` preserve these prerequisites before `dwv-x6y.3` can define retirement. Separate from R4/U12 and U15; preserve A13–A17. |
| U37 | Mounted-service trace continuity and renewable diagnostic retention | C1 | **OpenSpec planned; not current authority** | R7; validated change `make-linux-trace-retention-renewable` preserves the current Linux owner ID and defines a fixed 4,096-record terminal-only rolling window, partial-session disclosure, non-gating saturation, and conservative incomplete reservations. Execute `dwv-x6y.1.1` -> `.2` -> `.3`; no durable history or stable-format promise. |
| U38 | Home-write fence capacity, compacted coverage, and retirement | C1–C5 | **ownership-or-scope-unresolved; not OpenSpec-ready** | R7; the 16,384-fence export limit is implementation evidence, not a lifetime policy. `dwv-x6y.2.1`–`.3` own compacted witness liveness, pre-intent capacity reservation, and session-close supersession. Until exact atomic handoff is canonical, preserve fail-closed capacity and never reuse post-intent or uncertain evidence. |
| U39 | Finite reusable identity exhaustion and reset domains | All reusable operation identities | **unresolved retained target semantic** | R7; checked exhaustion remains explicit and pre-mutation. `dwv-x6y.4` must reconcile owner-local reset/incarnation rules and stale-reference rejection without widening durable recovery generations or topology epochs into a universal rollover policy. |
| U40 | Recovery-manifest representability under retained semantic state | C0a–C7 | **deferred dependency** | R7; `dwv-x6y.5` follows U36/U38 liveness. The 16 MiB SQLite profile is adapter conformance: every admitted supported snapshot fits, or admission selects explicit unsupported/migration behavior before dependent mutation. No truncation or silent record retirement. |

## 7. Independent source-coverage audit

### 7.1 v0.9 beginning-to-end audit

| Source cluster | Campaign disposition |
|---|---|
| Executive decision and §§1–3 | Product boundary and all 21 marked invariants map to A01–A21; target authority dimensions map to U01–U20. |
| §§4–5 layering, ownership, stores, topology, identity | Current review surfaces remain impact-mapped; writer/publication/claim lifecycle is U03, U11, U25. |
| §6 geometry, coding, basis | Current geometry surfaces remain impact-mapped; basis/currentization/coordination are U04–U08; P/Q is U28. |
| §7 recovery state and formats | Current recovery/schema surfaces remain impact-mapped; history and stable-format/tool boundaries are U13–U15, U26a–U27; retained-state representability is U40. |
| §§8–9 operations, write, crash, resources | Exact current owners remain review inputs; omitted F2 coordination and checkpoint closure are U07–U08; fairness/shutdown are U10–U11; fence capacity and retirement is U38. |
| §10 lifecycle/startup/rollover | C0a/C2/C3 and U01–U12 cover every consequential transition; strict startup is U29. |
| §§11–12 integrity and recovery | A10–A14 and U13–U20, U36, U38 cover current, historical, unknown, preservation, repair, rebuild, adoption, and retained recovery-evidence capacity; degraded writes are U34. |
| §13 namespace, encryption, deployment | Split into U21–U25 so optional encryption cannot swallow baseline deployment and mover/staging can advance independently. |
| §14 platforms | Current Linux/macOS review surfaces remain impact-mapped; mounted Linux trace continuity is U37; live macOS bridge is U35; platform durability remains evidence-tier bounded. |
| §15 security, operator, observability | Resource/claim invariants A14, A17, A19 and U01–U02, U15–U16, U36–U40 cover normative and unresolved long-horizon boundaries; privacy/threat statements remain review inputs, not new behavior owners. |
| §16 verification and tools | A19, A21, U26a–U27, U35 preserve claim gates. Evidence does not create target semantics. |
| §17 alternatives/future profiles | U28–U34 preserve each positive deferral separately; rejected architectures are non-normative rationale. |
| §18 relationship/judgments | Explicit v0.8 preservation/change statements were challenged against the backward audit below rather than accepted as self-proof. |
| Appendices | Glossary supports mapped target semantics; source/rationale index is archaeology, not authority. |

### 7.2 Backward v0.8 transition audit

The v0.8 source was walked by semantic cluster, then compared with v0.9 and F2. This is an independent transition check, not reliance on v0.9 §18.6's self-review.

| v0.8 cluster | Later treatment | Result |
|---|---|---|
| §§1–4 product boundary, ordinary members, recovery promise, portable layers, coherent exposure | Preserved in v0.9 §§1–4 and A01–A05 | No older-only consequence found. Current entailment remains to reconcile; no terminal downward disposition is claimed. |
| §§5–7 process, authority classes, identity, topology, coding, geometry | Preserved or deliberately extended in v0.9 §§4–6 and A02–A09 | Custody/basis is explicit superseding target meaning; no silent removal found. |
| §§8–10 recovery state, formats, normalized operations, write/crash protocol | Preserved and extended in v0.9 §§7–9 | Checkpoint closure was under-accounted by F2 and is now U08. |
| §11 transaction/executor concurrency | Preserved in v0.9 §§8–9 | **Consequential F2 omission:** global coded-range coordination U07. Checkpoint concurrency U08 and fairness U10 remain separate. |
| §§12–13 reads, repair, rebuild, integrity | Preserved and extended in v0.9 §§11–12 | Current/historical meanings map to A10–A14 and U13–U20. No older-only recovery authorization was inferred from history. |
| §14 namespace, mover, staging | Preserved in v0.9 §13 | F2's one combined row was too broad; split U21–U23. |
| §15 encryption and deployment lifecycle | Preserved in v0.9 §§13–14 | F2 combined an optional surface with baseline ordering; split U24–U25 and U11. |
| §§16–18 technologies and alternatives | v0.9 replaces provisional transaction/macOS posture explicitly and retains portable seams | Supersession is explicit; mechanism details are non-normative or deferred. |
| §19 security/control plane | Mostly preserved in v0.9 §15 | **Consequential older-only remainder:** general long-running job identity/resume/cancel rule is U12. Local authenticated control, inspection/mutation separation, bounds, and privacy remain later review inputs. |
| §§20–21 status and evidence | Preserved and extended in v0.9 §§15–16 | U01–U02 and A19/A21 preserve current versus target claim boundaries. |
| §§22–24 phases, gates, decision tables | v0.9 keeps architectural decisions but intentionally omits old work allocation | Historical sequencing is non-authoritative; no milestone/OpenSpec allocation is imported. |
| §25 authority and implementation-agent guidance | Replaced by repository authority rules and current skills | Non-product procedure; no semantic row required. |

**Backward-audit result:** Older architecture contained consequential meaning that F2 had not safely accounted for: coded-range coordination, checkpoint admission closure, general long-running job lifecycle, and independently advancing namespace/deployment boundaries. These are now explicit. No other consequential v0.8 semantic was found stranded outside v0.9, current canonical review surfaces, or this ledger. This is not source-retirement proof because many dispositions remain non-terminal.

## 8. Finite burn-down and completion gate

### 8.1 Burn-down procedure

No row disappears by editing this campaign alone.

1. **Reconcile marked current closure.** Process A01–A21 in these bounded semantic families, recording compact reconciliation anchors: product/portable boundaries; identity/authority/writer; dirty/crash/operation lifecycle; recovery/history/preservation; evidence/format. A row becomes `current-entailed` only after the bounded result proves complete composition.
2. **Advance target rows by operator capability.** Start with U01/C0a. Later choose only an operator-closed capability from Section 4. Use normal OpenSpec work to settle target meaning; do not preselect requirement IDs or change decomposition here.
3. **Resolve independent open rows.** Every U01–U40 must reach one permitted terminal disposition: canonicalized target, retained durable deferral, accepted removal, justified non-normative result, or evidence-only result attached to an already owned claim.
4. **Govern invariants.** Every retained A row must obtain a durable governed home and valid current impact links.
5. **Re-run transition inheritance.** For every new publication, epoch, basis, recovery, topology, retention, or adoption path, show that current blockers, indeterminate states, preservation duties, operation ownership, and retained claims survive or are explicitly superseded.
6. **Run a final independent source walk.** Recheck all v0.8 and v0.9 consequential clusters against terminal A/U dispositions and current canonical authority.
7. **Prove source exclusion.** With transitional architecture unavailable as ordinary semantic input, a repository-supported bounded reconstruction must still recover current behavior, governed invariants, durable deferrals/removals, claim limits, and remaining implementation obligations.
8. **Delete this campaign.** Retirement is complete only when a fresh agent no longer needs F2, v0.8, v0.9, or this file to answer a correctness-relevant current product question.

### 8.2 Completion gate

Normalization is complete only when all are true:

- every A01–A21 row has settled detailed semantics, complete capability applicability, invariant governance, and no sole-source dependence on v0.8/v0.9;
- every U01–U40 row has one terminal durable disposition;
- every terminal current-semantic conclusion has a still-applicable bounded reconciliation anchor;
- no umbrella owner restates valid composition and no independent policy remains scattered across composers;
- no new transition bypasses a current blocker, pending obligation, preservation duty, indeterminate state, lifecycle obligation, or retained claim;
- optional/deferred features do not swallow baseline refusal, authority, status, preservation, compatibility, progress, or claim rules;
- readiness is green and architecture-induced dependent reviews were resolved individually;
- the source-exclusion proof passes without supplying the retiring source as ordinary semantic input;
- focused evidence proves only the capability and tier claimed.

The gate is **not satisfied**. Source exclusion is premature while A and U rows remain open.

## 9. Selected and durable capability boundaries

### 9.1 C0a — Assess post-gap authority without mutation

**Operator story:** After reboot, an operator points DiskWeave at a complete candidate array. DiskWeave re-observes bounded identity, topology, recovery, envelope, and integrity evidence without taking writable claims, creating or migrating state, scanning payload solely to manufacture authority, or changing any payload. The result distinguishes accepted lineage from unproved custody continuity and separately reports whether each relevant range/role is current, prior, unprotected, indeterminate, or not yet interpretable. It explains that no mutation or publication was authorized and names the next safe action or blocker.

**Safe claim:** The assessment truthfully classifies observed authority and uncertainty under current evidence. A successful command means the observation completed, not that the array is writable or protected.

**Stronger claim refused:** It does not open a protection epoch, stabilize stores, authorize historical continuity, establish current parity, permit degraded reconstruction, clear dirty/indeterminate state, or select a destructive recovery action.

**Why this boundary is closed:**

- R1 found a coherent current observational owner and no canonical contradiction.
- The target delta is bounded to new classification dimensions and conservative explanations.
- The action is read-only, so U03–U16 mutation, coordination, checkpoint, retention, and authorization semantics are not prerequisites.
- Identity ambiguity, unsupported formats, conflicting evidence, missing recovery state, and unknown basis can remain explicit successful observations with blocked action.
- The first possible follow-on action is not silently authorized; it must enter C2, C4, C6, or C7 and satisfy that capability's independent gates.

**Durable work anchors (2026-08-14):**

- Archived OpenSpec `openspec/changes/archive/2026-08-14-add-post-gap-authority-assessment/` modifies the existing `req.operator-recovery.production-assessment-is-observational-and-multidimensional` owner. It preserves the stable requirement identity and adds C0a's lineage, custody, role-local basis, bounded output, and non-authorization semantics.
- Implementation: `src/operator.rs` and `src/cli.rs`; structured result cut over to `dwv.operator.v2`.
- Evidence: `src/operator.rs` operator unit tests and `tests/operator_cli.rs`; `cargo test -p diskweave --bin dwv operator::tests` and `cargo test -p diskweave --test operator_cli` passed. Knowledge readiness, docs check, and docs build passed with zero diagnostics.
- Beads `dwv-roy.1`, `.2`, and `.3` are complete. C0a is canonicalized; these anchors authorize no C0b, C2, recovery-state mutation, publication, write, repair, or historical-recovery work.
- The retained Linux acceptance artifact was refreshed on 2026-08-14 from the active runner and now carries `dwv.operator.v2`; its exact input and output digests are recorded in `docs/verification/linux-ublk-ext4-acceptance.md`. Bead `dwv-cfh` is complete. Historical M9 `dwv.operator.v1` evidence remains explicitly historical and is not rewritten.

### 9.2 C8a — Inspect recovery state independently without mutation

**Operator story:** An operator invokes one standalone command against one recovery-state artifact. The command uses the documented read-only adapter without the production service, operator workflow, frontend, writable ownership, initialization, migration, repair, or payload access. It reports `absent`, `supported`, `corrupt-or-unreadable`, `unsupported`, `migration-required`, or `reconciliation-required` through equivalent bounded human or structured output.

**Safe claim:** The command independently observed the artifact and reported the portable semantics it could establish. Every classification is a successful observation, not a writable, recovery, or format-stability decision.

**Stronger claim refused:** Inspection does not authorize writable interpretation, publication, mutation, repair, migration, parity or integrity, lineage or custody, historical recovery, or stable-format graduation.

**Why this boundary is closed:**

- R5 found complete current owners for classification, manifest meaning, schema/version facts, bounds, and non-mutation.
- The target delta is only an independently invocable command, one shared result, process-status mapping, explicit experimental status, and a dependency boundary.
- The action is read-only and every unfavorable artifact state remains a truthful bounded result.
- The first possible follow-on action is explicitly unauthorized and must enter separately canonicalized migration, repair, recovery, or format-governance semantics.
- C0b and C2 were not selected because their unresolved history/retention and mutation/coordination joins still prevent operator closure.

**Durable work anchors (2026-08-14):**

- Canonical owner `openspec/specs/independent-recovery-inspection/spec.md` now defines the bounded, non-authorizing command contract after archive `2026-08-14-add-independent-recovery-state-inspection`.
- Implementation `crates/dwv-recovery-sqlite/src/recovery_inspect.rs` reuses the portable `RecoveryInspection` seam and the SQLite read-only adapter; the adapter now bounds artifact admission and manifest extraction and preserves every safely known format fact.
- Evidence `crates/dwv-recovery-sqlite/tests/recovery_inspect.rs` covers all dispositions, renderer equivalence, deterministic statuses, hostile bounds, byte-for-byte non-mutation, and dependency isolation. Beads `dwv-bno`, `.1`, `.2`, and `.3` are closed.
- This closure authorizes no U26b tooling, migration, repair, recovery, parity construction, candidate export, or stable-format claim. U27 remains open.

### 9.3 U11 — Stop a running service without releasing live ownership early

**Operator story:** An operator stops a running service. DiskWeave closes new admission, quiesces frontends and namespace writers, drains or durably hands off admitted work, reconciles indeterminate completions where possible, obtains only exact owner-approved checkpoint and close-session evidence, withdraws exported writable endpoints, and releases store/recovery claims only after no writable alias can remain. The result distinguishes clean completion from failure or reconciliation-required state.

**Safe claim:** A clean result means the bounded shutdown sequence and its required evidence completed for the admitted service. A non-clean result preserves conservative dirty, indeterminate, stale-endpoint, and reconciliation consequences.

**Stronger claim refused:** Shutdown does not establish custody continuity, current or historical protection, lineage, recovery authority, payload integrity, publication, currentization, retention, repair, migration, or stable-format status. A flush, timeout, forced stop, process exit, unmount, endpoint removal, or in-memory transition cannot manufacture a clean close.

**Why planning is retained but implementation is blocked:**

- Current owners define frontend abandonment, operation-slot lifetime, transaction outcomes, dirty/restart consequences, exact store watermarks, typed checkpoint evidence, recovery writable-session mutations, and Linux endpoint/descriptor behavior.
- `RecoveryMutation` exposes writable-session begin/close semantics, but `HealthyPortableService` has no service-owned durable begin/close path. The discarded implementation attempt therefore supplies no accepted evidence.
- Bead `dwv-hg0.4` must reconcile one canonical owner and durable OpenSpec boundary for session begin/close and failure/restart consequences before product implementation.
- U03, U08, and U25 are review inputs because they touch adjacent transitions; none is declared the unblocker.

**Durable work anchors (2026-08-14):**

- OpenSpec change `openspec/changes/add-portable-shutdown-claim-release/` contains the target proposal, healthy-portable-io delta, Linux refinement, design, and blocked dependency-ordered tasks. It is not current canonical authority or implementation-ready work.
- Bead `dwv-hg0.4` blocks implementation Bead `dwv-hg0.1`; `.2` remains focused evidence after implementation, and `.3` remains verification/archive/canonicalization.
- Durable handoff is the integrated OpenSpec and Bead graph. A future agent starts from integrated main in a fresh owned jj workspace under `work/workspaces/`; it does not depend on the discarded implementation workspace.
- These anchors authorize no U11 product implementation, startup/publication, currentization, recovery mutation, retention, history, deployment, or recovery-inspection work.

### 9.4 C8b — Verify a bounded parity equation independently

**Operator story:** A recovery investigator invokes one standalone read-only verifier with an experimental descriptor naming bounded single-XOR protected geometry, a selected range, and distinct data/parity payloads. The verifier reads only declared ranges, applies only explicitly declared logical zero tails, checks each equation byte-for-byte, and returns one deterministic human or structured result and process status.

**Safe claim:** `matched`, `mismatched`, `incomplete`, `unknown`, invalid, unsupported, and resource-bound outcomes are bounded observations at the declared file-backed/model evidence tier.

**Stronger claim refused:** A result does not establish production identity, generation, custody, current or historical protection, checksum validity, hardware durability, clean state, a bad shard, repair authority, recovery authority, writable publication, or stable-format status.

**Why this boundary is independently closed:**

- Existing XOR, exhaustive parity-verification, evidence-scope, fail-closed uncertainty, deterministic-artifact, and resource-bound requirements remain the detailed owners.
- The target delta owns only independent invocation, experimental descriptor binding, bounded reads and reports, equivalent rendering, process status, and explicit non-authorization.
- It does not inspect recovery state and does not depend on C8a/U26a implementation results, U11, production service/operator/frontend private state, recovery databases, implementation feedback, or another open campaign item.
- Current parity build, candidate export, recovery-plan explanation, migration, and damaged/unknown drills remain separate U26b work; P/Q and stable-format graduation remain U28 and U27.

**Durable work anchors (current canonical, 2026-08-14):**

- Canonical owner `openspec/specs/independent-parity-verification/spec.md` now defines the standalone bounded read-only equation-verification contract. Its requirement identity is `req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing`; the archived change is the source-retirement proof, not current semantic authority.
- Implementation `crates/dwv-verify/src/independent_parity.rs` is linked to the canonical requirement at the standalone command boundary. It performs pre-read bounded admission, nonblocking regular-file admission on supported Unix targets with fail-closed unsupported-platform behavior, duplicate-member and explicit-null rejection, checked geometry, read-only canonical identity checks, equivalent structured/human semantic results, deterministic status, and non-panicking stderr handling. This implementation realizes the canonical contract but does not create authority.
- Focused evidence `crates/dwv-verify/tests/independent_parity.rs` and `docs/verification/independent-parity-verification.md` covers matching and mismatched equations, incomplete and unavailable payloads, logical zero tails, invalid/unsupported/aliased/bounded input, unchanged data/parity/descriptor digests, human/JSON equivalence, deterministic statuses, bounded resources, experimental scope, and the dependency boundary. `cargo test -p dwv-verify --test independent_parity` passed with 2 tests; `cargo tree -p dwv-verify --edges normal` showed no production service, frontend, SQLite, or independent recovery-inspection dependency.
- Bead `dwv-ykl.1` is closed after implementation; `dwv-ykl.2` and `.3` are closed after focused evidence, canonicalization, archive, and documentation gates; the parent `dwv-ykl` closes last. The archived change preserves source-retirement proof while the main spec is current authority.
- This closure authorizes no C8a/U26a recovery-state work, U11 shutdown, parity construction, candidate export, recovery-plan explanation, migration, damaged/unknown drills, repair, or stable-format graduation. Current parity build and export/recovery-plan semantics remain separate U26b work; P/Q and stable-format graduation remain U28 and U27.

### 9.5 C2-PROV — Durable range provenance candidate (nonblocking)

**Candidate intent:** A future durable provenance ledger could retain compact historical facts about range-level operations and authority transitions, such as scrub or verification observations, protection-basis changes, repairs, and claim transitions. A record would preserve the bounded range, operation or generation identity, observed outcome, causal relation, and evidence reference needed for later forensic questions.

**Boundary:** This is an implementation/architecture candidate, not a current requirement, authority source, recovery plan, or blocker for C8b or the next campaign slice. It cannot recreate lost payload bytes, prove custody continuity, manufacture current protection, or replace recovery state and canonical capability owners.

**Open owner questions:** A future bounded reconciliation must decide event meaning, crash/interruption semantics, retention and safe compaction, identity and generation binding, uncertainty representation, and how the ledger composes with U07–U08, U13–U17, U36, and U38 without creating a second semantic registry. No implementation, Bead dependency, or product claim is authorized by this record.

**Rejected adjacent candidates:** current parity build needs complete-data and output/persistence semantics; export and plan explanation need provenance, claim-lifetime, and recovery-authority semantics; migration and damaged/unknown drills need unresolved transition semantics; C0b, C2, C5, and C7 remain blocked by their recorded history, mutation, coordination, checkpoint, retention, or authorization joins.

## 10. Resumption record

Start a future pass with:

```text
cargo xtask docs knowledge readiness
cargo xtask docs knowledge architecture-candidate arch.diskweave.v0.9
```

Then retrieve one bounded current context packet for the selected A/U semantic family. Do not rerun overlapping `inspect`, `ownership`, and `context` requests for the same need.

Campaign repair reconciliation anchors:

```text
cargo xtask docs knowledge context \
  req.operator-recovery.production-assessment-is-observational-and-multidimensional \
  req.operator-recovery.declarative-array-policy-locates-but-does-not-authorize \
  req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred \
  req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative

cargo xtask docs knowledge context \
  req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity \
  req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed \
  req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation

cargo xtask docs knowledge context \
  req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence \
  req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences \
  req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence

cargo xtask docs knowledge context \
  req.recovery-state-semantics.offline-rebuild-progress-is-durable-semantic-authority \
  req.degraded-read-offline-rebuild.rebuild-resumes-and-completes-only-after-full-verification \
  req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission

cargo xtask docs knowledge context \
  req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout \
  req.recovery-state-semantics.semantic-schema-migrations-and-exports-are-versioned-independently-of-sqlite \
  req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms \
  req.architecture-contract.recovery-and-repair-never-promote-algebraic-possibility-to-authority \
  req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission
```

All five campaign-repair packets succeeded at this snapshot. C0a, C8a, and C8b are canonicalized; C8a is implemented and evidenced under `req.independent-recovery-inspection.independent-recovery-state-inspection-is-bounded-and-non-authorizing`, and C8b is implemented and evidenced under `req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing`. Beads `dwv-bno` -> `.1` -> `.2` -> `.3` and `dwv-ykl.1` -> `.2` -> `.3` are closed; the parent `dwv-ykl` closes last. U11 remains blocked on `dwv-hg0.4`. R7/U37–U40 and epic `dwv-x6y` preserve long-horizon work: `dwv-x6y.1.1` is the next ready trace implementation; fence and rebuild retirement are not OpenSpec-ready and retain their child prerequisites; identity and manifest work remain tracked. Do not start U11, C0b, C2, U26b migration/export/recovery-plan work, repair, or stable-format work from this selection.
