# How parity helps—and where it stops

For the current single-XOR profile, parity is computed byte by byte. With two data
bytes:

```text
data0   3C
data1   A5
parity  99    because 3C XOR A5 = 99
```

If `data1` is the one known missing member, DiskWeave can calculate:

```text
3C XOR 99 = A5
```

That establishes **algebraic reconstructability**: the available bytes and coding
geometry produce one candidate.

## Why the equation is not enough

Now suppose `data0` was silently corrupted from `3C` to `3D`. The same calculation
produces `A4`. If DiskWeave writes `A4` as the replacement, the new set can still
be made to satisfy an XOR equation while containing the wrong historical data.

Parity therefore answers:

> What value makes these currently observed members satisfy the coding equation?

It does not independently answer:

> Are these the correct and current members, and are their bytes authoritative?

That second question needs identity, topology, generation, checksum, recovery, and
persistence evidence.

## Follow four different evidence journeys

The operation is chosen by what is known, not merely by the fact that XOR can
produce a byte. Start from the healthy set above:

```text
data0 = 3C    data1 = A5    parity = 99
```

### Journey 1: serve one read while `data1` is unavailable

The topology identifies `data1` as the one missing member. `data0` and parity
have the expected identities and generation, so the requested byte can be
reconstructed as `3C XOR 99 = A5`.

A **degraded read** returns that `A5` to this caller. It does not write `A5`
anywhere, declare the member repaired, or make a durable recovery claim. If two
members are unavailable, or the surviving identities do not establish one
eligible erasure, the read stops instead of guessing.

### Journey 2: replace the unavailable member

An operator supplies a distinct empty replacement while the source topology is
quiesced. An **offline rebuild** repeats reconstruction over bounded ranges in
increasing order. For each range it:

1. reads the surviving source and parity bytes under degraded-read eligibility;
2. reconstructs the missing bytes in memory;
3. writes only the distinct replacement;
4. reads those bytes back and checks both exact equality and the parity equation;
5. durably flushes that replacement range;
6. only then advances the durable rebuild cursor.

If the process stops after the replacement flush but before the cursor update,
restart safely repeats that range. Advancing the cursor before durable flush
would be unsafe because recovery could skip bytes that never reached durable
media. Surviving data, parity, and the old missing-member path are never rebuild
targets. Only final whole-member verification permits promotion.

### Journey 3: every member reads, but the equation fails

Suppose the observed bytes are now:

```text
data0 = 3D    data1 = A5    parity = 99
```

There is no known erasure. XOR offers three internally consistent stories:

```text
data0 should be 3C    if data1 and parity are authoritative
data1 should be A4    if data0 and parity are authoritative
parity should be 98   if both data members are authoritative
```

The equation cannot choose among them. **Verified repair** is allowed only when
independent identity and integrity evidence identifies one bad protected value
and one authoritative replacement. Repair then uses a separate target, verified
readback, and persistence evidence. A parity mismatch by itself is a report, not
repair authority.

### Journey 4: authority cannot be recovered

If identities, generations, or integrity evidence cannot establish which
observed values are authoritative, DiskWeave cannot honestly call any candidate
a repair. **Rebaseline** is an explicit operator decision to adopt a new clean
lineage. It records new authority; it does not prove which historical bytes were
correct.

## Compare authority, result, and non-claim

| Operation | Immediate result | What it deliberately does not claim |
| --- | --- | --- |
| Degraded read | Returns one eligible reconstructed range. | That any member was repaired or durably changed. |
| Offline rebuild | Creates and verifies a distinct replacement. | That surviving sources were rewritten or ambiguous corruption was solved. |
| Verified repair | Replaces one independently identified bad value. | That parity alone identified the bad member. |
| Rebaseline | Starts a new operator-chosen clean lineage. | That the prior historical state was recovered. |

**Next:** {doc}`04-boundary-and-demo` runs the implemented disposable workflow and
states its evidence boundary.

## Traceable requirements

```{needlist}
:filter: "type == 'req' and capability in ['parity-verification-repair', 'checksum-scrub-verified-repair', 'degraded-read-offline-rebuild', 'checksum-plane']"
```

**Provenance:** `req.architecture-contract.recovery-and-repair-never-promote-algebraic-possibility-to-authority`; `req.checksum-plane.invalidation-precedes-data-parity-write`; `req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback`; `req.degraded-read-offline-rebuild.known-erasure-reads-reconstruct-exact-requested-bytes`; `req.degraded-read-offline-rebuild.offline-rebuild-writes-only-a-separate-replacement-target`; `req.parity-verification-repair.mismatch-classification-requires-independent-evidence`; scenario `scenario.normalized-recovery`.

