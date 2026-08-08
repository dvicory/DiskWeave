## Context

See `proposal.md` for motivation and the capability delta for observable
behavior. The architecture defines three experimental profiles: bare parity
(A), redundant session envelope (B), and B plus a coarse dirty bitmap (C).
The current workspace has parity/recovery semantics and file-backed fixtures but
no selected parity-envelope format or independent decoder.

## Goals / Non-Goals

**Goals:**

- Add a portable, bounded format seam for experimental parity-device metadata.
- Compare profiles A/B/C with the same protected geometry and fault schedules.
- Keep parity payload offsets explicit and capacity accounting exact.
- Make inspection and recovery-state interpretation independent of writer
  internals.
- Produce evidence for a provisional profile choice without freezing bytes.

**Non-Goals:**

- No required DiskWeave metadata in data members.
- No full per-extent checksum table, general WAL, allocator, or namespace data
  in the envelope.
- No Gate-H clean fast path, production format-v1 promise, or physical media
  claim.
- No changes to the XOR equation or normal service write protocol in this
  change.

## Decisions

### Add a narrow `dwv-format` crate

Create a portable workspace crate for envelope and related format codecs. Keep
it independent of frontend, SQLite, runtime, and filesystem path types. The
crate owns bounded byte-level encode/decode and semantic inspection values; the
recovery adapter remains responsible for applying those values to recovery
policy.

A new crate is preferable to putting experimental bytes in `dwv-recovery`:
recovery semantics must remain stable while the envelope layout is explicitly
replaceable. It also follows the architecture's planned `dwv-format` seam.

### Use a fixed, bounded canonical body for Profile B

Profile B uses two known envelope locations around the directly addressable
parity payload. Each copy contains a small fixed outer header and bounded
canonical body with the concrete bootstrap fields named by the handoff:
profile/features, array and parity identity, payload mapping, geometry,
topology/session generations, session state, migration state, and checksums.
It does not contain per-extent checksum records or arbitrary key/value data.

The exact offsets and reserved size remain experimental and are represented by
an explicit profile configuration rather than hidden constants in the decoder.

### Treat Profile A as a first-class comparison result

Profile A has no envelope bytes and uses the external recovery database. The
format inspector reports its lack of envelope evidence explicitly. This keeps
comparison honest and avoids making Profile B the default merely because it
has more code.

### Keep Profile C diagnostic until evidence proves its value

Profile C adds a bounded coarse dirty-region bitmap to Profile B. The simulator
and fixture comparison measure its recovery-scan benefit against extra metadata
writes, durability participants, torn-bitmap cases, and implementation
complexity. Profile C cannot authorize a clean or repair shortcut in this
change.

### Separate encoding from independent decoding

The encoder may use internal typed values, but the decoder validates bytes
independently: magic/version, feature compatibility, lengths, arithmetic
bounds, profile parameters, payload mapping, identity/generation fields,
checksums, and canonical ordering are checked before semantic values are
returned. Decoder tests include hand-built and mutated byte fixtures rather
than only round trips through the encoder.

### Conservative copy selection

Each copy is independently validated. Two matching current copies can provide
matching envelope evidence. A missing, torn, stale, clone-ambiguous, or
conflicting copy remains inspectable but resolves to `DIRTY`/`UNKNOWN` for
recovery policy. A single valid copy may be reported as discovery evidence but
cannot establish a clean fast path.

### Keep runtime integration out of the first format comparison

The existing `dwv demo inspect` path may expose envelope/profile inspection,
and the simulator/file fixtures exercise the recovery interpretation. The
healthy service write path does not begin depending on envelope bytes here.
That avoids coupling an experimental layout to normal data I/O before its
capacity and crash behavior are evidenced.

## Risks / Trade-offs

- **[Risk] Envelope offsets or reserve size become accidental compatibility.**
  → Mark all bytes FORMAT-EXPERIMENTAL, keep profile configuration explicit,
  and reject unsupported layouts rather than silently importing them.

- **[Risk] One apparently valid copy is treated as authoritative.**
  → Separate inspectability from recovery authorization; require matching
  current evidence for any stronger session interpretation.

- **[Risk] Profile C adds more durability complexity than it saves.**
  → Measure avoided verification work and write amplification; retain Profile B
  or A if the evidence does not justify C.

- **[Risk] Encoder/decoder share a bug.**
  → Use an independent decoder path, hand-authored fixtures, mutation tests,
  truncation/overflow tests, and cross-checks against the semantic model.

- **[Risk] Format code leaks SQLite or frontend assumptions.**
  → Enforce dependency direction and keep all persistence/application policy in
  adapters and recovery semantics.

## Migration Plan

No migration or compatibility promise is introduced. Existing fixtures and
ordinary parity payloads remain readable under Profile A. Profile B/C fixtures
are disposable experimental artifacts.

If a profile is rejected, delete or quarantine its envelope bytes and retain
only the comparison evidence and decoder tests. If a later change selects a
profile, it must add an explicit migration/import policy and independent
recovery tooling before any non-disposable use.
