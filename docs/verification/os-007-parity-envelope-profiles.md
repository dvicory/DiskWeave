# OS-007 verification: parity-envelope profiles

**Observed:** 2026-08-08 on the portable workspace and disposable file-backed
fixture path.

## Checks

- `cargo test --workspace` — 201 passed, 1 ignored.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo run -q --bin dwv -- demo init --root "$ROOT"` followed by
  `cargo run -q --bin dwv -- demo inspect --root "$ROOT"` — passed for a fresh
  disposable fixture.
- `openspec validate os-007-parity-envelope-profiles --strict --json` — passed
  before implementation and is rerun as part of final change verification.

## Observed fixture evidence

For the default 16,384-byte protected range, `demo inspect` independently
inspected two 4,096-byte Profile B copies and reported:

- `assessment: Clean`;
- `evidence_strength: MatchingCopies`;
- `metadata_bytes: 8192`;
- `payload_offset: 4096`;
- `payload_length: 16385`;
- `clean_recovery_authorized: false`.

The same inspection compared all three profiles and accepted these layouts:

| Profile | Metadata bytes | Payload capacity |
|---|---:|---:|
| BareParity | 0 | 24,577 |
| RedundantEnvelope | 8,192 | 16,385 |
| EnvelopeBitmap | 8,193 | 16,384 |

The simulator tests cover dirty-before-mutation ordering, missing/torn and
conflicting copies, crash after dirty state, fence/checkpoint gating, bounded
bitmap cost reporting, and interrupted migration retaining the prior profile.
The format tests cover valid decoding, truncation, oversized input, checksum
mutation, unknown required features, exact capacity, and copy selection.

## Claim boundary

This evidence covers portable Rust semantics, bounded byte inspection,
deterministic simulator behavior, and ordinary macOS-hosted files. It does not
claim FSKit, macFUSE, DiskImages, Linux frontend, physical power-loss
 durability, Gate-H certification, degraded writes, or a stable persistent
format. Envelope bytes remain disposable until a later change establishes a
format and migration boundary.
