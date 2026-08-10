## Context

All 25 current capability specifications, 152 stable current requirement IDs, and 349 current scenarios were inspected before this change. Current specifications remain semantic authority; the active architecture roadmap was used only for coherence. Detailed mutation, fence, abandonment, and routing predicates are repeated across reusable owners and consumers. Current relationship discovery is untyped, reviewed state hashes only local prose, and seven in-scope capabilities still import authority from retired work identities or historical artifacts.

This design is the external-review checkpoint. No delta is applied, no relationship/fingerprint tooling or reviewed state is changed, no evidence mapping is migrated, and no archive or successor change starts before approval.

## Goals / Non-Goals

**Goals:**

- select one complete owner for every affected detailed operational policy;
- leave refiners, composers, and adapters with only their local responsibility;
- make consequential `requires` and `refines` edges mechanically inspectable;
- invalidate every direct and transitive semantic dependent when effective semantics change;
- preserve stable IDs for semantics-preserving ownership relocation and clarification;
- retain evidence while retiring one completed correction-program requirement.

**Non-Goals:**

- infer semantic equivalence from prose;
- create a registry, manual backlinks, or general graph service;
- merge capability files or weaken constitutional, security, or evidence constraints;
- redesign SQLite or solve the broader lost-acknowledgement adapter problem;
- change metadata-certificate authority or service request APIs;
- broaden Linux support or durability claims.

## Audit coverage and classifications

Ten consequential clusters were found: protected mutation; watermark/fence/checkpoint; abandonment/failure/release; topology/request identity; verification/repair/rebuild; constitutional constraints; historical-authority leakage; Linux correction retirement; Linux acceptance scope; and documentation relationships/fingerprints.

Classification counts are non-exclusive because one current duplicate cluster becomes several valid owner/refiner/composer relationships:

| Classification | Count | Disposition |
|---|---:|---|
| owner + valid refinement | 6 | Preserve owners; narrow recovery, transaction, checksum, dirty-clear, and repair consumers |
| owner + composition | 4 | Healthy admission, write, durable-completion, and failure requirements retain service-only composition |
| owner + adapter conformance | 3 | Linux request mapping, volatile simulator store behavior, and macOS bridge selection remain adapter-local |
| constitutional constraint + detailed owner | 3 | Architecture, security, and evidence constraints remain high-level and do not duplicate operational state machines |
| duplicate ownership | 3 | Protected intent, complete fence predicates, and generic abandonment/failure wording are consolidated |
| contradiction | 0 | The draft lost-acknowledgement contradiction was corrected before this checkpoint |
| missing canonical behavior | 1 | Typed semantic relationships plus transitive effective-fingerprint freshness are added |
| historical-authority leakage | 7 | Checksum, healthy I/O, normalized requests, recovery, simulator, macOS bridge, and Linux correction wording receive exact replacements |
| implementation nonconformance | 0 | No implementation change or new implementation finding belongs to this proposal-only checkpoint |
| shadow architecture | 0 | No current implementation or historical artifact is selected over canonical semantics |
| confirmed no-problem | 17 | Thirteen capability audits need no delta; four cross-capability boundaries are recorded below |

| Canonical capability | Requirements inspected | Audit disposition |
|---|---:|---|
| `architecture-contract` | 6 | Confirmed high-level constitutional owner; no delta |
| `security-boundaries` | 3 | Confirmed security constraint owner; no delta |
| `evidence-boundaries` | 3 | Confirmed evidence constraint owner and relationship target; no delta |
| `normalized-block-semantics` | 5 | Request/lifecycle owner; remove historical gate wording |
| `xor-reference-model` | 4 | Independent algebra owner; target only |
| `store-operation-contracts` | 7 | Exact outcome, watermark, and operation-lifetime owners; targets only |
| `file-backed-stores` | 6 | Backend adapter ownership is distinct; no overlap |
| `volatile-media-simulator` | 9 | Store adapter conformance plus historical cleanup |
| `recovery-state-semantics` | 12 | Atomic recovery refinements and typed recovery authority |
| `dirty-integrity-invalidation` | 7 | Durable-intent, exact-clear, and restart owners |
| `explicit-transaction-machine` | 5 | Transaction action/order/state refinements |
| `transaction-engine-evidence-comparison` | 4 | Evidence comparison, not runtime semantic authority; no delta |
| `healthy-portable-io` | 6 | Service composition only; clarify stable-slot admission |
| `anchorless-topology-identity` | 7 | Topology identity/validation owner; target only |
| `parity-envelope-profiles` | 5 | Envelope evidence is not recovery authorization; no delta |
| `checksum-plane` | 6 | Generation-bound checksum validity owner and invalidation refiner |
| `parity-verification-repair` | 5 | Read-only equation scan and mismatch-classification owner; target only |
| `checksum-scrub-verified-repair` | 5 | Unique independent-evidence repair authority and repair-local durability mapping |
| `degraded-read-offline-rebuild` | 7 | Known-erasure eligibility and separate-target rebuild owner |
| `metadata-loss-recovery` | 5 | Distinct metadata-loss decision boundary; its authority cleanup is deliberately deferred |
| `normalized-trace-replay` | 4 | Trace normalization/replay evidence is distinct; no delta |
| `macos-bridge-feasibility` | 6 | Bounded adapter-selection evidence; remove historical goal identity |
| `macos-demo-cli` | 6 | Operator workflow, not a duplicate semantic owner; no delta |
| `linux-ublk-frontend` | 8 | Linux adapter and acceptance owner; retire completed correction-program requirement |
| `documentation-knowledge-architecture` | 11 | Add typed relationship/effective-fingerprint behavior |

Confirmed cross-capability non-problems:

1. `architecture-contract`, `security-boundaries`, and `evidence-boundaries` constrain detailed capabilities without owning their state transitions.
2. Parity verification, checksum-authorized repair, degraded rebuild, and metadata-loss recovery consume related evidence but make distinct decisions.
3. Shared tests and traces may verify multiple independent requirements without creating semantic co-ownership.
4. Parity-envelope session evidence and dirty recovery state are related layers; the envelope cannot authorize recovery state.

## Final ownership reconciliation

| Semantic policy | Current requirements | Selected owner | Refiners | Composers | Adapters | Scenario disposition | Semantic classification |
|---|---|---|---|---|---|---|---|
| Protected home mutation requires durable dirty and integrity intent | dirty intent; recovery home mutation; transaction intent; checksum invalidation; healthy write | `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation` | recovery one-generation persistence; transaction action emission; checksum `VALID`→`STALE`; repair-local persistence | healthy protected write | none | Keep owner success/failure; split recovery reject/mismatch from lost/corrupt/indeterminate; preserve transaction, checksum, repair, RMW, and full-overwrite local cases | duplicate ownership → owner + valid refinements/composition |
| Exact range outcome and persistence evidence | store outcomes; simulator outcomes; healthy writes | `req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence` | simulator exact-range adapter behavior; healthy write mapping | healthy write | volatile simulator | Preserve short/uncertain outcomes and exact store-write composition | owner + adapter conformance/composition |
| Raw store watermark truth | store watermark; recovery fence; dirty clear; transaction checkpoint; healthy completion | `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` | recovery admissibility, dirty clear, transaction release, healthy completion | healthy durable completion | none | Remove repeated complete fence predicates; preserve local acceptance/rejection cases | duplicate ownership → owner + valid refinements/composition |
| Typed authority for clean/valid/session claims | recovery typed fence; dirty clear; transaction checkpoint; healthy completion; repair acceptance | `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence` | dirty exact-region clear; transaction action/release; repair-local acceptance | healthy durable completion | none | Preserve valid and invalid evidence cases with owner/local boundaries explicit | owner + valid refinement/composition |
| Exact dirty-region clear subset and generation checks | dirty clear; transaction checkpoint; healthy completion | `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence` | transaction release mapping | healthy durable completion | none | Preserve covering and omitted/stale evidence cases under dirty owner | owner + valid refinement/composition |
| Frontend abandonment and delivery-interest semantics | normalized lifecycle; dirty failure; transaction failure; healthy failure; Linux resources | `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | none | dirty durable consequence; transaction state; healthy failure | Linux frontend mapping remains local | Preserve abandonment before/after irreversible work and reconciliation cases | duplicate ownership → owner + composition |
| Backend operation/tag/buffer lifetime and generation reuse | store slots; transaction failure; healthy admission/failure; Linux tags | `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | healthy admission/failure composition | transaction failure and healthy service | Linux request/tag handling | Preserve exhaustion, stale/duplicate completion, abandonment, and delayed release | owner + composition/adapter conformance |
| Transaction action order, terminal state, and release points | transaction actions; transaction intent/checkpoint/failure; healthy write/failure | `req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions` plus its three local state requirements | transaction durable-intent, checkpoint, and failure requirements refine imported owner policies | healthy write/failure | none | Rename generic owner scenarios to transaction-local accepted/unavailable/authority/reconciliation outcomes | owner + valid refinement |
| Durable dirty/restart consequence | dirty failure/restart; transaction failure; healthy failure; Linux restart | `req.dirty-integrity-invalidation.failures-and-restart-are-conservative` | none | transaction and healthy failure handling | Linux reacquisition | Preserve home-write failure; rename daemon crash to process loss; never infer clean from missing completion | owner + composition |
| Slot, role, coding position, assignment, generation, and epoch identity | topology validation; normalized request target; healthy admission; Linux mapping; degraded eligibility | `anchorless-topology-identity` for topology meaning; normalized request requirement for request-carried fields | healthy admission mapping; Linux request mapping | healthy admission | Linux request adapter | Preserve healthy/stale assembly and add reordered collection plus positional-mismatch cases | clarification, owner + composition/adapter conformance |
| Normalized request validation, ordering, and durability intent | normalized request and ordering; healthy admission; Linux mapping | the two `normalized-block-semantics` request/ordering requirements | healthy admission and Linux mapping | healthy service | Linux frontend | Preserve supported/unsupported/range/mapping/replay cases; remove historical gate language | owner + composition/adapter conformance |
| XOR parity update and known-erasure reconstruction | XOR update/reconstruction; healthy write; degraded read | corresponding `xor-reference-model` requirements | none | healthy write and degraded read consume exact math | none | Preserve RMW/full overwrite, single-erasure, and beyond-tolerance cases | confirmed no-problem |
| Read-only mismatch classification and automatic repair authority | parity mismatch classification; checksum validity; checksum automatic repair | parity verifier owns read-only classification; checksum scrub owns unique independent-evidence repair authorization | none | repair plan combines independent facts | none | Preserve unique parity/data target and ambiguous/beyond-tolerance cases | confirmed distinct owners |
| Separate-target repair durability and verified readback | checksum repair; dirty intent; recovery typed fence | `req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback` for repair-local action; imported owners retain durability authority | repair requirement refines dirty intent and recovery fence policy | none | none | Preserve success and target-write/readback failure cases | owner + valid refinement |
| Degraded read eligibility and offline rebuild/promotion | degraded eligibility/rebuild; topology; recovery export/cursor; XOR | the two `degraded-read-offline-rebuild` requirements | none | degraded requirements consume independent prerequisites | none | Preserve known erasure, stale evidence, ambiguity, chunk, unknown durability, and alias refusal | confirmed distinct owner + requirements |
| Metadata-loss recovery and envelope evidence | metadata-loss matrix; parity envelope; verifier; repair; recovery export | `metadata-loss-recovery` owns decisions; `parity-envelope-profiles` owns non-authorizing envelope facts | none in this change | none | none | Preserve all cases; certificate-authority repair and historical wording are deferred to the dedicated successor change | confirmed no-problem |
| Constitutional product, security, and evidence boundaries | architecture, security, evidence requirements | each capability owns its own high-level constraint | none | detailed capabilities remain constrained | none | No scenario movement | constitutional constraint + detailed owner |
| Canonical relationship graph and semantic freshness | canonical discovery; change impact; context | new `req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived` | none | change-impact and context commands require it | tooling implementation waits for review | Add valid/invalid graph, A→B→C propagation, and formatting-stability cases | missing canonical behavior |
| Platform adapter conformance and evidence scope | normalized adapter contract; simulator; macOS bridge; Linux mapping/acceptance; evidence scope | portable semantics remain with normalized/store owners; each platform requirement owns only translation/evidence | simulator, healthy, and Linux mapping use typed `refines` where they specialize an owner | none | simulator, macOS candidate, Linux frontend | Preserve platform refusal/non-claim cases; separate Linux declared profile from exact recorded environment | owner + adapter conformance |
| Linux correction evidence program | correction-program requirement plus durable requirements already mapped by its manifest entry | no product owner; the program is complete | none | none | none | Delete its two program-completion scenarios with the requirement; retain artifacts under the exact durable targets in the removal migration | obsolete requirement retirement |

## Scenario disposition

Every scenario in a modified, added, or removed requirement has one of these explicit dispositions:

| Requirement | Disposition |
|---|---|
| Linux kernel request mapping | Preserve all five current scenarios. Keep read/write/flush and overflow unchanged; generalize unsupported-intent wording without weakening it; preserve unrepresentable completion and retained-trace replay as distinct adapter conformance cases. |
| Linux live acceptance | Replace “complete VM workflow passes” with “declared supported profile passes”; move exact guest/kernel/queue/trace/source facts to evidence; preserve unavailable-environment and stronger-claim refusal. |
| Linux correction evidence | Delete “one proof layer is unavailable” and “evidence records the acceptance boundary” only because the completed program requirement is retired; the first remains covered by evidence fail-closed/bounded-artifact owners and the second by scope-accurate Linux acceptance plus direct mappings. |
| New canonical relationship requirement | Add valid owner/refiner extraction, invalid graph, A→B→C propagation with unrelated D, and formatting-only stability. |
| Documentation change impact | Replace generic relationship-change wording with dependency-closure semantic-prerequisite invalidation; preserve unavailable-baseline failure disclosure. |
| Documentation context | Preserve current-selection and context-bound failure scenarios while adding typed relationships, effective fingerprints, and derived reading order to their results. |
| Recovery home mutation | Preserve valid extent mutation. Replace one generic commit-failure scenario with separate rejected/generation-mismatched-before-commit and lost/corrupt/indeterminate-observation scenarios. |
| Recovery typed fence | Rename both scenarios to distinguish owner admissibility from raw fences; preserve successful clear and conservative volatile-evidence rejection. |
| Recovery SQLite decision | Preserve candidate comparison; remove `OS-005` authority and keep selection evidence-driven. |
| Recovery evaluation fixtures | Preserve rejected/uncertain commit and missing/corrupt reset cases; remove `OS-005` authority and retain conservative reconciliation. |
| Degraded-read eligibility | Preserve clean known erasure, stale-generation evidence, and ambiguity/beyond-tolerance scenarios unchanged. |
| Offline rebuild | Preserve chunk completion, unknown replacement durability, and source alias refusal unchanged. |
| Automatic checksum repair | Preserve unique parity, unique data target, and beyond-tolerance refusal unchanged. |
| Separate-target repair | Preserve successful verified repair and write/readback failure unchanged. |
| macOS bridge outcome | Preserve candidate-pass and no-candidate-pass behavior; remove `OS-021`/“live goal” authority. |
| Simulator store behavior | Preserve short write; rename uncertain completion to uncertain effect while retaining retry evidence. |
| Simulator safety invariants | Preserve power-loss replay; remove future-OpenSpec and `OS-004` ownership claims. |
| Portable evidence scope | Rename the two cases to portable-only evidence and later platform adapter; preserve the no-certification boundary and remove `OS-001`/successor-gate authority. |
| Checksum invalidation | Preserve valid-extent and parity-clean/stale-record cases; narrow the first to the checksum-local `VALID`→`STALE` transition through the durable-intent owner. |
| Healthy assembly/admission | Preserve healthy assembly and ambiguous/stale refusal; add reordered collection/assignment resolution and positional mismatch refusal. |
| Healthy write | Rename RMW explicitly; preserve full-overwrite behavior; both remain service-local composition cases. |
| Healthy durable completion | Rename the two fence cases around owner acceptance/rejection; preserve durable success and conservative failure. |
| Healthy abandonment/restart | Preserve abandonment-after-intent and restart-with-dirty-state as service composition cases. |
| Transaction durable intent | Rename success/failure to accepted/unavailable durable invalidation; preserve no-home-mutation and reconciliation outcomes. |
| Transaction checkpoint | Rename covering/missing fence cases around recovery authority acceptance/rejection; preserve action-order and release semantics. |
| Transaction failure | Rename frontend abandonment and daemon crash to delivery-interest abandonment and process loss; preserve transaction-state consequences. |
| Dirty checkpoint/clear | Rename complete/missing fence cases around exact selected-region authority; preserve subset/generation checks. |
| Dirty failure/restart | Preserve post-intent home-write failure; rename daemon crash to process loss before checkpoint; preserve durable dirty discovery. |

## Historical-authority replacements

| Current leakage | Exact proposed replacement |
|---|---|
| checksum `OS-010` intent | `dwv:refines req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation` plus checksum-local `VALID`→`STALE` prose |
| healthy purpose “required by the handoff” | direct portable owners and service-composition wording |
| healthy `OS-008/OS-010` ordering | typed dirty, recovery, transaction, XOR, and store relationships plus direct orchestration prose |
| normalized `OS-001` and “successor gate” | durable portable-evidence-does-not-certify-platform behavior |
| recovery `OS-005` selection/fixtures | current SQLite-decision and evaluation-fixture requirements stated directly |
| simulator “handoff Phase 0”, `OS-002`, future OpenSpec, and `OS-004` | standalone media model, store-operation refinement, and explicit owner-capability boundaries |
| macOS “live goal” and `OS-021` | evidence-backed macOS functional reference and current normalized frontend seam |
| Linux architecture-v0.8 correction mapping | retire the program requirement; map retained artifacts directly to the 18 current target IDs in its delta migration |

Documentation history-isolation scenarios intentionally mention handoffs and old goals as excluded test inputs; they do not import authority. Metadata-loss historical references are outside this delta and remain assigned to `metadata-certificate-authority-gate`.

## Relationship syntax and extraction

Accept zero or more contiguous markers immediately after an intrinsic marker:

```text
<!-- dwv:req req.capability.local-policy -->
<!-- dwv:requires req.other.prerequisite -->
<!-- dwv:refines req.owner.complete-policy -->
```

`requires` means the local requirement depends on another independently owned semantic fact. `refines` means the target owns the underlying detailed operational policy and the local requirement defines that policy's narrower local specialization, composition mapping, or adapter conformance. One source-target pair cannot use both kinds.

The canonical extractor records each edge with source path and line. A relation marker outside the contiguous marker block is invalid. The combined graph fails on unknown or non-current targets, self edges, duplicate edges, mixed-kind duplicates, and cycles. Diagnostics include source path, line, source ID, kind, target, and the smallest deterministic cycle.

Only forward edges persist. `required_by`, `refined_by`, capability aggregation, and owner-before-dependent reading order are derived in stable order under existing bounds.

The 48 proposed markers comprise 23 `requires` and 25 `refines` edges. The combined graph is acyclic. Relationship kinds were reviewed under the exact rule above: Linux request mapping refines normalized request/ordering semantics and separately requires operation-slot lifetime; no source-target pair uses both kinds.

Deliberately implicit edges:

- universal architecture/security/evidence constraints are not repeated on every detailed requirement;
- metadata-loss decision edges wait for its dedicated authority change to avoid overlapping semantics;
- reverse edges are derived, never authored;
- implementation call graphs, co-testing, lexical similarity, and optional future work are not semantic edges;
- offline rebuild relies transitively on topology/XOR through degraded eligibility instead of duplicating those edges;
- healthy failure relies on durable dirty consequences rather than redundantly linking every recovery-state observation.

## Fingerprints and dependency-closure semantic re-review

**Local semantic fingerprint:** digest of normalized local requirement prose and scenarios, excluding intrinsic and relationship comments and formatting-only changes.

**Effective semantic/review fingerprint:** after validating the combined semantic graph is acyclic, compute prerequisite-first a deterministic digest of the local semantic fingerprint, sorted outgoing `(relation kind, target ID)` pairs, and each target's effective semantic fingerprint.

Reviewed state stores and compares the effective fingerprint under a bumped schema. Each stale requirement requires an individual review and concrete reason; bulk acceptance remains forbidden. Changing only a review outcome/reason does not affect either fingerprint.

For `A → B → C` where B imports A and C imports B:

- changing A's local semantics changes A's effective fingerprint, then B's, then C's;
- formatting-only A changes preserve A, B, and C;
- unrelated D remains stable;
- changing B's edge set changes B and C, but not unrelated A or D unless the new edge itself imports them.

`knowledge affected`, readiness, and `docs check` report `semantic-prerequisite-changed` for every direct and transitive stale dependent.

## Ownership output

`cargo xtask docs knowledge ownership <requirement-id>` will return the local requirement, typed forward edges, derived backlinks, local/effective fingerprints, reviewed result/reason, existing implementation/evidence links, and bounded owner-before-dependent reading order. It reports facts only, never a semantic-coherence verdict.

## Requirement identity disposition

Preserved with ownership relocation/narrowing:

- dirty durable intent, checkpoint/clear, and failure/restart IDs;
- recovery home-mutation and typed-fence IDs;
- explicit transaction durable-intent, checkpoint, and failure IDs;
- checksum invalidation ID;
- healthy write, durable-completion, and abandonment IDs;
- separate-target repair ID.

Preserved with clarification or relationship-only change:

- healthy assembly/admission;
- Linux kernel request mapping and live acceptance;
- normalized portable-evidence boundary;
- simulator operation/safety requirements;
- recovery SQLite/evaluation requirements;
- macOS bridge outcome;
- checksum automatic repair;
- degraded-read eligibility and rebuild;
- documentation change-impact and context IDs.

New: `req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived`, because typed canonical relationships and transitive effective-semantic freshness are new durable behavior.

Retired: `req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries`, because it describes a completed correction/evidence program rather than product behavior. Its exact 18 durable evidence targets are listed in the Linux removal delta. No other ID changes.

## Risks / Trade-offs

- Incorrect owner selection could hide a behavior change. Mitigation: stable IDs are retained only for preserved semantics, every local scenario has a disposition, and every cluster is challenged against independent implementations and failure paths.
- Transitive invalidation can create review churn. This is required because imported semantics are part of each dependent's effective semantics; the graph stays sparse rather than suppressing invalidation.
- Relationship comments are Markdown-local syntax. Strict placement and deterministic diagnostics prevent silent attachment without adding a registry.
- Retiring the Linux correction ID breaks mappings. Application remains blocked until reviewed state, manifest, curriculum, Rust, maintained documentation, and relationship references are removed or mapped to the exact targets.
- Lost/corrupt/indeterminate recovery commit observation remains conservative: neither prior nor proposed resulting snapshot is authoritative for protected mutation; no home mutation is permitted until reconciliation.

## External review gate

This proposal is ready for external semantic review only. Applying deltas, implementing tooling, changing reviewed fingerprints, migrating evidence, archiving, or starting successor changes remains blocked until approval.
