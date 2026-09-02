## Why

`RecoverySnapshot::fences` currently grows with every protected write and has no normal retirement path. A bounded export limit therefore becomes a hidden lifetime allowance: healthy long-running operation eventually refuses even after settled writes no longer need their individual persistence evidence. The approved repair makes exact claim liveness, occurrence identity, migration, and retirement canonical before implementation.

## What Changes

- Define one stable semantic identity and multiplicity rule for each persisted fence occurrence, including duplicate values, canonical internal ordering, and exact references after reopen.
- Define owner-qualified liveness roots for current CLEAN and integrity claims, conservative current-session bindings, coded-capture evidence, exact adapter prior/proposed recovery commits, and legacy-unreconciled migration state, including each root's creation, discharge, and exact rebind boundary.
- Define exact per-region and per-integrity-extent supersession; no scalar-watermark or partial-range inference, and no retirement of the last occurrence satisfying a claim without an atomic rebind.
- Add a generation- and topology-checked durable retirement transition that preserves the exact predecessor on known rejection or non-commit and treats lost, corrupt, or unclassifiable acknowledgement as unresolved until exact reopen reconciliation.
- Require unrelated settled evidence to remain independently retireable when one dependency is unresolved; ordinary settled writes do not create permanent lifetime roots.
- Define an explicit semantic-schema migration for legacy manifests, with conservative legacy-unreconciled retention and no truncation or silent coalescing.
- Keep pre-mutation capacity reservation as the separately tracked `dwv-x6y.2.2` prerequisite for serving implementation; this change defines retirement/successor rejection, not that reservation protocol.
- Keep aggregate coverage summaries, external archives, and session-close supersession outside this first exact-retirement change; add them only through deliberately dependent changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recovery-state-semantics`: make exact fence occurrence identity, claim liveness, supersession, durable retirement, legacy migration, and conservative successor rejection canonical.

## Impact

- `dwv-recovery`: recovery manifest schema, exact fence occurrence bindings, root/discharge validation, retirement mutations, export bounds, and reopen reconciliation.
- `dwv-recovery-sqlite`: explicit semantic-schema migration and durable candidate/predecessor retirement persistence without exposing SQLite layout.
- `dwv-service`: compose current CLEAN, integrity, coded-capture, lifecycle, and uncertainty owner facts; preserve current writable-session global-fence bindings conservatively; session-close supersession remains `dwv-x6y.2.3`.
- All snapshot constructors, adapter dispatch, inspection projections, fixtures, and verification relationships must migrate together; ordinary data/parity payload files remain unchanged.
