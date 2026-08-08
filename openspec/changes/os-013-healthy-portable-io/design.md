# OS-013: Healthy portable read/write path

## 1. Architecture decisions and target gate

Implement the handoff Phase 1 OS-013 row and Gate D/Phase 1 evidence under D-001–D-003, D-007–D-010, D-014, D-017, D-018, and D-021. P-009 is not selected here; the result is the portable reference required before OS-020.

## 2. Concrete outcome

A small service can open ordinary file-backed data/parity members, process normalized requests, and produce reference-equivalent bytes and conservative recovery state on macOS.

## 3. Prerequisites

Integrate OS-006/008/010/011/012 contracts and archived OS-001–005 behavior. If a dependency is still incomplete, keep the service behind a compile-time seam and do not weaken acceptance.

## 4. Exact scope and non-scope

Implement orchestration and healthy single-parity I/O only. Degraded/rebuild/repair, bridge APIs, physical durability, and Linux frontend/executor work remain separate.

## 5. Semantic APIs and contracts

Split orchestration into request routing, range planning, parity I/O, operation slots, and service lifecycle modules. Reuse normalized core types and typed evidence. The service accepts an injected store/recovery/transaction implementation so tests do not require a particular runtime or database.

## 6. State ownership and lifecycle

Request/slot state is ephemeral; topology/recovery state is durable semantic state; file stores own payload bytes. An `Open` service validates identities/capabilities, a `Serving` service admits only valid topology, and `Recovering`/`Blocked` states refuse unsafe writes.

## 7. Persistent-state impact

Register new portable crates and retain independent `dwv-store-file` and transaction modules. Keep member files data-only. Add only semantic recovery/checksum records through their stores and export APIs.

## 8. Irreversible and durability boundaries

Use the OS-008 action order exactly: acquire, intent, read/compute/write, flush/fence, checkpoint/clear, release. Store completion evidence is carried through each layer; no layer upgrades `VolatileOrUnknown` to durable by assumption.

## 9. State and sequence diagrams

```text
Open -> Probe/identity -> Assemble -> Serving
Serving + Read  -> slot -> range -> data read -> completion -> release
Serving + Write -> slot -> intent -> RMW/parity writes -> fence -> checkpoint -> release
Any uncertainty/failure -> Dirty/Recovering or Blocked
```

Normalized trace fixtures are the integration oracle and feed OS-024 later.

## 10. Concurrency and resource rules

Use bounded slot and buffer pools; reserve capacity before accepting a request. Split ranges at logical block, RMW, lock, dirty, checksum, and maximum-transfer boundaries. Acquire parity/range locks in a stable order and release only after the logical transaction is terminal.

## 11. Failure matrix

The implementation maps exact store completion dispositions to service state without retrying uncertain non-idempotent writes. All post-intent failures retain dirty evidence. Aliases, identity ambiguity, stale topology, capability changes, and lock conflicts fail closed.

## 12. Deterministic simulator cases

Generate workloads from one range through bounded mixed reads/writes/flushes and inject a cut after every transaction action and child completion. Compare final data, parity, recovery, checksum, and resource obligations with the reference model.

## 13. Property, model, and fuzz tests

Add a workload model, request/range fuzzing, child-completion permutations, slot-generation reuse tests, and resource-bound assertions. Verify incremental XOR equals full parity recomputation for every generated write.

## 14. Integration tests

Create temporary regular files on macOS, assemble a small array, run randomized workloads, reopen after clean stop, and delete/rebuild disposable control state. Test ordinary data-file reads independently. Keep bridge and APFS mounting scenarios for OS-020/021/022.

## 15. Observability, security, and operator behavior

Provide bounded structured traces and metrics for request lifecycle, child ranges, parity RMW, fence evidence, recovery generations, checksum coverage, slot use, and lock waits. Normalize paths only at the adapter boundary and reject unsafe aliasing/identity ambiguity.

## 16. Performance and resource bounds

Establish bounded defaults for slots, buffers, range splits, and checksum jobs. Measure correctness-reference overhead, parity amplification, fence latency, and memory. Do not optimize across semantic boundaries until traces and resource failures are stable.

## 17. Executable acceptance criteria

Run the OS-013 randomized/reference/restart/abandonment matrix, macOS file-backed integration tests, full workspace tests, format, dependency inspection, and OpenSpec validation. Record the portable-demo limitation: no physical power-loss or FSKit production claim.

## 18. Forbidden outcomes

No unsafe write after uncertain intent, no false clean, no data-member metadata, no slot/buffer reuse before terminal completion, no aliasing, and no Linux/macOS adapter type in portable semantic contracts.

## 19. Migration and compatibility consequences

The service consumes versioned normalized requests and semantic recovery/checksum state. Future frontends must adapt to it. A changed parity profile or checksum set requires explicit rebuild/migration; no silent image reinterpretation.

## 20. Next OpenSpecs unlocked

OS-014–017 can build verification/recovery on the end-to-end path. OS-020 can probe FSKit/DiskImages against the normalized service, and OS-024 can replay its traces. Linux frontend work remains deferred.
