# ADR: VE-002 delegated Quint recovery-protocol model

- **Status:** Accepted for the portable evidence lane
- **Date:** 2026-08-17
- **Model source:** `models/quint/RecoveryProtocol.qnt`
- **Reference checker:** Quint 0.32.0 with its Rust simulation backend
- **Tool pin:** `mise.toml` pins `@informalsystems/quint` at `0.32.0`

## Decision

At the last synced revision, `models/quint/RecoveryProtocol.qnt` is the
sole current VE-002 model authority. The active
`repair-ve002-quint-authority` edits to that source are a proposed extension
until normal OpenSpec verification and sync; they do not currentize their
changed semantics in this working revision. The parameterized
`RecoveryProtocol` module is the canonical protocol source. The finite
`RecoveryProtocolAnalysis` module is maintained separately at
`verification/quint/RecoveryProtocolAnalysis.qnt`; it binds that relation to
the finite VE-002 evidence instance and provides assumptions, witnesses, and
runs.

At that synced boundary, the delegated authority is the parameterized state,
action, transition, and invariant relation. The VE-002 evidence instance uses
one represented write obligation, `Regions = {"data", "parity"}`,
`Stores = {"data", "parity"}`, and
`MaxDepth = 8`. Those values bound the verification evidence only; they are
not product cardinality limits, protocol alternatives, or exhaustive proof
of arbitrary-width instances.

The model does not define exact region mapping, checksum extent semantics,
topology identity, typed fence admissibility, store persistence, adapter
commit observations, operation-slot lifetime, frontend delivery, or
production recovery authority. Those decisions remain owned by their current
requirements.

This is an authority transition, not a retroactive rewrite. The archived
`replace-ve002-tla-with-quint` and `reconcile-ve002-quint-ownership` changes
established Quint as the current model authority. The retired TLA files remain
historical provenance, not a second current authority.

## Why Quint

The old PlusCal model was independent, but it advanced an unknown home effect
to durable through an implicit recovery step and did not represent an
uncertain intent commit separately from a rejected one. The Quint model makes
both uncertainty boundaries explicit and requires an explicit reconciliation
action before a clean or terminal state can be reached.

## Model scope and invariants

The parameterized protocol relation represents one admitted write obligation.
Its state vocabulary includes:

- pending, durable, and unknown invalidation intent;
- unmodified, volatile, durable, and unknown home effects;
- clean, dirty, and indeterminate recovery;
- unowned, in-flight, handoff, and terminal ownership;
- parameterized affected-region and store sets, fence coverage, checkpoint
  coverage, abandonment, process loss, and explicit reconciliation.

It checks the nine VE-002 invariants:

- `TypeInvariant`;
- `NoFalseClean`;
- `MutationRequiresIntent`;
- `UncertaintyIsVisible`;
- `DurableWorkIsOwned`;
- `TerminalRequiresRelease`;
- `TerminalRequiresEvidence`;
- `DurableHomeRequiresCoverage`;
- `FenceAndCheckpointCoverage`.

The analysis binds `MaxDepth = 8`, `Regions = {"data", "parity"}`, and
`Stores = {"data", "parity"}`. Its sampled runs are finite executable
evidence, not exhaustive model checking and not proof of the Rust
implementation, real I/O, a persistence engine, or unbounded recovery
progress.

## Checker comparison

| Option | Independence from DiskWeave | Evidence fit | Decision |
|---|---|---|---|
| **Quint 0.32.0** | High: model-only Quint source and separate simulator | Bounded safety, reachability, explicit uncertainty, deterministic traces, mutation checks | **Current VE-002 authority** |
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


## Active repair target (proposed until sync)

The active `repair-ve002-quint-authority` change repairs the delegated
relation without changing current OpenSpec authority until its normal
verification and sync. Its target adds explicit range-held and
range-release state, permits a new `begin` after release, gives
pre-mutation intent rejection an owned aborted outcome, guards terminal
reuse before release, and requires complete represented mutation coverage
before a durable home result. It also makes invalid, repeated, and
out-of-order action handling partial rather than silently transitioning.

Target evidence includes direct release/reuse, terminal-begin rejection, and
partial-home tests; bounded witnesses for aborted, released, resumed-mutation,
and durable-home-coverage states; bounded Apalache verification; and two
seeded Quint Connect projections through `dwv-transaction-ref`. These checks
are evidence for the active change, not a second semantic owner and not an
exhaustive proof. The Connect driver covers only the mapped lifecycle fields:
Rust batches reads, parity, and writes, while Quint separates abstract
region mutation; abstract fence and home-reconciliation observations have no
independent Rust action in the driver. Typed fence, watermark, generation,
topology, and result-class evidence remains Rust-owned.

## Lessons and next campaign

The canary showed that delegated authority is useful only when the boundary
names the exact state and actions. It exposed real modeling defects rather
than merely translating syntax: intent-commit uncertainty and home-effect
uncertainty need different explicit reconciliation states; terminal ownership
must survive until release; and a released range must be reusable without
silently replacing an unreleased obligation. It also showed that sampled
simulation needs witnesses for rare terminal paths; a green invariant run
alone would not establish that checkpoint and terminal ownership are
reachable.

The bounded Connect spike taught a second boundary lesson. A useful test can
replay a small shared lifecycle through the Rust reference machine without
turning the model into a Rust mirror. The exact projection must be named:
Rust's batched action/result and typed evidence seam is not a one-to-one
implementation of the abstract Quint relation. The test is therefore
conformance evidence for the mapped subset, not implementation proof.

The highest-value next application is **U11: portable shutdown, endpoint
withdrawal, and claim-release ordering**, after its existing implementation
readiness gate is resolved. A small Quint model should own only the bounded
ordering and conservative outcomes for admission closure, quiescence, drain
or handoff, exact checkpoint/close-session evidence, endpoint withdrawal, and
claim release. It should leave recovery authority, operation-slot lifetime,
frontend-specific endpoint ownership, and operator result vocabulary with
their current owners. VE-003 remains the follow-on choice once the executor,
job, and shutdown concurrency seam exists.

References:

- [Quint](https://github.com/informalsystems/quint)
- `openspec/specs/explicit-transaction-machine/spec.md`
- `docs/architecture/normalization/diskweave-v0.9-campaign.md` (U11)
