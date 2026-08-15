## Context

The existing Linux frontend keeps trace reservations in a bounded append-only collection. `trace.rs` reserves a position before the Linux request is translated or dispatched, fills it with terminal evidence, and refuses once 4,096 positions have ever been reserved. Export already rejects an incomplete reservation, sorts retained records by sequence, and replays normalized requests without backing I/O. The queue path in `linux.rs` owns the ordering between trace completion, kernel completion delivery, and tag/resource release. See `proposal.md` for the motivation and product boundary.

The design must preserve the existing operation-slot, tag, buffer, child-operation, durability, recovery, abandonment, and bounded-admission ownership contracts. A retired trace record is diagnostic retention only; retirement is not permission to release a live operation resource. The trace remains an experimental frontend artifact, not a durable history or a stable compatibility format.

## Goals / Non-Goals

**Goals:**

- Replace the lifetime-only trace ceiling with a fixed-capacity, renewable retained window while keeping reservation before semantic admission and protected mutation.
- Make safe retirement an explicit state transition that is possible only after terminal evidence and all owning contracts have completed required reconciliation.
- Keep retained original sequence numbers and deterministic normalized replay when the retained prefix begins after sequence one.
- Disclose retired prefixes and partial-session coverage at export, and expose bounded retired-record accounting with an explicit saturation state.
- Preserve conservative incomplete-reservation behavior through shutdown and crash paths.
- Keep the implementation small and reversible: a ring/deque-like index, a bounded slot collection, or an equivalent structure may be selected during implementation as long as these observable invariants hold.

**Non-Goals:**

- Durable trace segmentation, an unbounded audit/history store, or retention beyond the fixed 4,096-record window.
- A generic retention, garbage-collection, or lifecycle framework for other capabilities.
- A stable trace-schema, import-compatibility, or long-term serialization guarantee; an old experimental schema may be explicitly migrated or refused.
- Changing normalized request semantics, operation-slot lifetime authority, frontend abandonment meaning, recovery authority, or the general security boundary.
- Reconstructing retired records after restart or claiming a complete-session replay from a partial retained window.

## Decisions

### 1. Keep one semantic owner and compose with existing owners

The changed requirement remains `req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe`. It owns the Linux adapter's bounded trace admission, retirement eligibility, retained-window disclosure, and adapter-level refusal. Other requirements remain owners of their own policies:

| Current requirement | Semantic rule used here | Relationship | Consequence |
| --- | --- | --- | --- |
| `req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe` | Fixed-capacity trace reservation, safe terminal-only retirement, retained-window export/replay, and explicit bounded refusal | Canonical owner; MODIFIED with semantic broadening | Stable `req.*` identity remains; the lifetime trace ceiling becomes renewable without changing resource ownership. |
| `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` | Validate and preserve each admitted request and terminal mapping | Refinement/consumer | Replay reuses this contract; the trace requirement does not redefine request meaning or unsupported-operation behavior. |
| `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | Operation slots own backend lifetimes, tags, buffers, child identities, watermarks, and reconciliation until safe reclamation | Required owner | A trace slot cannot become reclaimable merely because a trace record has terminal-looking fields. |
| `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | Abandonment removes delivery interest without cancellation or early reclamation | Required owner | Abandoned records remain evidence and their resources remain unavailable until reconciliation. |
| `req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission` | Validate and bound hostile inputs and resource admission before unbounded work | Constitutional constraint | Trace renewal never truncates, loops, allocates unboundedly, or silently deletes history to admit work. |

No second requirement owns the rolling-retention predicate. The Linux requirement composes the normalized-request and operation-lifetime contracts rather than restating their full state machines.

### 2. Separate terminal evidence from reclaimable eligibility

The trace log needs three externally meaningful slot states even if implementation stores them differently: available, reserved/incomplete, and terminal-but-retained. A terminal record becomes eligible for retirement only after its operation owners have completed reconciliation and released the resources they own. The implementation SHALL make that ordering explicit, for example by finalizing a reservation only after the tag/operation owner signals safe release or by retaining a separate not-yet-reclaimable terminal state. The exact method and container are implementation choices.

At reservation time, select an available slot first; otherwise select the oldest terminal-but-reclaimable record by original sequence. Retire that record, increment the bounded retired-record accounting, and issue the new reservation. Never select a reserved, active, abandoned-but-unreconciled, or terminal-but-owned record. If no eligible record exists, return explicit bounded resource exhaustion before semantic admission or protected mutation and leave every existing record and owner unchanged.

This ordering closes the race in which trace evidence is written before a tag or operation owner has released its resource. Reusing storage must not make stale completions address a new generation.

### 3. Preserve original sequence and retained-window replay

Keep the existing monotonic sequence source for request ordering. Retained records keep their original sequence values; retirement removes records from the retained window but does not renumber the survivors. Replay validates strict ordering among records present in the retained window and accepts any positive first sequence, including a first sequence greater than one. It does not infer divergence from an absent retired prefix and does not execute backing I/O.

Export constructs a deterministic ordered view of currently retained terminal records. If a reservation is incomplete, export remains reconciliation-required rather than filling it with a synthetic result. Export metadata or disposition explicitly identifies that an earlier prefix was retired and that replay covers only the retained partial session. The metadata is versioned through the existing experimental schema boundary, not promised as a stable long-term format.

### 4. Keep retired accounting bounded and independent from admission

Track retired completed records separately from refusal/exhaustion outcomes. The retired count uses a bounded representation with a distinct saturated indication. Once saturated, further retirement still succeeds whenever a safe terminal record exists; the count does not control admission and cannot create a second lifetime ceiling. Export/status reports the saturated state so operators cannot mistake the count for a complete historical total.

If the existing experimental `exhausted_records` representation cannot distinguish retired records, refusal, and saturation without ambiguity, the implementation may perform an explicit schema migration or reject that schema. It must not silently reinterpret an old export while claiming compatibility.

### 5. Preserve conservative shutdown and crash behavior

Normal shutdown continues to close admission, stop the queue, and attempt trace export before reporting a clean result. Any reservation left incomplete causes an explicit incomplete or reconciliation-required outcome; it is not retired or made reusable. A process crash cannot manufacture terminal evidence, and a subsequent inspection/restart path must not treat an incomplete reservation or missing terminal evidence as a clean complete-session trace. No durable segment or crash journal is introduced by this change.

### 6. Keep the implementation boundary narrow

The likely edits are confined to the existing trace log and its Linux queue/export call sites. The data structure may be a ring/deque-like collection, a bounded vector with a logical head, or another equivalent representation; implementation should choose the smallest structure that enforces the state and ordering rules. Avoid a generic abstraction, new dependency, or cross-capability retention API.

## Risks / Trade-offs

- **Reclamation race:** Terminal trace evidence can be observed before operation/tag ownership is safe. A separate reclaimable transition and focused interleaving evidence are required; a plain `Option<TraceRecord>` check is insufficient.
- **Partial replay interpretation:** A valid replay of retained records can look clean while omitting the retired prefix. Export must carry an explicit partial-session disposition, and consumers must not infer complete-session coverage.
- **Experimental schema drift:** Adding window and saturation state can make old exports ambiguous. Explicit migration or refusal is safer than a compatibility promise and keeps the format boundary reversible.
- **Saturated accounting:** Saturation loses the exact lifetime total by design. The explicit saturation indicator preserves that limitation while ensuring accounting cannot stop service.
- **Crash evidence:** In-memory bounded retention cannot recover an incomplete reservation after process loss. Conservative reconciliation is intentional; durable trace history is excluded from this change.
- **Container choice:** A ring/deque may reduce scans, while a bounded vector is simpler. The design leaves this reversible because the semantic contract is slot eligibility and sequence order, not a particular data structure.
