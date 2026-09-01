## Context

See `proposal.md` for motivation. The synchronized coded-CLEAN requirement is the intended current contract. Round-5 external review found implementation nonconformance after the archived predecessor: public low-level APIs can bypass named owners, and the recovery store can commit a capture successor without the coupled semantic effects that make it safe.

The current normal service usually assembles complete transactions. That caller discipline is insufficient because correctness must hold at the durable owner boundary. The repair must preserve good scope geometry, persistence aggregation, retained-history quarantine, and recovery uncertainty behavior.

## Goals / Non-Goals

**Goals:**

- Make complete coded-CLEAN semantic successors indivisible and store-validated before durability.
- Make geometry, lifecycle, release, and cleanup authority non-bypassable in ordinary production code.
- Make accepted Connect correspondence use production owners and complete production transaction paths.
- Preserve finite-state boundedness while implementing nonterminal capture-capacity backpressure and eventual progress.
- Restore an honest external completion gate without rewriting the archived predecessor or current canonical requirement.

**Non-Goals:**

- A general effect system, transaction framework, persistent operation ledger, or compatibility shim.
- New coded-CLEAN policy beyond the narrow pre-commit bundle clarification in the delta.
- Startup publication, first-write currentization, shutdown, rollover, transport, or physical durability certification.
- Reworking the delegated Quint relation for recovery-store mechanics; those mechanics remain external owner behavior.

## Decisions

### Complete semantic transitions are opaque recovery inputs

Correctness-sensitive coded-CLEAN preparations will expose one opaque transition value representing the complete durable successor, not separable public capture updates/removals plus caller-assembled side effects. The recovery transaction accepts that value as one semantic transition group. The store validates its exact durable predecessor, topology/recovery generation, phase, proposed capture successor/removal, aggregate evidence, and every owner-required selected-state effect before any member becomes durable.

The minimum groups are:

- `CLEAN`: aggregate persistence evidence, the exact capture successor, and every required dirty-region, checksum-extent, and integrity-state mutation for that owner-approved successor;
- later cut: the newer dirty/write-recovery boundary and the exact capture successor recording that cut;
- refused cleanup: exact removal plus complete conservative dirty/stale state, unless the group carries an opaque current owner-issued proof bound to the exact capture/predecessor, topology/recovery generation, selected dirty/checksum geometry, and durable conservative state relied upon;
- inherited non-closable `CleanKnown` abandonment: exact removal plus every owner-required selected-state effect, with omission permitted only under the same exact opaque conservative-state proof;
- live empty `CleanKnown` cleanup: exact removal plus clean-closure evidence, with no invented invalidation.

Any incomplete, duplicate, stale, mixed-predecessor, unrecognized, or extra correctness-sensitive member within one semantic transition group rejects the transaction before durable state changes. A transaction containing any coded semantic group contains no ordinary recovery mutation. Multiple coded groups may share a transaction only when they name distinct captures and require compatible postconditions for every overlapping dirty region and checksum/integrity extent. The recovery store validates this composition before applying any mutation. This narrow rule closes before/after raw-mutation and conflicting-group holes without a general transaction effect framework.

The existing transaction vector remains sufficient: add one pre-commit coded-composition validation pass, then retain candidate staging and the transition-specific validators. Production already represents each coded successor as an opaque group; callers that need unrelated raw recovery work use a separate generation-checked transaction.

`CLEAN` and later-cut use the same exact predecessor-bound prepare → commit → install and unknown-acknowledgement semantics as release, compaction, and cleanup. Each group produces next live authority only after the complete group definitely commits or authoritative reopen reconciliation proves the exact successor. A may-have-committed `CLEAN` or later-cut installs neither process-local candidate.

### Owner provenance replaces public low-level authorization

Production coded admission consumes an opaque complete claim issued by the canonical geometry owner and bound to the exact admitted mutation: member identity, byte range, request/operation identity, operation generation where applicable, topology epoch, and coding profile. A claim for another mutation under the same topology/profile is stale or cross-request authority and cannot be replayed. Arbitrary mapped unit sets remain only in isolated component tests, not as an ordinary production admission input.

Final coded release consumes the canonical lifecycle owner's opaque full-lifecycle authorization. `OperationReleasePermit` remains an internal prerequisite and cannot call a public coded-removal path by itself.

Included lifecycle evidence is emitted only after the production lifecycle owner validates live finalization or a persisted release receipt. The current coordinator may validate evidence binding, but possession of the coordinator or fence certificate must not let a coded-CLEAN consumer mint positive evidence. Remove public issuer-returning constructors and direct positive issuance paths rather than adding another wrapper around them.

These are clean cutovers. Migrate every caller and remove obsolete bypass APIs; do not leave deprecated aliases.

### Connect uses production composition for accepted paths

Accepted Connect paths use the same production geometry owner, lifecycle owner, final release path, and complete recovery semantic transition as `HealthyPortableService`. The bridge may choose model inputs and project resulting production state, but it does not create authority or omit effects for convenience.

Negative probes may inject corrupted observations through explicit test-only fault seams. They must remain incapable of issuing production authority. Manifest and evidence claims name only behavior exercised by a runnable maintained command.

### Capture exhaustion is retained nonterminal work

When `max_coded_captures` is exhausted, the affected write remains retained and returns a nonterminal wait/backpressure outcome. Capacity exhaustion alone does not transition an otherwise healthy service to `Recovering`, discard unresolved evidence, duplicate admission, or turn the retained operation into a terminal/filesystem-visible failure. After an existing capture durably resolves and retires, driving the retained write resumes from its existing admission state without duplicate admission or authority.

This uses the existing retained-work/contention mechanism. Do not add a scheduler abstraction solely for this case.

### History is repaired forward

The archived `2026-08-29-repair-coded-clean-lifecycle-authority` change and synchronized canonical requirement remain intact. This successor records that its local-review completion claim was invalidated by external Round-5 `NO-GO`.

The successor may reach implemented-and-verified state, but task completion deliberately stops before canonical sync/archive and Bead closure. A fresh exact-snapshot external `GO` is a required input to those later actions.

## Risks / Trade-offs

- **Typed bundle breadth:** One opaque group must carry enough information for exact pre-commit validation. Keep groups specific to coded-CLEAN transitions; a generic extensible effect framework would obscure ownership.
- **Cross-crate authority:** Existing type placement may make producer privacy awkward. Prefer moving the smallest opaque capability to the lowest shared owner crate or retaining it inside an owner object; do not expose a public constructor merely to cross a crate boundary.
- **Recovery compatibility:** Persistent capture snapshots should not need reinterpretation. If a transaction or snapshot representation changes, readers must reject unsupported partial state rather than infer missing effects.
- **Connect coupling:** Reusing production paths can make drivers less convenient. That cost is intentional: a convenient bypass would invalidate correspondence evidence.
- **Capacity liveness:** Retained writes need deterministic resume evidence. The repair promises no fairness beyond progress once the blocking capture is durably retired and the retained work is driven again.
