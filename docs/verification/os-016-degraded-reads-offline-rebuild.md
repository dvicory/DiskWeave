# OS-016 degraded reads and offline rebuild verification

## Claim boundary

This evidence covers portable, read-only known-erasure reconstruction and a
separate-target, resumable single-XOR offline rebuild over ordinary macOS
regular/sparse files. It does not claim degraded writes, online rebuild, P/Q,
two erasures, automatic EIO classification, a certified Gate H envelope fast
path, a live APFS bridge, Linux frontend behavior, or physical-device
power-loss durability.

The replacement is directly readable after every `FileStore` is dropped. A
32 MiB detached APFS disk image was also rebuilt through the same engine,
attached independently read-only with `hdiutil` after all stores closed, and
its marker file matched byte-for-byte. Degraded APFS access through a live
DiskWeave bridge remains OS-022 because no buildable and installed bridge
candidate is available in this host session.

## Handoff mapping

- Section 7.6: verified replacement preparation retains the stable slot and
  coding position while changing assignment instance, assignment generation,
  store mapping, and topology epoch. Verification does not publish the active
  topology.
- Section 12.2: an opaque known-erasure authorization binds array, topology
  epoch, recovery generation, requested range, missing slot/position, geometry,
  and every source identity. Dirty, indeterminate, uncovered, stale,
  ambiguous, aliased, excluded, and short-read inputs fail closed. Successful
  results carry explicit degraded telemetry and perform no payload writes.
- Section 12.7: deterministic chunks order decode, separate-target write,
  exact readback/equation check, target flush/fence, and generation-checked
  recovery cursor commit. The cursor is the first unprocessed byte. Its typed
  state survives bounded semantic export and exact restore.
- Section 17.6: the ordinary-file fixture closes and reopens source and replacement
  files across an interrupted rebuild, rejects a different replacement file
  before resumed I/O, verifies every replacement byte, then drops all stores
  before an independent direct read. Surviving data, parity, and the retained
  reference remain byte-identical. The environment-gated fixture additionally
  rebuilds a detached APFS image for independent post-service attachment.
- Section 26.4: algebra alone never authorizes a read. The caller must provide
  parity-clean or replay-proven range evidence under a current recovery
  generation; dirty/unknown evidence refuses reconstruction.

## Implementation evidence

- `dwv-verify::degraded` owns the portable single-erasure authorization,
  exact-range decoder, zero-tail rules, refusal reasons, and degraded
  telemetry.
- `dwv-verify::rebuild` owns deterministic planning, the durability-target
  trait, post-readback/post-flush chunk receipts, idempotent replay, and the
  complete final verification receipt.
- `dwv-recovery::rebuild` owns bounded rebuild and target identities, immutable
  source/replacement binding, lifecycle, durable cursor/fence, final digest,
  generation-checked mutations, and prepared replacement topology. Semantic
  schema v3 adds this state and retains the v2-to-v3 migration plan.
- `dwv-service::rebuild` adapts exact `FileStore` operations and is the explicit
  verifier-receipt to recovery-transaction bridge. Resume checks the observed
  replacement file identity before any resumed payload I/O.
- `dwv-service::authorize_file_known_erasure` now backs the macOS rebuild
  fixture with active recovery topology, exact store/assignment identities,
  quiescence, and recovery-generation checks instead of caller-supplied clean
  flags.
- `dwv-recovery-sqlite::SqliteRecoveryStore` provides an exclusive process
  lease, complete-manifest validation, generation-CAS persistence, and
  candidate-before-in-memory-publish ordering. The close/reopen rebuild test
  advances typed rebuild state through this adapter.
- Crash-boundary tests cover refusal before a write, failed readback after a
  write, failed durability after readback, lost-checkpoint-safe idempotent
  replay, stale/atomic checkpoint refusal, interrupted manifest restore,
  mismatched replacement refusal, and failed final verification.

The new behavior is organized in dedicated `degraded.rs` and `rebuild.rs`
modules; the crate `lib.rs` files remain API/re-export boundaries rather than
holding the implementation.

## XOR dependency decision

The 2026-08-07 dependency review found no crate matching the handoff's
reference semantics. `simd-rs63` is fixed RS(9,6) over equal aligned blocks,
`rune-xor` is a repeating-key cipher helper, and `gf2` provides packed bit
vectors/matrices. None supplies per-slot lengths, exact byte ranges, and
DiskWeave zero-tail behavior. `dwv-codec::XorReference` remains the
dependency-free portable correctness oracle behind `ParityCodec`; an optimized
backend can be added later only with conformance and benchmark evidence.

## Acceptance commands

- `cargo test --workspace` — passed: 186 tests plus all doctests; the external
  DiskImages fixture is ignored unless explicitly enabled.
- `DWV_OS016_APFS_REFERENCE=... DWV_OS016_APFS_REPLACEMENT=... cargo test -p
  dwv-service --test os016_macos_apfs -- --ignored --nocapture` — passed using
  a disposable 32 MiB APFS image.
- `hdiutil attach -readonly -nobrowse -mountpoint ... replacement.dmg` plus
  byte comparison of the mounted marker — passed after the rebuild process
  released all file stores; the image detached cleanly.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p dwv-recovery --all-targets --no-deps -- -D warnings` —
  passed.
- `cargo clippy -p dwv-recovery-sqlite --all-targets --no-deps -- -D warnings`
  — passed.
- `cargo clippy -p dwv-verify --all-targets --no-deps -- -D warnings` — passed.
- `cargo clippy -p dwv-service --all-targets --no-deps -- -D warnings` —
  passed.
- `cargo tree --workspace -e normal` — passed; OS-016 adds no third-party
  dependency or `Cargo.toml` change.
- `cargo metadata --format-version 1 --no-deps` — passed.
- `openspec validate os-016-degraded-reads-offline-rebuild --strict --json` —
  passed.

## Deferred work

- OS-017 owns checksum scrub scheduling and uniquely evidenced verified repair.
- OS-021/OS-022 own live macOS bridge and APFS attach/mount acceptance.
- Linux ublk, online rebuild, P/Q, two-erasure, and hardware durability gates
  remain intentionally skipped on this macOS/core path.
