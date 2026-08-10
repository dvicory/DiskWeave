# Milestone 8 canonical semantic ownership — implementation review

Date: 2026-08-09
Change: `canonical-semantic-ownership`
Review gate: post-implementation, pre-archive
Repository: `/Users/daniel.vicory/src/DiskWeave`

## Requested verdict

Independently inspect the current working copy and return exactly one verdict:

- `APPROVED FOR ARCHIVE`, if the implementation faithfully applies the externally approved semantic model, preserves conservative recovery semantics, has adequate deterministic tests, and leaves no material defect; or
- `REJECTED`, followed by a numbered list of every material defect, ambiguity, missing test, misleading claim, or required correction.

Do not approve from this summary alone. Read the controlling artifacts and implementation. Treat current canonical OpenSpecs as semantic authority; treat the milestone and this report only as review context. Do not require a VCS operation or commit. The working copy contains other milestone work, and the implementation deliberately remains unarchived pending this verdict.

## Controlling artifacts

Read in this order:

1. `openspec/changes/canonical-semantic-ownership/proposal.md`
2. `openspec/changes/canonical-semantic-ownership/design.md`
3. all delta specs under `openspec/changes/canonical-semantic-ownership/specs/**/spec.md`
4. `openspec/changes/canonical-semantic-ownership/tasks.md`
5. `.agents/skills/diskweave-semantic-reconciliation/SKILL.md`
6. relevant current canonical specs under `openspec/specs/**/spec.md`
7. implementation and tests in `xtask/src/knowledge.rs`, `xtask/src/lib.rs`, and `tools/dwv-sphinx.py`
8. reviewed state in `docs/reviewed-requirements.toml`
9. evidence mappings in `verification/manifest.toml`
10. operator/agent documentation in `docs/README.md`, `docs/architecture/derived-documentation-system.md`, `docs/sphinx/reference.md`, and `.agents/skills/diskweave-knowledge/SKILL.md`

The proposal received renewed external semantic approval after three review rounds. The final approved review package is:

- `/Users/daniel.vicory/tmp/milestone8-canonical-semantic-ownership-review-followup-1.md`
- SHA-256 `2a17ea97f54c8ac150f1822072a8c8218c7eb74a9b4cddd86222f7b9e43f60fa`

The final approval explicitly accepted:

- one selected canonical owner per semantic rule;
- the strict `requires` versus `refines` definitions;
- the 49-edge proposal graph;
- conditionally mandatory relationship declarations at consequential boundaries;
- the local/effective fingerprint model;
- the Linux retirement/evidence remap;
- the proposal's metadata-loss successor boundary;
- no implementation before external approval.

## Implemented canonical graph

The current canonical graph contains:

- 25 capabilities;
- 152 current requirements;
- 49 authored relationship edges;
- 41 `requires` edges;
- 8 `refines` edges;
- 45 relationship-participating requirements;
- no unknown targets, self edges, duplicate same-kind edges, mixed-kind source/target pairs, direct cycles, or multi-node cycles.

The current graph was compared against the approved 49-edge proposal graph after application. It matches exactly by source, kind, and target. The 12 approved scenario additions are present. No proposal requirement is missing from the current canonical set; the retired Linux correction-program requirement is absent.

The accepted semantics are:

- `requires`: the source requirement's own semantics depend on an independently owned target rule;
- `refines`: the source owns a genuinely narrower specialization or adapter realization of the target rule;
- composition alone is not refinement;
- at most one relation kind may exist for a source/target pair;
- all authored relationship markers are forward edges; backlinks and capability aggregation are derived;
- combined `requires`/`refines` cycles fail closed.

The eight current `refines` edges and their approved same-policy rationale are:

1. `req.checksum-plane.invalidation-precedes-protected-mutation` → `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`: narrows the shared durable-intent policy to the affected generation-bound `VALID`→`STALE` checksum transition.
2. `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence` → `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`: narrows typed recovery authority to the exact region subset and generation checks that permit dirty clear.
3. `req.explicit-transaction-machine.clean-and-checkpoint-claims-require-fence-evidence` → `req.dirty-integrity-invalidation.checkpoint-and-clear-require-fence-evidence`: realizes the same clear policy as ordered checkpoint/clear/release action emission after terminal children.
4. `req.explicit-transaction-machine.durable-intent-precedes-every-protected-home-mutation` → `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`: narrows the shared protected-mutation policy to when the transaction machine may emit mutation actions.
5. `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` → `req.normalized-block-semantics.ordering-and-durability-intent-cannot-be-silently-weakened`: realizes portable ordering and durability intent in Linux kernel operations and flags.
6. `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` → `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics`: realizes the same portable request fields, validation, and outcomes at the Linux kernel boundary.
7. `req.recovery-state-semantics.home-mutation-requires-durable-dirty-and-integrity-invalidation-intent` → `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation`: narrows durable intent to atomic one-generation recovery-store persistence and commit observation.
8. `req.volatile-media-simulator.operations-use-exact-normalized-ranges-and-structured-evidence` → `req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence`: realizes the portable store range/outcome/evidence contract with simulator-local volatile-media outcomes.

Review the 49 edges individually rather than accepting the counts. In particular, challenge the protected-mutation, fence/checkpoint, abandonment/failure, healthy write, repair/rebuild, Linux mapping/acceptance, and documentation-context clusters for duplicated policy ownership or an incorrect `refines` classification.

## Canonical application and retirement

The approved deltas were applied to the affected current specifications. The application produced:

- 25 existing requirements with changed local semantic fingerprints;
- 1 new documentation relationship-policy requirement;
- 1 retired Linux correction-program requirement;
- 49 current authored edges;
- all approved scenario additions;
- stable semantic IDs for every retained requirement.

The retired identity is:

`req.linux-ublk-frontend.correction-evidence-covers-the-composed-correctness-boundaries`

It has no remaining occurrence in:

- `openspec/specs`;
- `docs/reviewed-requirements.toml`;
- `verification/manifest.toml`;
- `crates`;
- `xtask`.

Its durable evidence was not deleted. Each manifest row was remapped to a current canonical requirement that owns the demonstrated invariant:

- normalized request boundaries and frontend-neutral semantics;
- profile publication/acceptance;
- identity and topology validation;
- operation-slot ownership and backend generation safety;
- bounded frontend abandonment semantics.

Inspect every remapped row and reject any mapping that makes evidence claim broader product, durability, or environment semantics than it demonstrates.

Two non-requirement purpose paragraphs were also corrected after delta application because OpenSpec requirement deltas do not rewrite capability `Purpose` prose:

- `openspec/specs/healthy-portable-io/spec.md` now names current canonical contracts rather than a handoff;
- `openspec/specs/volatile-media-simulator/spec.md` now names the bounded deterministic media model rather than a handoff phase.

One current documentation requirement removed the literal `goal-v5` name while preserving the generic clean-room and historical-isolation contract. That semantic fingerprint change was reviewed individually with a concrete reason.

## Implementation path inventory

The ownership-change implementation surface below is the review target; the exact current working-copy summary is recorded in the external-review follow-up. Unrelated user WIP may exist elsewhere.

Change artifacts:

- `openspec/changes/canonical-semantic-ownership/{proposal.md,design.md,tasks.md,.openspec.yaml}`
- the 12 delta specs under `openspec/changes/canonical-semantic-ownership/specs/**/spec.md`

Current canonical specifications touched by those deltas:

- `openspec/specs/checksum-plane/spec.md`
- `openspec/specs/checksum-scrub-verified-repair/spec.md`
- `openspec/specs/degraded-read-offline-rebuild/spec.md`
- `openspec/specs/dirty-integrity-invalidation/spec.md`
- `openspec/specs/documentation-knowledge-architecture/spec.md`
- `openspec/specs/explicit-transaction-machine/spec.md`
- `openspec/specs/healthy-portable-io/spec.md`
- `openspec/specs/linux-ublk-frontend/spec.md`
- `openspec/specs/macos-bridge-feasibility/spec.md`
- `openspec/specs/normalized-block-semantics/spec.md`
- `openspec/specs/recovery-state-semantics/spec.md`
- `openspec/specs/volatile-media-simulator/spec.md`

Implementation, state, evidence, projection, and operator surfaces:

- `xtask/src/knowledge.rs`
- `xtask/src/lib.rs`
- `tools/dwv-sphinx.py`
- `docs/reviewed-requirements.toml`
- `verification/manifest.toml`
- `docs/sphinx/conf.py`
- `docs/sphinx/reference.md`
- `docs/README.md`
- `docs/architecture/derived-documentation-system.md`
- `.agents/skills/diskweave-knowledge/SKILL.md`

`docs/milestones/m8.md` is the controlling planning context already present in the working copy, not current semantic authority. The 12 delta specs are the exact reviewable canonical-spec application record. The non-spec implementation is reviewable directly at the enumerated paths and through the focused behavioral regressions; no hidden generated source or checked-in relationship registry participates.

## Relationship extraction and validation

`xtask/src/knowledge.rs` now:

1. parses `<!-- dwv:requires <id> -->` and `<!-- dwv:refines <id> -->` only from the contiguous relationship block immediately after the colocated `dwv:req` identity marker;
2. rejects missing or misplaced markers;
3. rejects invalid/non-current targets, self edges, duplicate same-kind edges, mixed-kind pairs, and cycles;
4. emits deterministic diagnostics including source path, line, source ID, kind, target ID, and the smallest deterministic cycle when applicable;
5. sorts forward relationships and all derived backlinks;
6. derives capability-level relationship aggregation and an owner-before-dependent topological reading order;
7. derives everything from current canonical source rather than persisting a relationship registry or backlink table.

Check the parser against malformed spacing, wrong placement, a marker after prose, archived/retired targets, the same pair with two kinds, direct cycles, and larger cycles. Check that diagnostics remain useful rather than collapsing to opaque errors.

## Fingerprints and review lifecycle

The schema cutover is clean:

- knowledge objects: `dwv.knowledge.objects.v2`;
- reviewed state: `dwv.knowledge.reviewed-links.v2`;
- readiness: `dwv.knowledge.readiness.v2`.

Every requirement has:

- a formatting-stable local semantic fingerprint over normalized local normative prose and scenarios; intrinsic IDs and authored relationship markers are excluded;
- an effective semantic/review fingerprint computed prerequisite-first from the local fingerprint plus sorted outgoing `(relationship kind, target ID, target effective fingerprint)` records.

Current reviewed state contains:

- 152 local fingerprints;
- 152 effective fingerprints;
- 44 explicit reviewed outcomes;
- 44 concrete reasons.

The previous single-fingerprint schema was removed; there is no compatibility alias or parallel old state. Removed IDs are dropped. Added or locally changed requirements require direct review. A changed prerequisite makes every transitive dependent stale through its effective fingerprint. Unrelated requirements stay stable. Formatting-only changes remain stable. Review outcomes are recorded per requirement; there is no bulk semantic-acceptance command.

Migration was not automatic approval. The externally approved canonical delta supplied the semantic review basis for each affected requirement. The state records individual reasons. The post-application documentation wording correction was separately resolved with its own reason.

Challenge whether the effective-fingerprint recursion correctly invalidates all and only the transitive dependency closure, including relationship-only changes, and whether conservative staleness remains intact for missing/orphaned state.

Focused regressions prove local prose changes alter both fingerprints and propagate through the transitive dependent closure; relationship addition/removal and target/kind changes preserve the local fingerprint while changing the source effective fingerprint; relationship-marker reordering, formatting-only changes, and reviewed outcome/reason changes preserve both fingerprints.

## Ownership inspection surface

The new bounded command is:

```bash
cargo xtask docs knowledge ownership <requirement-id>
```

It returns typed, deterministic facts:

- selected requirement identity and source;
- local and effective fingerprints;
- outgoing `requires` and `refines`;
- derived `required_by` and `refined_by`;
- capability aggregation;
- owner-before-dependent reading order;
- reviewed outcome and reason;
- current implementation, evidence, documentation, and curriculum links;
- omitted counts under existing repository bounds.

It deliberately does not issue a semantic-consistency verdict and creates no separate semantic database.

The Sphinx projection now exposes relationship fields, local/effective fingerprints, typed relationship links, backlinks, and a generated ownership view. `docs/sphinx/conf.py` defines `requires` and `refines` link types with forward and reverse labels. Agent/operator documentation tells users to call the reconciliation skill when inspection reveals conflicting ownership or shadow architecture.

Inspect the JSON shape, bound behavior, topological order, Sphinx links, and documentation for misleading authority claims.

## Deterministic regression coverage

`xtask/src/knowledge.rs` includes focused tests for:

- conditionally mandatory relationship authorship;
- valid `requires` and `refines` extraction;
- derived backlinks and capability aggregation;
- deterministic owner-before-dependent reading order;
- invalid target IDs and unknown current targets;
- self, duplicate, mixed-kind, direct-cycle, and multi-node-cycle rejection;
- misplaced relationship markers;
- archived, historical, milestone, and retired material exclusion;
- local change propagation across A → B → C while unrelated D remains stable;
- relationship-only transitive invalidation;
- formatting-only stability;
- bounded ownership output;
- current change-boundary recovery of relationship IDs from tracked and untracked Rust files.

The focused xtask result was 30 passing tests across three suites. The full workspace result was 286 passing tests across 34 suites, with 1 existing ignored test.

Reject if a plausible parser, graph, fingerprint, migration, or evidence-remap defect is not defended by a behavior-level test.

## Verification run after final edits

Observed commands and results:

```text
openspec validate --strict --all
  26 passed, 0 failed

cargo test -p xtask
  30 passed, 0 failed

cargo test --workspace
  286 passed across 34 suites, 1 ignored

cargo fmt --all -- --check
  passed, no output

cargo clippy --workspace --all-targets --no-deps -- -D warnings
  passed

cargo xtask docs knowledge export
  25 capabilities, 152 objects, 49 relationships

cargo xtask docs knowledge readiness
  ready=true, 152 requirements, every gate count zero

cargo xtask docs check
  ready=true, no diagnostics, no review-required current change

cargo xtask docs build
  passed, 152 objects, Sphinx HTML generated

cargo xtask docs clean-room
  equivalent=true
  excluded handoffs, archived changes, milestones, generated output, and global Python packages
  objects digest 03312e703f8d5bf98e195dc9c01c88c9680a1295ea09f77c6d08218c4cc77a7d
```

All seven milestone ownership sample commands succeeded. Their outputs showed typed graph relationships and zero omitted implementation/evidence/documentation/curriculum links under current bounds where reported.

The retained Linux identity scan returned no matches. The historical-authority scan found only:

1. the documentation architecture's explicit negative rule that excludes named historical handoffs/architecture revisions from current extraction and context; and
2. historical OS/gate/handoff references in `metadata-loss-recovery`, which the approved design explicitly assigns to the immediately following `metadata-certificate-authority-gate` change because replacing them requires a real executable authorization contract rather than wording substitution.

Review whether this classification matches the approved proposal. Reject if either class can influence current semantics as hidden authority or if the metadata deferral was used to evade a correction required in this change.

## Required semantic challenge

For each affected ownership cluster, answer all of these from the current source and tests:

1. What is the exact rule?
2. Which requirement owns it?
3. Is every dependent merely importing it, or does any dependent restate policy?
4. Does every `refines` edge express a genuine narrower specialization?
5. Do local scenarios test only the local contract?
6. Do failure, uncertainty, restart, and abandonment paths stay conservative?
7. Does implementation/evidence still falsify the same claims?
8. Is historical context needed to interpret current behavior?
9. Would a change to the selected owner invalidate the complete dependent closure and no unrelated requirement?

Pay particular attention to:

- durable dirty/integrity intent before protected mutation;
- store write watermarks versus recovery typed-fence authorization;
- clean/checkpoint publication;
- frontend abandonment versus transaction/recovery/store consequences;
- exact topology/identity validation and Linux stable-slot mapping;
- healthy write composition across normalized request, transaction, recovery, store, dirty, and XOR owners;
- separate-target repair and rebuild evidence;
- documentation context dependency selection and semantic relationship authorship.

## Explicit boundaries and pending work

This review is only for `canonical-semantic-ownership`.

Intentionally not done:

- the change is not archived;
- task 5.3 remains unchecked pending this verdict;
- `metadata-certificate-authority-gate` has not started;
- `normalized-service-request-boundary` has not started;
- `milestone-planning-cutover` has not started;
- no runtime storage, recovery, Linux, macOS, durability, or platform claim was broadened;
- no numeric roadmap identity was consumed;
- no compatibility shim, duplicate relationship registry, manual backlink table, or generalized semantic database was added.

If approved, the next action is to mark task 5.3 complete, archive only `canonical-semantic-ownership`, rerun strict OpenSpec and documentation readiness/build/clean-room checks, record the archive outcome, and immediately begin `metadata-certificate-authority-gate`. If rejected, the change must remain open and every material finding must be corrected and re-reviewed before archive.
