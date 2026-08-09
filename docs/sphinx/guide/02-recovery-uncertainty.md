# What recovery can and cannot know

After restart, memory is gone. Recovery can use only durable payload bytes,
durable recovery records, stable member identities, and evidence that is still
valid for the same topology and recovery generation.

Continue the four-byte example from the previous chapter. Three broad states are
possible:

| Durable facts after restart | Safe conclusion |
| --- | --- |
| No durable intent and old payload/parity remain | The old state is authoritative. |
| Durable `DIRTY`/`STALE` intent exists, but no completed checkpoint | The range needs recovery; do not call it clean. |
| Payload/parity fences and the matching checkpoint are durable | The checkpointed state may become authoritative for that exact range and generation. |

These are examples of evidence patterns, not permission to guess from file
timestamps or elapsed time.

## Why identity and generation matter

Evidence is meaningful only for the objects it describes. Before using it,
DiskWeave checks:

- the array topology epoch;
- each member's stable identity and coding position;
- the recovery generation;
- the exact byte range;
- the fence or checksum evidence attached to that range.

A checkpoint from an older generation cannot make a newer write clean. A
replacement disk that happens to occupy the same path is not automatically the
same member. A checksum for one extent says nothing about an adjacent extent.

## Availability is not repair authority

Suppose one data member is known missing:

- A **direct read** uses the selected member when it is present and readable.
- An authorized **degraded read** reconstructs only the requested range and does
  not modify protected storage.
- An **offline rebuild** reconstructs bounded ranges into a different, empty
  replacement target.
- A **repair** changes protected state and therefore needs stronger independent
  evidence and explicit authority.
- A **rebaseline** starts a new authority lineage when history cannot be recovered;
  it does not pretend the old state was proven.

The normalized recovery fixture exercises these distinctions in one deterministic
sequence. Its successful terminal state proves that sequence under its modeled
faults; it does not prove that every possible interrupted history is known.

## Recovery's practical checklist

For each range, ask in this order:

1. Which exact topology, identities, and generation does the evidence describe?
2. Which bytes and records are durable rather than merely process-visible?
3. Is the range directly readable?
4. If not, is there one explicitly authorized reconstruction?
5. If writing a replacement or repair target, has readback, parity verification,
   and a durability fence completed before promotion?

When an answer is missing or contradictory, the safe result is refusal or an
explicit uncertain/degraded state—not a clean guess.

**Next:** {doc}`03-parity-and-integrity` shows why a solvable parity equation is
necessary but not sufficient evidence.

## Traceable requirements

```{needlist}
:filter: "type == 'req' and capability == 'recovery-state-semantics'"
```

**Provenance:** `req.recovery-state-semantics.recovery-transactions-are-generation-checked-and-atomic`; `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence`; `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`; `req.normalized-trace-replay.replay-is-deterministic-across-portable-backends`; scenario `scenario.normalized-recovery`.

