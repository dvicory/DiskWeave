## 1. Trace model

- [x] 1.1 Add the pure `dwv-trace` crate with bounded serde event and fixture types.
- [x] 1.2 Implement canonical JSON encoding, strict validation, schema-0 migration, and bounded failing-prefix minimization.
- [x] 1.3 Add model tests for round trips, privacy-safe fields, oversized/malformed input, migration, and event bounds.

## 2. Backend replay

- [x] 2.1 Build the deterministic symbolic workflow covering I/O, durability, recovery, checksum, degraded-read, rebuild, and repair events.
- [x] 2.2 Replay write/read/flush operations through `dwv-sim` and copied regular files through `FileStore` without mutating the caller fixture.
- [x] 2.3 Normalize payload/parity/integrity/recovery/terminal summaries and emit the shortest failing prefix on divergence.

## 3. CLI integration

- [x] 3.1 Add `demo trace-export`, `demo trace-render`, and `demo trace-replay` with bounded root-relative trace paths and machine-readable outcomes.
- [x] 3.2 Add CLI coverage for export/render/replay, malformed and oversized traces, schema migration, and source-fixture preservation.
- [x] 3.3 Record portable claim boundaries and explicit exclusions for Linux, bridge, physical durability, and unsupported backend semantics.

## 4. Verification and completion

- [x] 4.1 Run focused trace/model/file-backed tests and the full workspace test/lint/format checks.
- [x] 4.2 Run strict OpenSpec validation and end-to-end CLI replay evidence.
- [x] 4.3 Record executable commands, representative outputs, and deferred boundaries in verification documentation.
- [x] 4.4 Archive only after implementation, evidence, and the decision record agree.
