# ADR: VE-002 independent recovery-protocol model

- **Status:** Accepted for the portable evidence lane
- **Date:** 2026-08-08
- **Model source:** PlusCal embedded in `verification/tla/RecoveryProtocol.tla`
- **Reference checker:** official TLC 2.19 from `tla2tools.jar` 1.7.4
- **Rust checker:** `tla-checker` 0.6.11 (`tla-rs`)

## Decision

Use `verification/tla/RecoveryProtocol.tla` as the first independent abstract
model of the dirty/integrity/recovery protocol. Its PlusCal algorithm is the
source; the generated `BEGIN TRANSLATION` block is checked by a checker and
must be regenerated with the official translator after source changes. The
checked configuration is `verification/tla/RecoveryProtocol.cfg`.

The repository pins the Java major version through `mise.toml`:

```toml
java = "temurin-21"
```

The local TLC evidence run used Temurin 21.0.12 and official `tla2tools.jar`
1.7.4 (SHA-1 `bee4a54f3ee3d4afc347c3240ec2d9e93b075104`). The jar is not
committed; it is a tool input, not a DiskWeave runtime dependency.

`cargo install tla-checker --version 0.6.11 --locked` provides a Java-free
Rust checker for the same TLA+ source and config. It is recorded as a
cross-check, not as a second production model or a replacement semantic
implementation.

## Checker comparison

| Option | Independence from DiskWeave | Evidence fit | Decision |
|---|---|---|---|
| Official TLC 2.19 | High: separate TLA+ notation and mature JVM checker | Safety, reachability, explicit crash boundaries, and qualified liveness | Reference checker |
| **`tla-rs` / `tla-checker` 0.6.11** | High model independence: it parses the same TLA+ source in a separate Rust implementation; lower tool maturity than TLC | Finite safety/reachability, JSON counterexample traces, bounded exploration | **Use as the Java-free Rust cross-check** |
| Stateright 0.31 | Medium: Rust model-only types can avoid production coupling, but it requires a second Rust model | Safety and reachability; cyclic `eventually` behavior is not sufficient for a liveness gate | Keep as a future alternative, not needed for this slice |
| Rust custom BFS/`proptest` explorer | Low to medium: checker and model errors share the Rust environment | Useful smoke/property evidence, not a replacement for an independent TLA+ source | Not selected |

`tla-rs` is not a Rust translation of PlusCal. It is a Rust implementation
of a TLA+ checker, so the model remains independently expressed from the
production Rust code while the checker can run without Java. Official TLC
remains the reference until longer-term compatibility and maintenance evidence
justifies changing that status.

## Model scope and invariants

The model has one bounded write obligation and enumerates intent, home-media,
recovery, and ownership states. It checks:

- clean state requires either no outstanding work or terminal evidence;
- home mutation requires durable intent/integrity invalidation;
- uncertainty remains visible and cannot become clean or terminal silently;
- durable work remains owned by an in-flight, handoff, or terminal obligation;
- terminal state requires durable home evidence, invalidation, and clean recovery.

Bounds are `MaxDepth = 8`, one region, one home mutation, and no payload bytes,
concurrent slots, topology changes, filesystem behavior, SQLite pages, runtime
scheduling, or physical durability. The result is finite safety/reachability
evidence for this abstraction, not proof of the Rust implementation, real I/O,
or unbounded recovery progress.

## Reproducible evidence

```text
mise install java
mise exec -- java -cp /path/to/tla2tools.jar pcal.trans \
  verification/tla/RecoveryProtocol.tla
mise exec -- java -cp /path/to/tla2tools.jar tlc2.TLC \
  -config verification/tla/RecoveryProtocol.cfg \
  verification/tla/RecoveryProtocol.tla

cargo install tla-checker --version 0.6.11 --locked
tla verification/tla/RecoveryProtocol.tla \
  --config verification/tla/RecoveryProtocol.cfg --json
```

The TLC run on 2026-08-08 completed with no errors: 82 states generated, 53
distinct states, complete depth 9, and all six invariants passed.

The `tla-rs` run independently completed with `status: ok`, 53 states
explored, 81 transitions, and maximum depth 9.

A temporary seeded mutation removed the durable-intent guard from the home
write transition. Both TLC and `tla-rs` rejected it with
`MutationRequiresIntent` at depth 2. The mutant was kept outside the
repository; no bad model is part of the production or evidence source.

## Consequences

This closes the current VE-002 evidence gap without adding a Rust model crate,
runtime dependency, or production representation. Counterexamples, if later
found, must be translated into `dwv-sim` schedules and retained as normalized
regression artifacts. The next portable evidence item is VE-001 bounded
arithmetic verification; VE-003 remains gated on real concurrent executor/job
and shutdown code.

References:

- [TLA+ releases](https://github.com/tlaplus/tlaplus/releases)
- [`tla-rs` repository](https://github.com/fabracht/tla-rs)
- [`tla-checker` documentation](https://docs.rs/tla-checker/0.6.11/)
- [Stateright 0.31 documentation](https://docs.rs/stateright/0.31.0/)
