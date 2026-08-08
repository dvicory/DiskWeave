## Context

The repository already has a deterministic volatile-media simulator, a
file-backed `FileStore`, and a transaction-level in-memory trace, but none of
them share a serialized semantic trace contract. The CLI is a binary crate with
an established disposable fixture and JSON result envelope. See `proposal.md`
for motivation and the capability contract.

## Goals / Non-Goals

**Goals:**

- Keep the trace model dependency-light and portable, separate from filesystem
  and simulator implementation types.
- Make canonical JSON validation, one-version migration, bounded event storage,
  and prefix minimization testable without a backend.
- Use one deterministic symbolic workflow so simulator and copied regular-file
  fixtures execute the same write/read/flush operations and produce comparable
  normalized summaries.
- Keep existing CLI commands and JSON/exit-code conventions intact.

**Non-Goals:**

- Serializing raw simulator deliveries, payloads, file paths, SQLite rows, or
  OS-specific identities.
- Replacing the existing transaction trace, simulator, recovery store, or
  repair implementation.
- Claiming replay equivalence for Linux, a live bridge, physical media, or
  workflows whose backend-specific semantics are not represented by the trace.

## Decisions

1. **Add a pure `dwv-trace` crate.** It owns `Trace`, bounded event types,
   canonical JSON, validation, schema-0 migration, rendering data, normalized
   summaries, and failing-prefix minimization. It depends only on `serde` and
   `serde_json`; it does not depend on `dwv-sim`, `dwv-store-file`, or the CLI.
2. **Use symbolic payload patterns.** A write records offset, length, and a
   deterministic pattern descriptor (`zeroes` or seeded counter), never bytes.
   Store targets and outcome classes are enums/integers, not paths or handles.
3. **Use three explicit trace commands.** `demo trace-export` writes a
   canonical trace, `demo trace-render` validates and returns a bounded event
   summary, and `demo trace-replay` validates then runs the trace against both
   backends. `--trace` remains root-relative and defaults to `trace.json`.
4. **Replay only through existing seams.** The simulator receives a generated
   `Schedule` with write, flush, and read submissions; the file backend uses a
   copied disposable fixture and `FileStore` exact-range operations. Semantic
   recovery/checksum/degraded/rebuild/repair events update the normalized
   outcome state but do not invent backend writes.
5. **Compare digests and semantic state, not object identity.** Each backend
   reports BLAKE3 payload/parity digests plus integrity state, recovery
   generation, and terminal outcome. A mismatch is reported with the shortest
   failing event prefix; no backend is mutated in the caller's fixture root.
6. **Migrate only schema 0 to schema 1.** Schema 0 has the same event payload
   with top-level `size`/`seed`; migration supplies the current fixture object.
   Future or unknown versions are rejected before any replay work.

## Risks / Trade-offs

- The first replay fixture covers the portable semantic vocabulary with one
  deterministic workflow rather than every future service operation. Unsupported
  semantics remain explicit trace events and cannot be mistaken for backend
  equivalence.
- Copying a regular-file fixture adds setup I/O, but it prevents replay from
  mutating the user's source fixture and keeps the proof portable.
- A pure JSON trace is larger than a binary log, but the 1 MiB bound, canonical
  encoding, and human-readable rendering make review and minimized regressions
  practical.
