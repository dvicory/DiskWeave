# DiskWeave continuation brief

## Goal

Continue from `diskweave-refined-architecture-v0.6.md` using the repository's
OpenSpec skills and CLI. Implement as much portable core and macOS-capable work
as is honestly dependency-ready, skip Linux-only work, keep changes grounded in
the handoff, commit conventionally with `jj`, and archive an OpenSpec change only
after verification finds no critical issue.

Do not edit the architecture handoff. Do not add a custom OpenSpec checker; use
the OpenSpec CLI and skills. Keep task checkboxes conservative.

## Current checkpoint

- Active change: `os-016-degraded-reads-offline-rebuild`, spec-driven, 9/20 tasks
  complete.
- Archived roadmap work includes OS-000 through OS-015 except OS-007 and OS-009,
  plus OS-020.
- Current jj change ID: `pwwxlqmy`, described as
  `feat(rebuild): add evidence-gated offline rebuild foundation`.
- Its parent is `chore(rust): satisfy strict clippy on current toolchain`.
- Last complete verification:
  - `cargo test --workspace`: 185 passed; the environment-gated APFS test was
    intentionally ignored in the ordinary run.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo fmt --all -- --check`: passed.
  - strict OpenSpec validation: passed.
- A disposable APFS image was previously rebuilt, independently attached
  read-only with `hdiutil` after stores closed, and byte-checked. The standard
  test remains environment-gated.
- `XorReference` remains behind `ParityCodec`. The OS-016 design records why the
  reviewed XOR/RS/GF(2) crates do not implement DiskWeave's exact heterogeneous
  length and zero-tail contract.

Recent completed work includes a lazy aligned bounded rebuild planner, exact
durability fences, atomic `create_new` replacement targets, restart-monotonic
file-adapter fences, complete SQLite manifest serialization/integrity checking,
and a real SQLite close/reopen test that resumes a checkpointed rebuild.

## Resume OS-016 first

Run:

```sh
openspec instructions apply --change os-016-degraded-reads-offline-rebuild --json
```

Read every returned context file and the cited handoff sections before editing.
The best remaining order is:

1. **Production authorization.** Route file-backed degraded reads through
   `FileRebuildSource` and `authorize_file_known_erasure`, grounded in the active
   `RecoverySnapshot`, core topology, quiescence, exact identities, and current
   range evidence. Do not treat caller-supplied clean/source flags as production
   authority. Preserve fail-closed behavior for heterogeneous geometry until
   per-slot lengths have authoritative topology representation.
2. **Durable SQLite authority.** Add a real `SqliteRecoveryStore` implementing
   `RecoveryStateStore`: exclusive create/open, validated complete-manifest load,
   generation-CAS persistence, and candidate -> SQLite commit -> in-memory
   publish ordering. Map adapter failures without publishing the candidate. Add
   missing/oversized/corrupt/digest/header/semantic-invalid tests. Extend the
   existing real close/reopen rebuild test through this adapter.
3. **Receipt provenance.** Re-review verifier/service/recovery chunk receipts.
   They must bind the rebuild, source/replacement assignment, exact durable
   fence, range/cursor, and verified bytes/digest without a publicly forgeable
   shortcut. The service currently rereads and digests the target before commit;
   preserve that boundary.
4. **Crash and topology boundaries.** Complete the before/after irreversible-cut
   matrix, idempotent replay, mismatched resume, final verification failure, and
   prepared-topology behavior. Verification must not publish active topology.
5. Convert acceptance fixtures to the production authorization/source path,
   then update evidence, run full verification, and only then check remaining
   tasks or archive.

Key files:

- `openspec/changes/os-016-degraded-reads-offline-rebuild/`
- `crates/dwv-recovery/src/rebuild.rs`
- `crates/dwv-recovery-sqlite/src/lib.rs`
- `crates/dwv-verify/src/degraded.rs`
- `crates/dwv-verify/src/rebuild.rs`
- `crates/dwv-service/src/degraded.rs`
- `crates/dwv-service/src/rebuild.rs`
- `docs/verification/os-016-degraded-reads-offline-rebuild.md`

## Next OpenSpecs from the roadmap

If a disjoint agent is available while OS-016 continues:

1. **OS-024 — normalized trace fixture/replay engine** is the safest independent
   next change. OS-013 is archived and is its only dependency. Keep it bounded,
   versioned, privacy-safe, deterministic, portable/simulator/macOS-capable, and
   exclude Linux capture (OS-033). It can be specified and implemented now.
2. **OS-007 — parity envelope A/B/C and independent decoder** is an earlier
   roadmap gap whose OS-005/006 prerequisites are archived. It is safe to specify
   as FORMAT-EXPERIMENTAL, but must not silently select/freeze a stable format or
   enable the Gate-H clean fast path.
3. **OS-009 — `procmachines` evidence comparison** is another earlier gap whose
   OS-008 prerequisite is archived. It may compare against the existing explicit
   oracle, but must not replace/select the transaction machine without semantic,
   fault, resource, and maintenance evidence.
4. **OS-021** is safe to specify with an explicit implementation entry gate, but
   live implementation remains blocked by the OS-020-recorded Xcode/Swift,
   signing/entitlement, DiskImages attachment, cache, and synchronization gaps.
5. **OS-017** may be drafted, but implementation waits for archived OS-016.
   **OS-022/023 are premature** until OS-016 and the live macOS frontend exist.
   Skip OS-030+ here because those are Linux-only.

Prefer finishing OS-016 over opening several active changes. If parallelizing,
use disjoint write sets and do not let a future spec redefine OS-016 authority.

## Final hygiene

Before claiming or archiving completion, run:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
openspec validate os-016-degraded-reads-offline-rebuild --strict --json
jj status
```

Use the OpenSpec verification skill before archive. A green build is not enough:
all task requirements, handoff acceptance, negative cases, and claim boundaries
must be evidenced. Keep the jj description conventional and honest while the
change remains a foundation rather than a completed feature.
