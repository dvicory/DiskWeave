# ADR: OS-009 transaction-engine evidence comparison

- **Status:** Accepted interim disposition; final adoption decision deferred
- **Date:** 2026-08-08
- **Scope:** Portable semantic transaction orchestration only

## Interim decision boundary

The maintainer decision for this milestone is to keep both implementations:
`dwv-transaction-ref::TransactionMachine` remains the correctness oracle and
current production fallback, while `dwv-transaction-proc` remains an isolated
comparison spike. Do not integrate the candidate into `dwv-service` or any
runtime/backend path while the final adoption decision is deferred.

`procmachines` is pinned to `=1.0.0` and confined to
`dwv-transaction-proc`. The candidate exchanges only DiskWeave semantic
actions and results. Its procedural task does not own persistent recovery
state, backend I/O, operation slots, buffers, durability, or crash recovery.

The maintainer has explicitly deferred the final adoption decision. Keeping
both implementations means the explicit machine remains the production
fallback and the procedural candidate remains available for future review,
without allowing the candidate into service or runtime paths.

The 10,000-run measurement is evidence for that deferred decision: the
candidate was semantically equivalent but had a median of 11,542 ns versus
3,792 ns for the explicit machine (3.044x slower), with a 680-byte candidate
IO surface versus a 560-byte reference machine surface.

## Evidence

The candidate and reference machine share one dirty-region parity-write plan and
are driven by the same normalized result sequence. The comparison reports:

- semantic action/result kinds and pre/post stages;
- terminal safety classification;
- deterministic polling and synchronization-operation counts;
- candidate IO structural size and reference-machine structural size;
- repeated-run median elapsed time for the candidate path.

The executable comparison covers successful completion, acquisition failure,
post-write uncertain completion, stale recovery-generation rejection, delayed
semantic result delivery, wrong-order result rejection, duplicate terminal
delivery, frontend abandonment before completion, daemon crash before intent,
and power loss after intent acquisition. The candidate requires the same fence,
topology, generation, and integrity evidence before checkpoint completion.

## Apples-to-apples boundary

The comparison is apples-to-apples for semantic work, not for every
implementation-level cost:

- both engines receive the same fresh plan and the same prebuilt result
  sequence;
- normalized action/result/stage traces and terminal disposition must match;
- the representative plan contains eight semantic steps, and the report
  asserts both engines emit all eight;
- reference timing measures direct `TransactionMachine` construction and
  `apply` calls, while candidate timing measures the `procmachines` task,
  `IoExchange` traffic, polling, and waker path.

Therefore the 3.021x result measures the cost of this complete procedural
adapter against the direct reference machine. It does not prove that a
procedural algorithm performs 3.021x more DiskWeave policy work. The two size
values are structural proxies for different types, not an apples-to-apples
memory measurement. A future CPU-only engine comparison would need a common
step-level harness and separately measured adapter overhead; that evidence was
not collected here.

## Code-smell and maintainability assessment

- **Duplication risk:** the candidate independently repeats action ordering,
  result validation, failure classification, and stage mapping. This is
  required for an independent oracle comparison, but it is a clear long-term
  drift hazard if both implementations become production code.
- **Procedural ergonomics:** adding a new semantic action requires edits to
  `semantic_actions`, result matching, stage mapping, and fault classification.
  The async syntax hides suspension bookkeeping, but it does not remove
  DiskWeave policy code.
- **Boundary quality:** no `ProcMachine`, `IoExchange`, guard, or dependency
  type crosses the public adapter API. The candidate is easy to delete and
  cannot silently enter `dwv-service`.
- **Test ergonomics:** deterministic polling, delayed delivery, control
  injection, normalized traces, and repeated measurements are straightforward;
  this is the candidate's strongest practical benefit.
- **Complexity cost:** the candidate is larger in memory and slower in this
  representative run before any backend work is included. The current evidence
  does not justify paying that cost for authoring convenience.

## Consequences

- `dwv-transaction-ref` remains independently reviewable and modelable.
- The candidate can be removed without changing service or persistent formats.
- `procmachines` may simplify branch-heavy procedural orchestration, but this
  spike does not establish parity correctness, cancellation safety, operation
  lifetime, or physical durability.
- Timing is comparison evidence only. No telemetry framework or production
  exporter is added to the portable core; future observability belongs at the
  service/executor seam.
- Child-I/O batching and kernel completion ordering are intentionally outside
  this semantic comparison. The one-outstanding-action boundary records delayed
  delivery and rejects a result delivered for the wrong action.
