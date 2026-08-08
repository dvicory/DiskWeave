# Candidate next OpenSpecs

These are planning notes grounded in
`diskweave-refined-architecture-v0.6.md`, not OpenSpec artifacts. Check the
handoff again when writing each proposal, and use the repository's OpenSpec
skills and CLI rather than inventing a parallel workflow.

## Recommended order

1. Finish, verify, and archive OS-016.
2. OS-024 is the safest independent change to specify and implement now.
3. Backfill the skipped evidence changes OS-007 and OS-009 without retroactively
   treating their provisional choices as proven.
4. Specify OS-017 after OS-016's authority and receipt contracts settle, then
   implement it after OS-016 is archived.
5. OS-021 may be specified with a hard implementation entry gate. Live work is
   blocked on macOS bridge evidence recorded by OS-020.
6. OS-022 follows completed OS-016 and an implemented OS-021; OS-023 follows
   OS-022.

OS-030 and later Phase 3 work is Linux-specific and should be skipped for the
current portable-core/macOS push.

## OS-024 — normalized trace fixture and replay engine

**Readiness:** ready now and independent of OS-016. OS-013, its dependency, is
archived. This is the best next implementation if work must proceed in parallel.

**Handoff anchors:** Sections 9.8, 19, 21.7, and the OS-024 row in Section 22.4.

The change should introduce a bounded, versioned, privacy-safe trace at the
boundary after frontend normalization and before transaction planning. A trace
should carry provenance, slot/range/operation/flags, ordering and fence IDs,
submission/completion concurrency, abandonment, initial topology and
capabilities, deterministic generated-payload seeds or digests, and expected
final data/parity/integrity/recovery state.

Minimum acceptance:

- deterministic replay produces the same final payload, parity, integrity, and
  recovery state;
- failing traces can be minimized without changing the failure;
- malformed, oversized, unsupported-version, and privacy-violating inputs fail
  safely;
- schema migration is tested;
- generated-data traces replay against the simulator and portable/macOS file
  path with equivalent normalized outcomes;
- raw payload bytes and clear user paths are excluded by default.

Keep Linux workload capture out of scope; that belongs to OS-033. Do not add new
storage, parity, transaction, or recovery semantics merely to make replay easy.

## OS-007 — parity envelope profiles A/B/C

**Readiness:** safe to specify and evaluate. OS-005 and OS-006 are archived, but
OS-007 is a skipped historical roadmap gap even though downstream work exists.
Treat existing envelope behavior as provisional evidence, not a frozen format.

**Handoff anchors:** Sections 8.4–8.5, 8.9, 8.11, 16.6, 21.2, and the OS-007 row
in Section 22.2.

Compare these FORMAT-EXPERIMENTAL profiles:

- A: bare parity payload plus external recovery database;
- B: redundant envelope/session certificate plus external database;
- C: profile B plus a coarse durable dirty-region bitmap.

The change needs an independent decoder, an explicit exact-capacity policy ADR,
and evidence for session-certificate crash behavior. It must cover torn copies,
A/B disagreement, unknown required features, migration, bounds and hostile
inputs, capacity accounting, and DB/envelope ACTIVE/CLOSED versus DIRTY/CLEAN
reconciliation.

Do not silently truncate protected capacity, turn the envelope into a second
transaction database, select a stable format without evidence, or enable the
matching-clean-certificate fast path before Gate H and chain-of-custody proof.
Profile B is only the handoff's likely baseline; profile C must justify its extra
hot-path durability participant and write amplification.

## OS-009 — `procmachines` evidence comparison

**Readiness:** safe to specify and investigate. OS-008 is archived. Like OS-007,
this is a skipped evidence gate whose downstream consumers already exist.

**Handoff anchors:** Sections 9.8, 16.4, 21.3, and the OS-009 row in Section
22.2.

Implement or adapt a procedural `procmachines` transaction machine behind the
same semantic interface as the explicit OS-008 reference oracle. Drive both
with identical generated schedules and normalized action results. Compare:

- permitted terminal states and action traces;
- clean/dirty and integrity-invalidation decisions;
- operation-slot and uncertain-completion obligations;
- deterministic fault behavior;
- allocation, locking, CPU, latency, concurrency, and scaling;
- maintainability and dependency health.

Mutation tests must prove the comparison catches skipped invalidation, premature
clear, stale slot-token reuse, and ignored uncertain completion. Research the
current crate and alternatives before adding a dependency. The ADR must record
selection or fallback plus an exit plan; do not replace the explicit oracle just
because the candidate is ergonomic.

## OS-017 — checksum scrub and verified repair

**Readiness:** requirements can be drafted, but implementation waits for OS-016
to be verified and archived. Dependencies are OS-011, OS-014, and OS-016.

**Handoff anchors:** Sections 13.1–13.7, 21.1–21.2, and the OS-017 row in Section
22.3.

Build a full scrub classifier for verified-good, known-bad, missing,
stale/unknown, and unreadable data/P/Q extents. Automatic repair is allowed only
when surviving evidence yields a unique verified solution. Validate the repair
candidate before mutation, perform repair through the ordinary dirty/integrity
invalidation and durable-fence protocol, then verify both the installed digest
and parity equation.

Acceptance must include crash schedules around every repair boundary,
beyond-tolerance and conflicting-evidence refusal, preservation of media on
ambiguity, and parallel checksum-set migration without a coverage gap. Do not
infer the bad side from parity disagreement alone. Online scrub/rebuild, P/Q
repair, and degraded writes are later work.

## OS-021 — macOS DiskWeave frontend

**Readiness:** safe to specify only with an explicit implementation entry gate.
OS-020 is archived, but its ADR did not prove or select a live bridge in the
available environment. Read `docs/adr/os-020-macos-bridge-feasibility.md` first.

**Handoff anchors:** Sections 9.1–9.4, 17.1–17.5, 21.6, and the OS-021 row in
Section 22.4.

The frontend should expose one fixed-size seekable proxy endpoint per stable
slot and translate bridge operations into normalized requests. It must preserve
stable geometry and slot identity, enforce bounded backpressure and abandonment,
drain on detach/restart, reject truncate/hole-punch/bypass, detect path or file-ID
replacement, and keep exported proxy endpoints distinct from backing files.

Parity, recovery, and integrity logic stay in portable Rust, not Swift, FSKit,
macFUSE, or DiskImages glue. Claims remain `portable-demo` until synchronization
is characterized. Implementation cannot honestly begin until a matching macOS
toolchain and privileged session prove installation/signing/entitlements,
DiskImages attachment, cache invalidation, synchronization mapping, disconnect,
and automation for the selected bridge.

## OS-022 — APFS degraded/recovery acceptance

**Readiness:** premature. It depends on OS-015, completed OS-016, and an
implemented OS-021.

**Handoff anchors:** Sections 17.6, 21.6, and the OS-022 row in Section 22.4.

This is the automated twelve-scenario APFS demonstration: create and mount three
virtual members; run ordinary and sync-heavy workloads; verify clean restart;
serve a missing clean member read-only; rebuild and verify a replacement; mount
the rebuilt image independently; recover after deleting `array.sqlite3` without
rewriting matching parity; exercise selective-repair and ambiguous-mismatch
refusal; reject a cloned member; and prove deleting `control.sqlite3` is harmless
to safety.

The milestone fails if it embeds required DiskWeave metadata in a data image,
aliases backing and export endpoints, changes stable slot identity, auto-selects
a clone, overwrites an ambiguous mismatch, or cannot mount the rebuilt image
independently.

## OS-023 — macOS synchronization and durability characterization

**Readiness:** wait for OS-022.

**Handoff anchors:** Sections 17.5, 21.6, and the OS-023 row in Section 22.4.

Characterize the exact mapping from normalized flush/order semantics through the
chosen bridge, DiskImages, host APFS, and backing-file synchronization. Include
sync-heavy process-kill/restart and detach/reattach tests plus operation traces.
The output is a precise `portable-demo` support contract and a list of unsupported
semantics—not a physical power-loss, FUA, controller-cache, or production
durability claim.

## Workflow for the next agent

Before opening one of these changes, run `openspec list --json`, read the cited
handoff sections and existing archived prerequisites, then use
`openspec-propose` (or `openspec-new-change` plus `openspec-continue-change`). Use
`openspec-apply-change` for implementation, `openspec-verify-change` before any
completion claim, and `openspec-archive-change` only after requirements and
acceptance evidence pass. Keep one conventional `jj` description per coherent
change and do not check tasks merely because code compiles.
