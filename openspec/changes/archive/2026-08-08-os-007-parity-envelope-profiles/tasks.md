## 1. Format seam and experimental profiles

- [x] 1.1 Add the portable `dwv-format` workspace crate with dependency-direction checks and bounded format error types.
- [x] 1.2 Define experimental Profile A/B/C configuration values, exact protected-capacity accounting, and explicit parity-payload mapping.
- [x] 1.3 Define bounded semantic envelope metadata for identity, profile/features, geometry, topology/session generations, migration state, and session state.

## 2. Encoding and independent decoding

- [x] 2.1 Implement bounded canonical Profile B copy encoding with explicit offsets, lengths, and checksums.
- [x] 2.2 Implement Profile A inspection and Profile C bounded bitmap representation without adding per-extent checksum tables.
- [x] 2.3 Implement an independent decoder that validates versions, feature compatibility, lengths, overflow, checksums, canonical fields, and payload coverage.
- [x] 2.4 Add hand-authored valid, truncated, oversized, unknown-feature, checksum-invalid, and mutated envelope fixtures.

## 3. Copy selection and recovery interpretation

- [x] 3.1 Implement deterministic copy assessment for matching, missing, torn, stale, clone-ambiguous, and conflicting copies.
- [x] 3.2 Map envelope/session evidence to conservative `CLEAN`, `DIRTY`, and `UNKNOWN` outcomes without enabling the Gate-H fast path.
- [x] 3.3 Add interrupted profile migration behavior that preserves the prior interpretable profile and rejects incomplete new evidence.

## 4. Simulator and file-backed evidence

- [x] 4.1 Add simulator schedules for session transitions, torn copies, disagreement, crash, and interrupted migration.
- [x] 4.2 Compare Profile A/B/C verification work, metadata writes, recovery evidence, and bounded resource/write-amplification costs.
- [x] 4.3 Add disposable file-backed parity fixtures with independent inspection and exact-capacity boundary cases.
- [x] 4.4 Extend the existing `dwv demo inspect`/status evidence without changing ordinary data-member payloads or the XOR service path.

## 5. Decision and verification

- [x] 5.1 Record the measured provisional profile choice or explicit fallback and its exit/migration policy in an ADR.
- [x] 5.2 Record executable commands, observed results, resource bounds, and portable/macOS claim boundaries in verification documentation.
- [x] 5.3 Run focused format, simulator, file-backed, dependency-direction, workspace, and strict OpenSpec validation checks.
- [x] 5.4 Verify the implementation against every OS-007 scenario before marking the change complete or archiving it.
