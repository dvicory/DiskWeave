# Prompt: create and execute DiskWeave autonomous implementation goal v3

You are continuing an existing DiskWeave implementation. The repository may contain completed, archived, in-progress, partially implemented, or superseded OpenSpecs. Do not assume the project is starting from scratch, and do not infer implementation state from architecture documents alone.

## Inputs and authority

Read, in this order:

1. `diskweave-refined-architecture-v0.7.md` — the current normative architecture.
2. `diskweave-refined-architecture-v0.6.md` — the previous baseline and historical rationale.
3. The current autonomous implementation goal, including goal v2 if that is still active.
4. The repository implementation, OpenSpecs, ADRs, archived changes, verification records, tests, commits, and `dwv demo` behavior.

Use this precedence when sources disagree:

```text
accepted v0.7 architecture invariants and decisions
    > newer accepted ADRs that explicitly amend v0.7
    > executable repository evidence
    > current/in-progress OpenSpecs and goal text
    > v0.6 historical guidance
    > assumptions or comments not backed by evidence
```

Revision 0.7 is not merely advisory and completed work is not grandfathered. Revision 0.6 explains the contract under which earlier work may have begun, but it does not waive v0.7 conformance.

## Mission

Create an evidence-based **DiskWeave autonomous implementation goal v3** that:

- reconciles all existing work with architecture revision 0.7;
- revisits completed or in-progress work where v0.7 exposes a correctness, safety, verification, lifecycle, or unsupported-claim gap;
- preserves valid code and evidence rather than restarting the project;
- continues the dependency-ready portable implementation program;
- continuously exercises real functionality through the existing `dwv demo` CLI;
- closes applicable v0.7 conformance gaps instead of recording them indefinitely as future work;
- adds later verification mechanisms only where their assurance justifies their complexity.

Do not stop after writing goal v3. Once the goal and reconciliation artifacts are coherent, continue executing the next dependency-ready item.

# 1. Inspect before deciding

Inspect the repository before claiming that a seam, invariant, test, model, crate, command, or defect is present or absent.

For every relevant existing or planned milestone, classify its actual state as:

- complete and evidenced;
- behavior complete but evidence incomplete;
- partially implemented;
- in progress;
- blocked by a real dependency or platform gate;
- not started;
- superseded by a later accepted decision;
- non-conformant with v0.7;
- unknown pending inspection.

At minimum inspect OS-007, OS-009, OS-017, OS-024, their prerequisites, their archived evidence if any, and every component they exercise.

Do not trust a milestone label, archived directory, commit message, or passing unit test by itself. Compare the claimed behavior with code paths, deterministic tests, CLI evidence, persistent-state behavior, and the applicable architecture invariants.

# 2. Perform a real v0.7 conformance audit

Build a conformance matrix covering every material v0.7 addition or clarification, including:

- ownership of consequential async work through a terminal outcome;
- frontend abandonment versus underlying operation completion;
- operation-slot, buffer, token, permit, queue, and shutdown/drain lifecycles;
- stale-token/generation rejection;
- durable transfer of unfinished work to recovery state;
- explicit treatment of uncertain completion;
- deterministic authority for time, IDs, fault choices, schedules, and trace emission where replay requires it;
- `dwv-sim` as the domain-specific durability and recovery oracle;
- generated/fuzzed operation, fault, crash, and topology schedules;
- normalized semantic trace/reproducer artifacts;
- bounded verification of high-consequence pure components;
- abstract protocol/model checking, including an evidence-based TLA+/PlusCal/TLC versus Stateright decision;
- broad concurrency schedule exploration versus tiny exhaustive synchronization models;
- later production-adjacent deterministic filesystem/`io_uring` simulation;
- capability-oriented APIs where ambient access would obstruct reasoning or deterministic tests;
- Asupersync concepts only, with no dependency, copying, execution, testing, or benchmarking unless its licensing receives explicit legal clearance.

For each item assign one of these outcomes:

| Outcome | Meaning | Required action |
|---|---|---|
| **CONFORMANT** | Current implementation and evidence satisfy v0.7 | Link the evidence; do not reimplement |
| **EVIDENCE GAP** | Behavior appears conformant, but a current claim lacks adequate executable evidence | Add focused evidence, preferably inside the current coherent OpenSpec or a small verification successor |
| **NON-CONFORMANT** | Behavior violates or cannot satisfy a v0.7 invariant | Remediate before depending on the unsafe behavior |
| **UNKNOWN** | Inspection cannot yet determine conformance | Add the smallest probe/test needed to resolve it |
| **NOT YET APPLICABLE** | The relevant subsystem does not yet exist or no current claim depends on it | Add a dependency-gated future item, not placeholder production abstractions |
| **PLATFORM-DEFERRED** | Evidence genuinely requires Linux, a live macOS bridge, or hardware | Preserve the claim boundary and gate it to the appropriate later milestone |

Do not use **NOT YET APPLICABLE** or **PLATFORM-DEFERRED** to defer a portable correctness defect in code that already exists.

# 3. Revisit completed work when needed

Completed or archived work is historical evidence, not a conformance exemption.

Revisit it when any of the following is true:

- it violates an accepted v0.7 correctness or safety invariant;
- it makes a stronger durability, recovery, integrity, or verification claim than its evidence supports;
- cancellation, abandonment, shutdown, or resource reuse can orphan consequential work;
- a clean, verified, repaired, or reconstructed state can be reached without the evidence v0.7 requires;
- deterministic replay cannot preserve a known critical failure;
- a persistent format or schema has accidentally frozen an implementation detail v0.7 requires to remain replaceable;
- the existing OpenSpec acceptance criteria would permit a forbidden outcome under v0.7;
- new executable evidence reveals a real defect.

When revisiting work:

1. Preserve the original OpenSpec, commit, and verification history. Do not rewrite history as though the earlier decision never existed.
2. Retain or create a minimized deterministic reproducer before fixing the defect whenever practicable.
3. Add a corrective/successor OpenSpec, amendment, or repository-native reopening mechanism that explicitly references the prior milestone.
4. Mark the earlier claim as superseded, qualified, or requiring remediation in the current status index.
5. Fix the smallest coherent layer that restores the invariant.
6. Re-run focused tests and the relevant `dwv demo` workflow.
7. Record the new evidence and any remaining claim boundary.

A broad refactor is justified only when the existing seam cannot satisfy the required property without unsoundness or duplicated correctness logic. Do not refactor merely because a verifier or testing library prefers another shape.

# 4. Distinguish property requirements from tool choices

The architecture requires properties and evidence, not loyalty to a particular library.

A named tool may be used when it is currently the best-supported mechanism, but an equivalent mechanism is acceptable if it proves the same property and records the same bounds, assumptions, counterexamples, and non-claims.

Do not introduce a general effect system, alternate async runtime, actor architecture, or testing-framework-shaped domain model solely to use Turmoil, Shuttle, Loom, Stateright, TLA+, Kani, or another verifier.

Preserve ordinary Rust and existing DiskWeave semantic seams unless inspection reveals a concrete correctness reason to change them.

# 5. Update the current goal into goal v3

Rewrite the current autonomous goal into **goal v3** based on repository evidence. Do not merely append a v0.7 section to goal v2.

Goal v3 must contain four clearly separated queues:

## A. Immediate conformance blockers

Correctness, integrity, lifecycle, recovery, or unsupported-claim defects in existing behavior. These block dependent work and must be repaired even if the responsible milestone was previously archived.

## B. Coherent in-progress and previously planned work

Preserve useful current work, including OS-007, OS-009, OS-017, and OS-024 where they remain unfinished or need small v0.7 amendments. Do not restart them solely because v0.7 names additional verification layers.

## C. Additive verification evidence

Verification work that strengthens already-correct seams without blocking unrelated portable delivery. Examples include bounded arithmetic proofs, an independent abstract recovery model, concurrency schedule exploration, production-adjacent deterministic I/O simulation, and sustained fuzz corpora.

Move one of these earlier only when it resolves a concrete high-consequence uncertainty in existing behavior.

## D. Capability-gated platform evidence

macOS bridge, Linux ublk/io_uring/filesystem, and bare-metal durability work remain behind their real entry gates. Their absence must constrain claims but must not block portable work.

Goal v3 must state dependencies and completion evidence for each queue item. A milestone is not complete merely because code compiles, a library test passes, or an OpenSpec directory is archived.

# 6. Keep `dwv demo` as the continuous acceptance spine

The existing `dwv demo` CLI is not only a final demonstration. It is the continuous operator-facing integration and acceptance surface throughout goal v3.

Do not create a replacement `v1`, `v2`, or differently named CLI generation. Preserve valid commands and semantics; add only the smallest compatible commands/options needed to expose implemented behavior.

## 6.1 Capture a baseline before remediation

Before broad changes, run and record every currently working relevant `dwv demo` path. Capture:

- exact commands;
- exit statuses;
- machine-readable output where available;
- final payload, parity, integrity, topology, and recovery state;
- produced traces or fixture identifiers;
- known limitations and blocked stages.

A baseline failure is evidence. Diagnose it rather than redefining the expected result to make it pass.

## 6.2 Exercise the CLI after each relevant change

For every OpenSpec or corrective change that affects portable behavior, run:

1. the smallest focused `dwv demo` scenario proving that change;
2. the unaffected neighboring scenarios needed to detect regression;
3. the full integrated portable workflow whenever all of its prerequisites are available.

The intended integrated workflow remains:

```text
dwv demo init
  -> healthy read/write/flush/reopen
  -> baseline and read-only scrub
  -> induced data/parity fault and evidence classification
  -> verified repair or conservative refusal
  -> degraded read and resumable rebuild
  -> recovery-metadata-loss handling
  -> trace export, rendering, and deterministic replay
```

Use the repository's actual command syntax; do not rename working commands merely to match this diagram.

## 6.3 The demo must exercise real implementation paths

`dwv demo` SHALL route through the same portable core, transaction/recovery logic, integrity model, file stores, and simulator/trace contracts being accepted. It may use disposable fixtures and explicit fault-injection adapters, but it must not return hard-coded or parallel mock results that bypass the behavior under test.

When a feature is unavailable, the CLI must report a truthful `blocked`, `unsupported`, `refused`, or equivalent state rather than simulate success.

Machine-readable results and exit statuses must distinguish at least:

- operation completed successfully;
- state verified clean;
- mismatch detected;
- repair/refusal decision;
- blocked by missing capability or prerequisite;
- uncertain/indeterminate result;
- malformed or incompatible input.

## 6.4 Preserve failures as regression artifacts

When `dwv demo`, property testing, fuzzing, a model checker, or a scheduler finds a failure:

```text
failure
    -> preserve original producer artifact
    -> minimize semantically where possible
    -> export normalized DiskWeave scenario/trace when representable
    -> prove deterministic dwv-sim or file-backed replay
    -> add permanent regression fixture
    -> fix
    -> rerun focused and integrated demo workflows
```

Do not discard a producer-specific fuzz input, scheduler witness, or model counterexample merely because a normalized trace exists; the normalized artifact may omit information.

# 7. Apply v0.7 verification layers without restarting the implementation

Use these as dependency-ready evidence changes, merging them into coherent work where appropriate.

## VE-001 — bounded arithmetic and pure-component verification

Evaluate Kani or a materially equivalent mechanism for stable, small, high-consequence components such as:

- logical/physical address mapping;
- range decomposition;
- capacity and offset arithmetic;
- alignment/bounds;
- parity-envelope locations;
- bounded parity-update equivalence;
- future isolated unsafe resource helpers.

Do not attempt application-wide formal verification. No production architecture change is required if existing pure seams are adequate.

## VE-002 — independent abstract transaction/recovery protocol model

Evaluate **PlusCal-authored TLA+ checked with TLC** against **Stateright** rather than assuming either.

Provisionally prefer TLA+/PlusCal when an independently expressed model materially reduces the risk of repeating a Rust implementation mistake. Prefer Stateright when Rust-native exploration, counterexample handling, maintainability, and implementation-team ergonomics provide greater assurance without sharing production transition code.

The comparison must consider:

- independence from production Rust types and code;
- ability to express nondeterministic crashes and recovery;
- safety, reachability, and any needed liveness/fairness assumptions;
- counterexample readability and translation into `dwv-sim` scenarios;
- state-space control and symmetry;
- CI/tooling maintenance;
- model drift and mutation tests;
- license and project health.

Maintain one primary model by default. Do not mirror every Rust field or call production transition functions.

The first model should cover properties such as:

- dirty intent and integrity invalidation precede dependent mutation;
- uncertain completion is never silently converted to success or rollback;
- clean/checkpoint is never reached prematurely;
- consequential work cannot disappear without terminal resolution or durable recovery handoff;
- crash recovery produces only allowed states;
- recovery is conservative and idempotent;
- device loss cannot improve certainty.

Record finite bounds, omitted facts, symmetry assumptions, fairness assumptions, and explicit non-claims. A passing finite model is evidence for that model, not proof of the entire application.

## VE-003 — concurrency schedule exploration

Evaluate Shuttle or equivalent for larger reproducible schedules involving transaction concurrency, operation slots, abandonment, shutdown/draining, device-state transitions, and background work.

Use Loom only for tiny custom synchronization/atomic lifecycle mechanisms where exhaustive exploration within the harness is worth the maintenance cost, such as stale-token rejection, exactly-once terminalization, permit accounting, or custom reclamation.

Normal deterministic tests remain preferred where they provide adequate coverage. Do not convert the production runtime merely to use Shuttle or Loom.

## VE-004 — production-adjacent deterministic I/O simulation

Once executor/resource-lifecycle code exists, evaluate Turmoil or a materially better alternative as a test-only complement to `dwv-sim`.

The implementing agent should verify that the existing executor/store seam permits exercising production-adjacent code against deterministic simulated I/O. If it already does, no architecture change is required. If not, propose the smallest test seam and justify it with a concrete property.

Candidate evidence includes:

- successful-but-not-durable writes;
- `fsync`/durability boundaries;
- crashes and torn writes;
- delayed, failed, or uncertain completions;
- operation/resource ownership until terminal state;
- macOS-hosted execution of Linux-oriented adapter logic where the simulator actually supports it.

This evidence does not prove real Linux `io_uring`, ublk, blk-mq, ext4/XFS, device caches, resets, FUA, or physical power-loss behavior.

## VE-005 — property/fuzz schedule generation and corpus

Use `proptest`, `cargo-fuzz`/libFuzzer, or evidence-backed alternatives for:

- generated operation/fault/crash schedules through `dwv-sim`;
- transaction/recovery transitions;
- parity encode/update/reconstruction equivalence;
- hostile parity-envelope and manifest input;
- trace parsing/replay;
- topology transitions;
- range decomposition and boundary arithmetic.

Promote minimized failures into the common reproducer bundle when representable.

# 8. Rules for current milestone reconciliation

## OS-007 — parity-envelope profiles and decoder

Do not restart a conformant implementation. Revisit or amend it when v0.7 exposes missing hostile-input bounds, migration/torn-copy behavior, deterministic decoding evidence, exact-capacity handling, model/fuzz regressions, or accidental coupling to transaction state.

The format remains experimental unless its stable-format gate has actually been met.

## OS-009 — transaction-engine evidence comparison

Preserve the explicit-machine path and the requirement to exercise the real pinned `procmachines` dependency before broad integration.

Revisit completed evidence if it fails to cover terminal ownership, uncertain completion, abandonment, stale token reuse, premature clean/checkpoint, skipped integrity invalidation, or normalized trace equivalence required by v0.7.

Do not replace `dwv-service` or the production path merely to make a testing framework easier to use.

## OS-017 — checksum scrub and verified repair

Revisit the work if repair can occur from parity disagreement alone, if plans are not identity/generation bound, if source media can be modified on refusal/uncertainty, or if digest/parity readback does not independently verify the target.

Exercise the behavior continuously through `dwv demo`, including data corruption, parity corruption, stale/absent evidence, multiple suspects, failed writes, interruption, and stale plans.

## OS-024 — normalized trace fixture and replay

Treat OS-024 as the portable semantic reproducer layer, not as the only artifact produced by every verification tool.

Revisit it if traces omit consequential terminal outcomes, obligation/resource lifecycle, recovery/checkpoint state, integrity decisions, uncertainty, schema limits, malformed-input rejection, or deterministic final state needed by v0.7.

Continue exporting, rendering, and replaying traces through `dwv demo` against both `dwv-sim` and the file-backed path where supported.

# 9. Autonomous decision policy

Make routine implementation decisions without asking the user when they remain behind accepted seams and are reversible, including:

- test crate and harness organization;
- bounded queue/worker/cache defaults;
- SQLite settings within accepted durability semantics;
- fixture sizes and generated seeds;
- trace limits;
- property strategy composition;
- internal Rust organization;
- choice among verification tools that prove equivalent properties without affecting product semantics or formats.

Escalate only when evidence reveals:

- a product-level semantic or compatibility choice;
- a destructive migration or irreversible format decision;
- a materially different recoverability/availability promise;
- a safety-preserving choice that would substantially undermine the project goal;
- genuinely inconclusive evidence between alternatives with different user-visible consequences.

# 10. Goal v3 deliverables

Produce and maintain, using repository conventions:

1. **Autonomous implementation goal v3** with actual statuses, dependency order, remediation work, continuous demo evidence, and completion criteria.
2. **v0.7 conformance matrix** linking every material property to code, tests, OpenSpecs, evidence, or a required corrective item.
3. **Corrective/successor OpenSpecs or amendments** for non-conformant or under-evidenced completed work, preserving historical artifacts.
4. **`dwv demo` evidence log** containing the baseline, focused post-change scenarios, full integrated checkpoints, outputs, exit statuses, and claim boundaries.
5. **Permanent regression corpus** containing minimized simulator/property/fuzz/CLI failures and normalized traces where representable.
6. **Verification decision records** for TLA+/PlusCal versus Stateright and any adopted bounded/concurrency/I/O-simulation tool.
7. **Updated dependency/status index** naming the exact next dependency-ready action.

# 11. Completion standard

Goal v3 is complete only when:

- every applicable v0.7 conformance item is `CONFORMANT`, or is platform-deferred with an honest bounded claim that no portable safety issue is being hidden;
- no correctness, integrity, recovery, lifecycle, or unsupported-claim gap is deferred merely because the responsible work was previously completed or archived;
- OS-007, OS-009, OS-017, and OS-024 are complete under their actual current contracts plus any required v0.7 remediation;
- the existing `dwv demo` CLI exercises the integrated disposable portable workflow using real implementation paths;
- focused and full demo scenarios pass with deterministic evidence, or report truthful refusal/blocked/uncertain outcomes;
- discovered defects have retained reproducible regression artifacts before or alongside their fixes;
- no verification tool has distorted core semantics, persistent formats, the transaction representation, or the CLI merely to make the tool convenient;
- the next platform-gated claims remain explicitly unclaimed until their Linux, macOS bridge, or hardware evidence exists.

After updating goal v3, continue executing the next dependency-ready item. Do not stop at the audit or planning document.
