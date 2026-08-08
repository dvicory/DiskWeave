# OS-009 transaction-engine evidence comparison verification

## Claim boundary

This record covers a portable semantic comparison between the explicit
`dwv-transaction-ref` machine and one `procmachines = 1.0.0` adapter. It does
not claim production transaction-engine selection, Linux ublk behavior, macOS
bridge behavior, filesystem semantics, physical power-loss durability, backend
completion ordering, or safe cancellation of kernel operations.

## Implementation evidence

- `crates/dwv-transaction-proc/src/lib.rs` keeps `IoExchange`, the procedural
task, polling guards, and `procmachines` types private.
- `semantic_actions` emits one existing `TransactionAction` at a time.
- `CandidateMachine` consumes one existing `ActionResult` at a time and exposes
only semantic actions, results, terminal classes, normalized events, and
comparison metrics.
- `compare_schedule` clones one `TransactionPlan`, drives both machines with
  the same result sequence, and compares normalized action/result/stage events
  and terminal classes.
- `measure_schedule` repeats equivalent comparisons and reports candidate and
  reference median elapsed time, synchronization operations, and structural
  sizes.
- Generation, topology, intent-evidence, fence-evidence, and checkpoint proof
  inputs are validated before the candidate can report completion.
- `abandon`, `daemon_crash`, and `power_loss` are semantic control boundaries;
  operation-slot ownership and actual backend lifetime remain in `dwv-store` and
  its executor adapters.

## Acceptance commands

The following commands were run on 2026-08-08:

- `cargo fmt --all -- --check` — passed.
- `cargo test -p dwv-transaction-proc` — passed: 16 tests.
- `cargo test --workspace` — passed: 217 tests, 1 ignored.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings` — passed.
- `cargo tree -p dwv-transaction-proc -e normal` — passed; `procmachines
  v1.0.0` appears only under the candidate crate's dependency tree.
- `cargo metadata --format-version 1 --no-deps` — passed; the candidate is a
  workspace member and no production service dependency was added.
- `cargo run -q -p dwv-transaction-proc --example comparison-report -- 10000`
  — passed: `equivalent=true`, semantic steps `8`, candidate median
  `11542 ns`, reference median `3792 ns`, candidate/reference `3.044`, 8
  synchronization operations, candidate IO size 680 bytes, reference machine
  size 560 bytes.
- `openspec validate os-009-transaction-engine-evidence-comparison --strict
  --json` — passed.
The candidate tests exercise successful completion, pre-intent failure,
post-write uncertainty, short I/O, stale recovery generation, delayed
delivery, wrong-order result rejection, duplicate terminal delivery,
abandonment before and after durable intent, daemon crash and power loss at
every modeled semantic suspension point, rejection of non-durable fence
evidence, normalized trace equivalence, and repeated-run median measurement.

## Apples-to-apples boundary

Semantic work is matched: both engines receive the same fresh plan and
prebuilt result sequence, emit the same normalized action/result/stage trace,
reach the same terminal class, and are checked for eight semantic steps in the
comparison example. Implementation work is not identical: the reference uses
direct state-machine calls, while the candidate includes the procedural task,
`IoExchange` traffic, polling, and waker path. The elapsed ratio therefore
measures the complete adapter path, not pure DiskWeave policy cost. The
candidate-IO and reference-machine sizes are structural proxies for different
types, not comparable memory footprints.

## Code-smell and maintainability evidence

The candidate is easy to isolate and delete, and its deterministic test seam
is easier to drive than a hand-written suspended-state enum. The negative
maintainability finding is duplicated policy: action ordering, result
validation, stage mapping, and failure classification are repeated outside
the reference machine. That duplication is necessary for an independent
comparison, but would be a serious drift hazard if both implementations were
kept in production.

The 10,000-run result is a negative performance signal for this workload:
procedural orchestration was about 3x slower and its IO surface was larger.
This is a sequential semantic microbenchmark, not a concurrency or backend
benchmark; it is sufficient to reject an automatic migration, not to reject
the library for every future recovery procedure.

## Decision boundary

The maintainer explicitly deferred the final adoption decision while keeping
both implementations. The explicit machine remains the interim production
fallback; the procedural candidate remains isolated and is not integrated into
`dwv-service` or runtime/backend paths. The evidence does not authorize
automatic adoption or deletion.
