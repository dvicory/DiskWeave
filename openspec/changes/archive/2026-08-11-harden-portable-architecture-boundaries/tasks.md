## 1. Reconfirm Boundaries and Baseline

- [x] 1.1 Re-run knowledge readiness and inspect the named architecture, store, healthy-I/O, recovery, and frontend requirement owners against current `main`.
- [x] 1.2 Capture focused baseline results for service I/O, file-store exact completions, recovery SQLite evaluation, and ublk translation/fixture behavior.
- [x] 1.3 Recheck Cargo normal-dependency edges and LSP implementations/references; drop any M10 finding no longer present instead of recreating it.

## 2. Make the Store Port Usable by the Portable Service

- [x] 2.1 Change the synchronous store payload boundary to borrow exact read/write buffers while preserving child operation identity, exact completion disposition, watermarks, and persistence evidence.
- [x] 2.2 Migrate the file-store and existing fake adapter implementations; remove the concrete file-store payload registry and avoidable write-payload clone.
- [x] 2.3 Parameterize member bindings and the healthy portable service over the store port, then migrate read, write, flush, identity revalidation, and publication identity without weakening request or recovery checks.
- [x] 2.4 Replace file-specific degraded/rebuild service wrappers with store-generic wrappers and migrate operator, demo, macOS acceptance, verification, and frontend callers in one clean cutover.
- [x] 2.5 Add a deterministic non-file service adapter that proves read, write, flush, short completion, definite failure, uncertain completion, stale token, and bounded exhaustion behavior through the real service path.
- [x] 2.6 Remove the service package's normal dependency on `dwv-store-file` and verify no production service source imports file paths, file descriptors, or file-store error types.

## 3. Restore the Recovery Adapter Boundary

- [x] 3.1 Move SQLite journal, synchronization, checkpoint, connection, reset, failure, evaluation-case, matrix, and fixture types from portable recovery into `dwv-recovery-sqlite`.
- [x] 3.2 Migrate candidate-matrix and conservative reset/failure tests while retaining generic commit observations, recovery health/disposition, and mechanism-independent fault schedules in `dwv-recovery`.
- [x] 3.3 Delete the unused `RecoveryStateAdapter` marker trait and blanket implementation; migrate any newly discovered caller directly to `RecoveryStateStore`.
- [x] 3.4 Verify the portable recovery public API contains no SQLite mechanism/layout types and the full SQLite evaluation matrix retains its prior outcomes and evidence scope.

## 4. Decouple Live ublk Composition

- [x] 4.1 Replace the concrete live backend with one generic admitted-service runner that preserves normalized request translation, tag/buffer ownership, publication metadata, and shutdown behavior.
- [x] 4.2 Adapt disposable fixture open to yield its admitted service and fixture metadata to the same runner without making fixture state production authority.
- [x] 4.3 Migrate production `start`, demo serve, live discovery, cleanup, trace, and platform stubs; remove concrete SQLite selection from the live frontend type/signature.
- [x] 4.4 Run portable frontend/fixture tests and the environment-gated bounded Linux publication acceptance when its prerequisites are available.

## 5. Integrated Verification and Architecture Gate

- [x] 5.1 Run formatting, focused package tests, `cargo check --workspace --all-targets`, Clippy under the repository's configured policy, and the full workspace test suite.
- [x] 5.2 Run strict OpenSpec validation for this change and verify implementation against the proposal, design decisions, and every completed task.
- [x] 5.3 Run knowledge readiness, documentation change-boundary checks, and documentation build/clean-room gates required by changed semantic links.
- [x] 5.4 Rebuild the normal Cargo dependency graph and use LSP references/implementations to prove the service-to-file inversion, production live-frontend-to-SQLite type coupling, SQLite recovery leak, and unused marker abstraction are gone.
- [x] 5.5 Perform a final Rust architecture review for cycles, inverted dependencies, god modules, ghost abstractions, generic soup, business logic in adapters, and claim-boundary regressions; record any residual risk without broadening platform or durability claims.
