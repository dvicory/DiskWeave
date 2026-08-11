## Context

See `proposal.md` and the transaction orchestration sections of
`docs/architecture/archive/diskweave-architecture-roadmap-v0.6.md`. `dwv-transaction-ref`
already contains the explicit semantic action/result vocabulary, reference
machine, and trace. The comparison must happen before changing service
selection or extracting a shared production contract.

## Goals / Non-Goals

**Goals:**

- Exercise one real `procmachines` state machine against the existing explicit
  machine, not a fake stand-in or a source-text comparison.
- Reuse the existing semantic action/result types and plans.
- Drive both implementations with identical deterministic schedules and compare
  normalized traces and terminal safety classes.
- Measure stable comparison metrics and document dependency/audit boundaries.

**Non-Goals:**

- No service-engine migration, broad refactor, public `TransactionMachine` trait,
  or replacement of `dwv-transaction-ref`.
- No runtime, kernel, ublk, filesystem, SQLite, or physical durability adapter.
- No claim that `procmachines` supplies parity correctness, cancellation safety,
  buffer lifetime, or crash recovery.

## Decisions

### Isolate the candidate in `dwv-transaction-proc`

Add one workspace crate depending on `procmachines = "1.0"` and the existing
reference crate's semantic vocabulary. Its public API contains only DiskWeave
plans, semantic actions/results, normalized events, and comparison outcomes.
The `procmachines` builder, IO exchanges, task futures, and guards remain
private to the crate.

This is preferable to adding the dependency to `dwv-service` or replacing the
reference machine first: a failed comparison then deletes one crate and one
lockfile dependency without touching production semantics.

### Use one procedural task over semantic rendezvous

The candidate task emits exactly one semantic action at a time through a
`procmachines` `IoExchange`, awaits one `ActionResult`, and advances its small
semantic state. External code drives the exchange synchronously. Child I/O
fanout is intentionally absent; the comparison boundary is the semantic action,
not SQE/CQE behavior.

The task mirrors only the reference machine's safety transitions: pre-intent
failure may abort, post-intent/home-mutation failure requires reconciliation,
and checkpoint completion requires the same fence evidence. It does not copy
reference-machine internals or call the reference machine to compute its
outcome.

### Normalize after external driving

A comparison driver builds one `TransactionPlan`, clones it for both engines,
and supplies the same `ActionResult` schedule. It records action kind, result
kind, pre/post semantic stage, and terminal disposition. Trace comparison uses
those normalized records; child ordering and batching are not represented.

The driver includes schedules for every listed fault boundary. Duplicate
terminal results are replayed through the adapter's explicit duplicate policy;
stale operation generations are tested at the operation-slot seam and never
reach reused semantic state.

### Measure evidence, not a winner by intuition

Each run records elapsed time, semantic action/result counts, trace length,
structural machine/IO sizes, synchronization-operation counts exposed by the
adapter, and dependency metadata. Timing is comparative evidence only, not a
production benchmark. The ADR uses mandatory correctness gates first; resource
and maintenance costs break ties only after semantic equivalence passes.

### Keep selection reversible

The first implementation records an ADR and verification report. It does not
change `dwv-service` engine selection. If the candidate wins a later change may
place it behind a semantic adapter; if it loses, deleting `dwv-transaction-proc`
and its dependency leaves the explicit machine and all existing traces intact.

## Risks / Trade-offs

- **[Risk] A candidate test accidentally compares two copies of the same
  algorithm.** → Keep the procedural task's transitions independent and test
  action/result traces against the explicit machine only through the driver.

- **[Risk] `procmachines` API or maintenance changes.** → Pin the resolved
  version, keep all uses in one crate, record license/metadata, and document a
  vendor/fork exit path.

- **[Risk] Timing noise produces a false performance conclusion.** → Treat
  correctness as a hard gate, repeat deterministic schedules, report medians,
  and never select on one timing sample.

- **[Risk] Semantic vocabulary remains coupled to the reference crate.** →
  Deliberately defer extraction until equivalence evidence exists; the later
  extraction is an explicit follow-on, not hidden in this spike.

## Migration Plan

No production migration occurs. The candidate is added, exercised, and either
recorded as provisional behind no service path or rejected. Rollback is deleting
the candidate crate, its Cargo dependency, comparison tests, and this change's
experimental evidence; `dwv-transaction-ref` remains unchanged.
