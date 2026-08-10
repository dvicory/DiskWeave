# Milestone 8 canonical semantic ownership — implementation review follow-up 1

Date: 2026-08-09  
Change: `canonical-semantic-ownership`  
External verdict addressed: `REJECTED pending one implementation correction and one bounded evidence verification`  
Repository: `/Users/daniel.vicory/src/DiskWeave`

## 1. Disposition

Both requested corrections are complete.

1. The approved local/effective fingerprint contract was already implemented correctly. The inaccurate statement in `/Users/daniel.vicory/tmp/milestone8-canonical-semantic-ownership-implementation-review.md` was corrected, and the requested behavioral regression matrix was added to `xtask/src/knowledge.rs`.
2. The Linux correction-evidence retirement remap was challenged mapping by mapping. Exactly one evidence row changed. Its only mapping change was removal of the retired umbrella requirement; all 23 direct current-requirement mappings remain supported by the bounded live record plus its explicitly linked deterministic/model evidence. No mapping, claim, or non-claim needed further narrowing.

The OpenSpec change remains unarchived. `openspec list --json` reports `canonical-semantic-ownership` as `in-progress`, 21/22 tasks complete. No `metadata-certificate-authority-gate` change was started.

## 2. Fingerprint contract correction

### 2.1 Actual implementation

The implementation separates the two fingerprint levels exactly as approved:

- `xtask/src/knowledge.rs:93-219` (`knowledge_model`) slices each requirement unit, calls `normalize_markdown`, hashes that normalized body at line 146, extracts relationship markers separately, sorts the relationship records, and then computes effective fingerprints.
- `xtask/src/lib.rs:470-555` (`normalize_markdown`) converts normative Markdown structure and text into formatting-stable records. HTML comments beginning with `<!--` are deliberately ignored by the event match, so intrinsic `dwv:req` IDs and `dwv:requires`/`dwv:refines` markers do not enter the normalized body or local digest.
- `xtask/src/knowledge.rs:226-309` (`extract_relationships` and `parse_relationship_marker`) parses the authored relationship block separately from local prose.
- `xtask/src/knowledge.rs:443-468` (`effective_fingerprint`) hashes the local fingerprint plus each sorted outgoing `(kind, target ID, target effective fingerprint)` tuple recursively.
- `ReviewedState.outcomes` and `ReviewedState.reasons` are read/write review metadata. Neither field participates in `knowledge_model`, `normalize_markdown`, or `effective_fingerprint`.

Therefore:

- **local semantic fingerprint** = normalized local normative prose and scenarios only, excluding intrinsic IDs and relationship markers;
- **effective semantic/review fingerprint** = local fingerprint plus sorted outgoing relationship kind, target identity, and target effective fingerprint, propagated transitively.

The original review report incorrectly said the local fingerprint included relationship markers. That sentence now states the contract above and records the focused regressions. No production implementation change was required.

### 2.2 Focused regression matrix

| Change | Local fingerprint | Source effective fingerprint | Dependents / unrelated requirements | Regression |
|---|---:|---:|---|---|
| Local normative prose changes | changes | changes | all transitive dependents change; unrelated requirement remains stable | `effective_fingerprints_invalidate_dependency_closure_only` |
| Relationship added | unchanged | changes | readiness reports `semantic-prerequisite-changed` | `relationship_only_change_is_semantic_and_ownership_output_is_bounded` |
| Relationship removed | unchanged | returns to original | source effective fingerprint returns to original | `relationship_only_change_is_semantic_and_ownership_output_is_bounded` |
| Relationship target changes | unchanged | changes | only relationship-sensitive closure is affected | `relationship_target_kind_and_order_change_only_effective_fingerprints` |
| Relationship kind changes | unchanged | changes | only relationship-sensitive closure is affected | `relationship_target_kind_and_order_change_only_effective_fingerprints` |
| Same valid relationship markers reordered | unchanged | unchanged | unchanged | `relationship_target_kind_and_order_change_only_effective_fingerprints` |
| Formatting-only prose reflow | unchanged | unchanged | unchanged | `formatting_and_review_metadata_leave_semantic_fingerprints_stable` |
| Reviewed outcome/reason changes | unchanged | unchanged | unchanged | `formatting_and_review_metadata_leave_semantic_fingerprints_stable` |

Observed focused result:

```text
cargo test -p xtask fingerprint
cargo test: 4 passed (2 suites, 28 filtered, 0.00s)

cargo test -p xtask relationship_only_change_is_semantic_and_ownership_output_is_bounded
cargo test: 1 passed (2 suites, 31 filtered, 0.00s)

cargo test -p xtask invalid_relationship_edges_fail_with_structured_diagnostics
cargo test: 1 passed (2 suites, 31 filtered, 0.00s)
```

## 3. Missing-edge boundary

Deterministic tooling validates authored representation; it does not infer undeclared semantic relationships.

- `knowledge_model` obtains relationships only from `extract_relationships`.
- `extract_relationships` recognizes only exact contiguous `<!-- dwv:requires <id> -->` and `<!-- dwv:refines <id> -->` markers immediately following the intrinsic ID.
- There is no hard-coded owner/edge registry and no lexical prose matcher that invents a relationship.
- A requirement with no relationship markers is structurally valid. The focused test writes one, then successfully extracts exactly one requirement with zero relationships.
- A syntactically incomplete marker such as `<!-- dwv:requires -->` fails mechanically with `invalid_relationship_marker`.
- A valid marker outside the contiguous post-ID block fails mechanically with `misplaced_relationship_marker`.
- Unknown targets, retired targets, self edges, duplicate edges, mixed-kind pairs, and cycles also fail mechanically.

A semantically missing but syntactically absent relationship is not a deterministic-parser verdict. Canonical-spec authorship and the semantic-reconciliation process decide whether the omission is consequential. `.agents/skills/diskweave-semantic-reconciliation/SKILL.md` tells the reasoning agent to use typed relationships when available and, when they are unavailable, inspect current canonical prose and ownership evidence to establish the bounded semantic closure. That manual semantic decision is not encoded as hidden tool inference.

## 4. Linux correction-evidence remap

### 4.1 Changed-row table

| Evidence ID | Evidence path | Exact demonstrated claim | Previous requirement mapping | Final requirement mapping | Why each final mapping is actually proved | Explicit non-claims |
|---|---|---|---|---|---|---|
| `evidence.linux-ublk-ext4-acceptance` | `docs/verification/linux-ublk-ext4-acceptance.md` (machine facts: `verification/linux-ublk-ext4-acceptance.json`; retained traces: `verification/linux-ublk-trace-first.json`, `verification/linux-ublk-trace-second.json`; model: `verification/tla/RecoveryProtocol.tla`) | One exact ARM64 Linux ublk environment completed bounded ext4 create/fsync/overwrite/rename/directory-sync/delete, clean shutdown, restart, and read-only remount through DiskWeave; the record also retains bounded deterministic frontend, transaction, dirty-region, fence, recovery, ownership, and lifecycle evidence. | The 23 final mappings M1–M23 below **plus** retired `req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries`. | Exactly M1–M23 below. The retired umbrella mapping is removed; no new mapping was added. | Each mapping was challenged independently below. The record directly links the live JSON/traces and the named Rust/TLA/Kani checks. No mapping is justified merely by transitive graph membership or former correction-program membership. | No production concurrency, daemon/crash recovery, or deployment correctness; no raw-device, physical power-loss, or hardware durability; no FUA, discard, write-zeroes, broader-filesystem, multi-device-publication, or online-topology-mutation support. Trace replay is validation-only and does not certify a live backend, application consumption, or physical durability. |

### 4.2 Final mapping proof matrix

| # | Final requirement mapping | Direct proof retained by the evidence record |
|---:|---|---|
| M1 | `req.linux-ublk-frontend.linux-prerequisite-probing-is-executable-and-non-destructive` | The live JSON records architecture, kernel, control, ext4, and mount probe facts. The frontend regression set covers unavailable-control and denied-control classification; the record states pre-publication refusals left payload hashes unchanged. |
| M2 | `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` | Two retained `dwv.ublk.trace.v2` live traces contain 458 and 81 translated records and replay cleanly; the guest frontend tests cover trace divergence, stale/duplicate completion, and lifecycle refusal. |
| M3 | `req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe` | Both traces record queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, and zero exhaustion. Guest tests cover pre-admission reservation plus stale/duplicate completion. |
| M4 | `req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow` | Real `/dev/ublkb0` publication completed the one-data/one-parity ext4 workflow. Negative evidence rejects unsupported topology and discard; resource geometry is bounded. |
| M5 | `req.linux-ublk-frontend.assembly-and-shutdown-preserve-ownership-and-recovery-authority` | Both live runs reached `stopped` only after drain, checkpoint, endpoint removal, and trace replay. Second-owner, conflicting-cleanup, stale-readiness, missing-recovery, unknown-cleanup, and owner-death cases fail closed. |
| M6 | `req.linux-ublk-frontend.the-disposable-linux-fixture-remains-independently-inspectable` | After service shutdown, the ordinary data backing file mounted directly as read-only ext4 and exposed the same content; data/parity files remain ordinary independently hashable files. |
| M7 | `req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate` | The record names the source digest, exact ARM64 kernel/environment, commands, trace bounds/digests, ext4 workflow, negative cases, and non-claims. |
| M8 | `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics` | Live ublk requests are retained as normalized trace records and replayed through the portable `demo.disk.trace-replay` contract; malformed and oversized traces are rejected by the linked CLI regression. |
| M9 | `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | Unknown cleanup and owner-process death remain reconciliation-required until explicit owned-endpoint cleanup; successful reacquisition occurs only afterward. Frontend tests cover lifecycle refusal. |
| M10 | `req.normalized-block-semantics.adapters-expose-bounded-deterministic-conformance-behavior` | Both bounded normalized traces replay deterministically and report zero exhaustion; frontend/CLI tests cover clean replay, divergence, malformed input, oversize refusal, and source-preserving replay. |
| M11 | `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | Guest frontend regressions explicitly cover pre-admission reservation and stale/duplicate completion, and owner-death/unknown-cleanup evidence prevents resource reclamation from being mistaken for completion. |
| M12 | `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` | The linked workspace transaction/store regressions exercise submitted watermarks and typed store fence references. The Kani fence harness verifies every store, region, and incarnation; partial multi-store fence coverage is rejected. |
| M13 | `req.store-operation-contracts.store-failures-are-conservative-and-testable` | Resource, topology, ownership, stale-publication, missing-authority, unsupported-operation, and cleanup failures are retained as structured failed-closed outcomes with unchanged pre-publication payload hashes. |
| M14 | `req.dirty-integrity-invalidation.dirty-region-coverage-is-complete-and-checked` | Kani `dirty_region_mapping_covers_every_intersection_once` completed with 0/584 failed checks; the TLA model composes complete data/parity region mutation before clean publication. |
| M15 | `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation` | TLC checks the ordered durable-intent/dirty-state model across crash cuts. The retained before-intent mutant requires `ReconciliationRequired` rather than permitting mutation/clean publication. |
| M16 | `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence` | TLC permits clean checkpoint only after durable intent, complete protected mutation, and all store fences. Kani verifies full store/region/incarnation fence coverage; the partial multi-store fence regression is rejected. |
| M17 | `req.dirty-integrity-invalidation.failures-and-restart-are-conservative` | The model explores crash cuts before intent and after intent, mutation, fence, and checkpoint. Live stale readiness, missing authority, unknown cleanup, and owner death remain failed closed or reconciliation-required. |
| M18 | `req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity` | The real ext4 workload performs create/fsync/overwrite/rename/delete across the normalized service. After shutdown the one-data/one-parity payload hashes are equal, and the independently mounted data file contains the expected content. |
| M19 | `req.healthy-portable-io.durable-completion-and-clean-checkpoint-require-fences` | Both runs stop cleanly only after drain/checkpoint checks; restart preserves the fsynced content. TLC and the fence Kani harness require complete typed fence coverage before clean checkpoint. This does not claim physical power-loss durability. |
| M20 | `req.healthy-portable-io.abandonment-restart-and-failure-preserve-operation-safety` | Owner-process death, stale readiness, unknown cleanup, and missing recovery authority do not silently publish or resume. Explicit cleanup precedes reacquisition; restart then succeeds with bounded traces. |
| M21 | `req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit` | Second-owner acquisition and cleanup against a differently owned endpoint fail explicitly. Owner death releases the process claim but stale endpoint state still requires identity-checked cleanup before reuse. |
| M22 | `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout` | `demo.disk.inspect` emits the stable CLI contract with fixture digest, data/parity hashes, topology epoch, recovery generation, and `Healthy` integrity while SQLite is explicitly classified only as an evaluation adapter, not semantic authority. |
| M23 | `req.recovery-state-semantics.writable-recovery-ownership-is-crash-releasing` | The owner-process-death case proves the OS releases the writable process claim; the next owner must still reconcile and clean the identity-bound endpoint before a successful restart. |

### 4.3 Mapping decision

The pre-retirement row contained 24 requirement IDs. The final row contains 23. The exact set difference is only:

```text
- req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries
```

No final mapping is broader than the combined live/deterministic artifact claim. No artifact inherited a model, checker, or durability claim merely because it belonged to the former correction program. The manifest's existing claim and three non-claim boundaries remain accurate, so no further manifest edit was made.

## 5. Current graph and reviewed state

Fresh `cargo xtask docs knowledge export` and direct inspection of `target/dwv-docs/knowledge/objects.json` / `docs/reviewed-requirements.toml` show:

- capabilities: 25;
- current requirements: 152;
- current relationships: 49;
- `requires`: 41;
- `refines`: 8;
- relationship-participating requirements: 45;
- reviewed-state schema: `dwv.knowledge.reviewed-links.v2`;
- local fingerprints: 152;
- effective fingerprints: 152;
- explicit outcomes: 44, all `reviewed`;
- explicit reasons: 44.

The retired ID has no match under `openspec/specs`, `docs/reviewed-requirements.toml`, `verification/manifest.toml`, `crates`, or `xtask`.

Readiness is clean:

```text
requirements: 152
ready: true
historical-reference: 0
local-fingerprint-suspect: 0
mapping-collision: 0
orphaned-state: 0
semantic-prerequisite-changed: 0
uncovered: 0
unknown-reference: 0
```

## 6. Exact validation observations

| Command | Observed result |
|---|---|
| `cargo test -p xtask` | 32 passed across 3 suites; 0 failed. |
| `cargo xtask docs knowledge export` | success; 25 capabilities, 152 objects, 49 relationships; `dwv.knowledge.objects.v2`. |
| `cargo xtask docs knowledge readiness` | success; ready `true`; all seven gate counts zero; 152 requirements. |
| `cargo xtask docs check` | success; ready `true`; all gate counts zero; change-impact review not required. |
| `cargo xtask docs build` | success; 152 objects; Sphinx HTML emitted under `target/dwv-docs/sphinx/html`. |
| `cargo xtask docs clean-room` | success; `equivalent: true`; objects digest `03312e703f8d5bf98e195dc9c01c88c9680a1295ea09f77c6d08218c4cc77a7d`. |
| `openspec validate --strict --all` | 26 passed, 0 failed; includes `change/canonical-semantic-ownership`. |
| `cargo test --workspace` | 288 passed across 34 suites; 1 ignored; 0 failed. |
| `cargo fmt --all -- --check` | exit 0; no output. |
| `cargo clippy --workspace --all-targets --no-deps -- -D warnings` | exit 0; `OK`. |

## 7. Complete current working-copy diff evidence

Exact `jj diff --summary`:

```text
M xtask/src/knowledge.rs
```

Exact `jj diff --stat`:

```text
xtask/src/knowledge.rs | 152 +++++++++++++++++++++++++++++++++++++++++++++++++++
1 file changed, 152 insertions(+), 0 deletions(-)
```

The current working-copy delta is only the focused follow-up test expansion. The fingerprint implementation, canonical graph, reviewed-state migration, and Linux evidence retirement/remap are already present in the unarchived OpenSpec implementation baseline. The corrected implementation-review report and this follow-up are staged under `/Users/daniel.vicory/tmp/` and therefore are outside the repository diff.

`canonical-semantic-ownership` remains fully implemented and validated but unarchived pending external follow-up approval.
