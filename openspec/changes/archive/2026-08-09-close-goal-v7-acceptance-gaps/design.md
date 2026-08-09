## Context

`dwv-frontend-ublk` currently emits a bounded vector of flattened terminal records after service execution. The record does not preserve all normalized identity/order/durability fields, collapses semantic and kernel terminal states, cannot be replayed, and reserves trace capacity after mutation. The queue also releases a tag before the kernel completion command returns. The portable service and existing `dwv demo file trace-replay` already establish the reusable semantic patterns; Linux-specific completion mechanics must remain in the adapter.

## Goals / Non-Goals

**Goals:**

- Make trace capacity part of pre-mutation admission.
- Preserve raw submission, normalized request, semantic terminal class, and Linux completion separately in a versioned bounded document.
- Deterministically replay translation and completion mapping without opening backing storage.
- Retain tags until kernel completion succeeds; classify failed completion as abandonment/reconciliation.
- Exercise unavailable/denied probe results and incomplete shutdown through pure deterministic seams, plus live Linux checks where the kernel is required.

**Non-Goals:**

- Re-execute payload I/O during frontend trace replay.
- Change portable parity, recovery, store, or normalized-request contracts.
- Add async frameworks, a daemon protocol, stable fixture formats, or production recovery/durability claims.

## Decisions

### One adapter-owned trace document

Add a `dwv.ublk.trace.v2` document in `dwv-frontend-ublk` containing capacity, topology epoch, published bounds, and ordered records. Each record contains the original kernel request, an optional normalized snapshot, semantic terminal result, and explicit kernel completion. Symbolic enums avoid raw pointers and payloads. Serialization rejects unknown fields and import rejects oversized bytes, records beyond the configured bound, non-monotonic sequences, invalid translation, or divergent completion mapping.

### Reserve evidence before semantic admission

`TraceLog::reserve` appends an empty bounded slot before tag or service admission. `complete` fills that exact slot. If no trace slot exists, the request receives resource exhaustion without semantic execution. A known completion failure fills the reserved slot with an abandoned terminal; any still-incomplete reservation prevents clean serialization and requires reconciliation. The frontend writes every complete trace document before deciding whether shutdown is clean, so exhausted or abandoned evidence survives a reported reconciliation failure.

### Kernel completion owns tag release

The queue reserves trace and tag state, executes the portable request, maps the semantic result, submits the kernel completion, then releases the tag only after a safe completion boundary. `libublk` 0.4.6 implements completion as a combined commit-and-next-fetch operation. A successful return is safe. `QueueIsDown` is also safe only when shutdown was explicitly requested after consumers unmounted and no queue failure was observed: in that case the abort applies to the next fetch, while successful unmount proves the preceding completion was observed. Every other completion failure leaves the tag active, records abandonment, and forces reconciliation.

### Pure classification seams

Probe control classification accepts an `io::Result` plus module-presence fact, allowing absent and permission-denied cases to run without host mutation. Completion mapping and shutdown cleanliness are pure functions used by the Linux path and deterministic tests. This avoids test-only production flags or a general fault-injection framework.

### CLI replay is validation-only

`dwv demo disk trace-replay --trace <path>` imports the adapter trace and replays translation/completion mapping without fixture or device access. The live script copies the retained trace into checked evidence, invokes replay, and records its digest, schema, count, and outcome.

## Risks / Trade-offs

- Completion-command success is the narrow observable kernel boundary exposed by `libublk`; it does not prove application consumption or durability.
- A trace-bound refusal cannot append another record because the bound is already full; shutdown evidence records the bound exhaustion count separately.
- Validation-only replay proves deterministic adapter translation/mapping, while existing service/model tests remain the evidence for portable payload semantics.
