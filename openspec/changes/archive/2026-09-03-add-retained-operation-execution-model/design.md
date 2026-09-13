## Context

The current portable service retains `WriteDriver` state after `submit_write` returns. `drive_write` emits one normalized physical action; `accept_write_work` establishes the exact service acceptance boundary; the executor may execute later; and `deliver_write_result` resumes the retained driver. `OperationSlotTable` already owns generation-qualified child identities, completion, reconciliation, and resource lifetime. Existing tests cover these operations separately but do not provide one bounded model-to-production correspondence surface.

See `proposal.md` for motivation. This design follows the reviewed model boundary and does not replace `RecoveryProtocol`, coded-range/CLEAN authority, persistence evidence, or media/crash simulation.

## Goals / Non-Goals

**Goals:**

- Keep driver-local work state distinct from slot-owned child submission state.
- Exercise a six-child protected-write pipeline with exact operation, generation, store, incarnation, topology, range, and action identity.
- Exercise a retained-driver slot projection with multiple accepted children, delayed and reverse-order execution/result delivery, partial completion, uncertainty, failure, and one finite duplicate observation.
- Use the full wrapper's fan-in and release observations as bounded model evidence; prove production correspondence only through the delegated `Reclaimable` boundary, where reconciliation remains owner-local and service release is explicitly outside the claim.
- Record the model's exact delegation boundary and production correspondence without claiming that model input production is proof of upstream owner correctness.

**Non-Goals:**

- No change to `WriteDriver`, `OperationSlotTable`, backend scheduling, or persistent formats.
- No model of topology discovery, request/resource limits, buffers, frontend tags, drain policy, watermark admissibility, persistence-engine behavior, recovery `CLEAN`, or physical durability.
- No claim that the current service emits several physical calls concurrently; the fan-in projection is entered from the same retained driver to exercise the slot owner independently.
- No canonicalization of the finite analysis profiles or Connect trace as a replacement for the model relation.

## Decisions

1. **Separate the delegated core from the correspondence wrapper.** `PortableOperationExecutionCore.qnt` owns one retained-operation child-correlation relation from registered work through `SlotReclaimable`; it has no driver cursor, physical scheduler, transaction marker, release authorization, or post-reclaim state. `PortableOperationExecution.qnt` retains the full two-mode driver/fan-in composition only as evidence.
2. **Keep the wrapper's two ownership projections explicit.** Its `DriverMode` follows one retained service driver through the ordered physical pipeline. Its `SlotFanInMode` joins the same retained operation to independently correlated child fan-in and does not claim concurrent `WriteDriver` scheduling.
3. **Represent two ownership layers in the core's children.** `WorkState` tracks planned, emitted, accepted, executed, delivered, or refused driver work. `SlotChildState` tracks registered, accepted, terminal, or refused slot ownership. Emission changes only driver-local state; exact acceptance changes the slot state.
4. **Make identity a typed model input.** Every normal acceptance, execution, delivery, and duplicate action takes a `ChildIdentity` containing operation generation, store, incarnation, topology, range, and action kind. A mismatched input disables the action before its next-state assignment; Connect negative probes exercise the real child and completion identity rejection paths, with the current slot snapshot unchanged.
5. **Make execution explicit.** An accepted child must pass through execution before delivery. This keeps the core from treating normalized result delivery as proof that no earlier physical effect occurred.
6. **Keep uncertainty aggregate and sticky.** A final slot projection remains `SlotCompletionUncertain` whenever the delivered child or any retained terminal child is uncertain. A short, failed, or uncertain delivery also refuses every sibling not yet accepted; accepted siblings remain owned. Reconciliation is idempotent for the recorded outcome and rejects an outcome change. A later child acceptance preserves `SlotPartiallyCompleted` or `SlotCompletionUncertain` rather than resetting aggregate state.
7. **Use compact formal profiles and richer correspondence.** Two-child wrapper profiles make bounded Apalache checks complete within the short execution envelope. The core profile and Connect wrapper bind the six-child delegated production domain; the difference is a declared evidence bound, not a semantic shortcut.
8. **Keep release and OpenSpec delegation narrow.** Core reconciliation reaches `SlotReclaimable` and stops. Fixed protected-write ordering, `TransactionMachine` semantics, explicit release-authorization and transaction-release observations, post-`Reclaimable` cleanup, and the surrounding operation-slot, request, resource, persistence, recovery, lifecycle, frontend, and implementation-conformance requirements remain canonical outside the delegated core.

## Risks / Trade-offs

- The correspondence wrapper's fixed six-child sequence is intentionally a protected-write profile, not an arbitrary scheduler. Evidence must retain that bound.
- The fan-in projection uses a test-only slot-owner path because the current driver serializes physical work. It proves slot correlation and retention, not concurrent service scheduling.
- Compact formal profiles do not exhaust the six-child Connect path. Deterministic six-child correspondence and explicit non-claims are required to avoid overstating the formal result.
- External owner observations are consumed, not generated, by the model. Their production remains separately reviewed through the real Rust owners and service path.
- Model-to-Rust mapping failures are treated as semantic/architecture signals; no adapter shim or fabricated observation is allowed to make a trace pass.
