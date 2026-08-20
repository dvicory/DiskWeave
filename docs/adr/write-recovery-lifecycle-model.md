# ADR: verify.write-recovery-lifecycle delegated Quint recovery-protocol model

- **Status:** Accepted for the portable evidence lane
- **Date:** 2026-08-17
- **Current model source:** `models/quint/RecoveryProtocol.qnt`
- **Reference checker:** Quint 0.32.0 with its Rust simulation backend
- **Tool pin:** `mise.toml` pins `@informalsystems/quint` at `0.32.0`

## Decision

At the current synced revision, `models/quint/RecoveryProtocol.qnt` is the
sole current model authority for the bounded write-recovery lifecycle reference relation. The
parameterized `RecoveryProtocol` module is the canonical protocol source. The
separate `RecoveryProtocolAnalysis` module is maintained at
`verification/quint/RecoveryProtocolAnalysis.qnt`; it binds that relation to
the finite write-recovery lifecycle evidence instance and provides assumptions, witnesses, and
runs without adding protocol semantics.

At this boundary, the delegated authority is the parameterized state, action,
transition, release, and invariant relation. It admits one active write
obligation at a time; explicit release permits sequential range reuse. The
finite evidence instance uses `Regions = {"data", "parity"}`,
`Stores = {"data", "parity"}`, and checker depth 12. Those values bound
verification evidence only; they are not product cardinality limits, protocol
alternatives, or exhaustive proof of arbitrary-width instances.

The model does not define exact region mapping, checksum extent semantics,
topology identity, persistence-evidence admissibility, store persistence, adapter
commit observations, operation-slot lifetime, frontend delivery, or
production recovery authority. Those decisions remain owned by their current
requirements.

This is an authority transition, not a retroactive rewrite. The retired TLA
source, its recorded evidence, and archived change artifacts remain
historical provenance, not a second current authority.


## Why Quint

The old PlusCal model was independent, but it advanced an unknown data/parity-write outcome
to durable through an implicit recovery step and did not represent an
uncertain write-recovery-record commit separately from a rejected one. The Quint model makes
both uncertainty boundaries explicit and requires an explicit reconciliation
action before a clean or terminal state can be reached.

## Model scope and invariants

The parameterized protocol relation admits one active write lifecycle at a
time. Explicit release permits sequential range reuse within the represented
relation. Its state vocabulary includes:

- pending, durable, and unknown write-recovery records;
- no, awaiting-durability, durable, and unknown data/parity writes;
- clean, dirty, and indeterminate recovery;
- unowned, normal, interrupted, and completed-awaiting-release ownership;
- parameterized affected-region and store sets, persistence-evidence and
  recovery-CLEAN coverage, abandonment, process loss, and explicit
  reconciliation.

It checks the ten current write-recovery lifecycle invariants:

- `TypeInvariant`;
- `NoFalseClean`;
- `DataParityWriteRequiresWriteRecoveryRecord`;
- `UncertaintyIsVisible`;
- `UncertaintyIsOwned`;
- `DurableWorkIsOwned`;
- `CompletedOrAbortedRequiresRelease`;
- `CompletedWriteRequiresEvidence`;
- `DurableDataParityWriteRequiresCoverage`;
- `PersistenceEvidenceAndRecoveryCleanCoverage`.

The write-recovery lifecycle analysis binds `Regions = {"data", "parity"}` and
`Stores = {"data", "parity"}` and uses checker `--max-steps 12`. These are
finite executable evidence bounds, not exhaustive model checking, proof of
the Rust implementation, real I/O, a persistence engine, or unbounded
recovery progress.

## Checker comparison

| Option | Independence from DiskWeave | Evidence fit | Decision |
|---|---|---|---|
| **Quint 0.32.0** | High: model-only Quint source and separate simulator | Bounded safety, reachability, explicit uncertainty, deterministic traces, mutation checks | **Current write-recovery lifecycle authority** |
| Official TLC | High: mature separate checker for TLA+ | Historical bounded safety and reachability | Retired with the source model |
| `tla-rs` / `tla-checker` 0.6.11 | High checker independence, same retired TLA+ source | Historical finite cross-check | Retired with the source model |
| Stateright 0.31 | Medium: separate Rust model required | Safety and reachability | Not selected; no second model |

Quint is a verification model, not a Rust translation of production
transitions. The model remains independent from DiskWeave implementation
types and runtime code.

## Historical canary evidence (pre-repair)

The following command record and observations were captured before the active
`repair-ve002-quint-authority` change. They preserve the first Quint canary's
evidence and are not a current reproduction against the repaired working tree.

```text
quint typecheck models/quint/RecoveryProtocol.qnt
quint test verification/quint/RecoveryProtocolAnalysis.qnt --main RecoveryProtocolAnalysis
quint run verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --max-steps 12 \
  --invariants TypeInvariant \
  --invariants NoFalseClean \
  --invariants MutationRequiresIntent \
  --invariants UncertaintyIsVisible \
  --invariants DurableWorkIsOwned \
  --invariants TerminalRequiresEvidence \
  --invariants FenceAndCheckpointCoverage \
  --witnesses beginReachable \
  --witnesses durableIntentReachable \
  --witnesses mutationReachable \
  --witnesses uncertainIntentReachable \
  --witnesses uncertainHomeReachable \
  --witnesses reconciliationReachable \
  --witnesses terminalReachable \
  --max-samples 10000 \
  --seed 22082026
```

On 2026-08-17, the historical typecheck and bounded-assumption test passed.
The sampled run found no invariant violation across 10,000 traces. Witness
coverage was: begin 100.00%, durable intent 95.09%, mutation 90.70%,
uncertain intent 67.05%, uncertain home 84.17%, reconciliation handoff
99.98%, and terminal ownership 1.96% (196 traces). The terminal witness's
low rate was expected from the guarded path; its non-zero reachability was
the required result.

Two runs with the same seed produced byte-identical normalized ITF traces
after removing generated timestamps. A disposable copy with the `mutate`
durable-intent guard removed was rejected by `MutationRequiresIntent`. The
mutant was not retained.


## Current model and evidence

The current `models/quint/RecoveryProtocol.qnt` relation adds explicit
range-owned and release state, permits a new `startWrite` after release, gives
pre-write-recovery-record rejection an owned aborted outcome, guards completed
reuse before release, and requires complete represented data/parity-write
coverage before a durable data/parity-write result. Invalid, repeated, and
out-of-order action handling is partial rather than silently transitioning.

Current evidence includes direct release/reuse, completed-start rejection, and
partial-data/parity-write tests; bounded witnesses for aborted, released,
resumed-data/parity-write, reconciled-data/parity-write, and
`durableDataParityWriteReachable` states; bounded Quint verification at checker
depth 12; deterministic ITF replay; disposable negative mutations; and two
seeded Quint Connect runs.

The retained bounded command is:

```text
quint verify verification/quint/RecoveryProtocolAnalysis.qnt \
  --main RecoveryProtocolAnalysis \
  --max-steps 12 \
    DataParityWriteRequiresWriteRecoveryRecord \
    UncertaintyIsVisible UncertaintyIsOwned DurableWorkIsOwned \
    CompletedOrAbortedRequiresRelease CompletedWriteRequiresEvidence \
    DurableDataParityWriteRequiresCoverage \
    PersistenceEvidenceAndRecoveryCleanCoverage
```

With Quint `0.32.0` and Apalache `0.56.1`, it completed without an invariant
violation. The command binds the finite analysis instance
`Regions = {"data", "parity"}` and `Stores = {"data", "parity"}`; depth 12 is
the retained checker bound, not a product or `MaxDepth` parameter. The model
checks the ten invariants listed above. Sampled simulation, direct scenario
tests, mutations, ITF replay, and Connect executions remain separate evidence
types.

The two retained Connect traces, seeds `22082026` and `1`, each complete a
mapped lifecycle through `releaseRange`, then execute a new `startWrite` and
second lifecycle through `releaseRange`. The Connect driver covers only the
mapped write-lifecycle fields: Rust batches reads, parity, and writes, while
Quint separates one mapped abstract region mutation. The projection excludes
abstract persistence-evidence and data/parity-write-reconciliation transitions
whose concrete evidence is owned elsewhere. Persistence evidence, watermark,
generation, topology, concrete stale-result correlation, and result-class
evidence remains Rust-owned.

## Lessons and next campaign

The canary showed that delegated authority is useful only when the boundary
names the exact state and actions. It exposed real modeling defects rather
than merely translating syntax: write-recovery-record uncertainty and
data/parity-write uncertainty need different explicit reconciliation states;
terminal ownership must survive until release; and a released range must be
reusable without silently replacing an unreleased obligation. It also showed
that sampled simulation needs witnesses for rare terminal paths; a green
invariant run alone would not establish that persistence evidence and terminal
ownership are reachable.

The bounded Connect spike taught a second boundary lesson. A useful test can
replay a small shared lifecycle through the Rust reference machine without
turning the model into a Rust mirror. The exact projection must be named:
Rust's batched action/result and typed evidence seam is not a one-to-one
implementation of the abstract Quint relation. The test is therefore
conformance evidence for the mapped subset, not implementation proof.

The highest-value next application is the **portable shutdown, endpoint
withdrawal, and claim-release ordering** capability, after its existing
implementation readiness gate is resolved. A small Quint model should own only the bounded
ordering and conservative outcomes for admission closure, quiescence, drain
or handoff, exact recovery-CLEAN/close-session evidence, endpoint withdrawal,
and claim release. It should leave recovery authority, operation-slot lifetime,
frontend-specific endpoint ownership, and operator result vocabulary with
their current owners.
`verify.concurrency-schedules` remains the follow-on choice once the executor,
job, and shutdown concurrency seam exists.

References:

- [Quint](https://github.com/informalsystems/quint)
- `docs/architecture/normalization/diskweave-v0.9-campaign.md` (portable shutdown, endpoint withdrawal, and claim-release ordering)
