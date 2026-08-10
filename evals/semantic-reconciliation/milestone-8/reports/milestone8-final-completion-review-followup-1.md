# Milestone 8 final completion review — follow-up 1

Date: 2026-08-10  
Repository: `/Users/daniel.vicory/src/DiskWeave`  
Purpose: post-archive completion evidence requested by the externally supplied final implementation review.

## A. Result and approval boundary

The requested finalization is complete:

- `milestone-planning-cutover` was finished and archived through the normal OpenSpec workflow.
- All four Milestone 8 changes are archived.
- No active OpenSpec change or active OpenSpec task remains.
- The active-roadmap requirement was reconciled by bounded semantic tests and retained under its existing stable ID.
- The repository-native planning scan reports zero forbidden project-planning uses.
- The final Linux acceptance was rerun because the post-archive source archive differed from the retained pre-archive archive; replacement evidence and traces were retained and replayed.
- `docs/verification/m8.md` and `docs/verification/linux-ublk-ext4-acceptance.md` now record the final archive, provenance, canonical, validation, and Linux evidence state.
- No later roadmap work was started.

This package is agent-authored evidence, not an external verdict. The externally supplied final review in the conversation withheld Milestone 8 completion approval pending this package. Final external completion approval therefore remains with the reviewer.

## B. Final OpenSpec archive state

### B.1 Archived Milestone 8 changes

| Change | Final archive path |
|---|---|
| Canonical semantic ownership | `openspec/changes/archive/2026-08-09-canonical-semantic-ownership/` |
| Metadata certificate-authority gate | `openspec/changes/archive/2026-08-10-metadata-certificate-authority-gate/` |
| Normalized service request boundary | `openspec/changes/archive/2026-08-09-normalized-service-request-boundary/` |
| Milestone planning cutover | `openspec/changes/archive/2026-08-10-milestone-planning-cutover/` |

The fourth archive contains `.openspec.yaml`, `proposal.md`, `design.md`, its documentation-knowledge-architecture delta, and `tasks.md`. The archive operation preserved the delta without reapplying it because its six changed requirement blocks already matched the current canonical specification exactly. Its final checklist is 24/24 complete.

### B.2 Active state and strict validation

Exact `openspec list --json` result:

```json
{
  "changes": [],
  "root": {
    "path": "/Users/daniel.vicory/src/DiskWeave",
    "source": "nearest"
  }
}
```

Consequences:

- active OpenSpec changes: **0**;
- remaining active OpenSpec tasks: **0**;
- `openspec validate --strict --all`: **25 passed, 0 failed** after the fourth archive and after the final evidence-record edits.

## C. Final Jujutsu revision and tree

Captured only after the final repository edits and validations:

- change ID: `wkunqmklopsyklltnooxyxxxlkowovkv`;
- commit ID: `215201a3b7dd91e51cd46613bec91de5f663e342`;
- tree ID: `825c8609260cf02244cb1bee4f3b165218badc84`;
- working-copy description: unset;
- parent: `qxsmtwqv 41af7711`, `chore: milestone 8 pre-final-archive integrated review checkpoint`.

The exact commit/tree IDs are recorded here rather than inserted into tracked `docs/verification/m8.md`: inserting either into that tracked file would change the commit and tree being named. The stable archive/evidence facts are in the tracked verification record; the self-referential revision identifiers are in this external follow-up.

Exact six-revision summary:

```text
wkunqmkl 215201a3b7dd 
qxsmtwqv 41af7711916d chore: milestone 8 pre-final-archive integrated review checkpoint
nkntwxmq 132253e3e7bb chore: milestone 8 normalized-service-request-boundary review checkpoint
lzyrykpp 8820fa1293df chore: milestone 8 pre-request-boundary checkpoint
ymxkqywk 99cdfbeb254c chore: milestone 8 metadata-certificate-authority-gate implementation
qxsmmvvl a54d8df7349c chore: milestone 8 canonical-semantic-ownership implementation
```

## D. Active-roadmap semantic reconciliation

### D.1 Exact retained requirement

Stable ID: `req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative`

Current requirement text:

> The documentation workflow SHALL deterministically discover exactly one active architecture roadmap from one explicit repository marker. The selected roadmap SHALL remain excluded from ordinary current semantic discovery, projections, freshness checks, and requirement context. A deliberate milestone-planning request MAY select the marked roadmap, but the resulting material SHALL be identified as non-authoritative design direction and SHALL NOT satisfy current requirement coverage. A detected disagreement between the roadmap and current canonical requirements SHALL be reported for explicit reconciliation and SHALL NOT be resolved by silently preferring either source.

Current scenarios:

1. **Exactly one active roadmap is marked**
   - **WHEN** documentation readiness scans the maintained repository
   - **THEN** it resolves one deterministic active-roadmap path and continues without treating that document as current semantic authority.
2. **The active-roadmap marker is missing or ambiguous**
   - **WHEN** zero or multiple active-roadmap markers are present
   - **THEN** documentation readiness fails with the marker count and every marked path rather than choosing a roadmap implicitly.
3. **Milestone planning deliberately requests roadmap context**
   - **WHEN** milestone planning selects the marked active roadmap after inspecting current canonical requirements
   - **THEN** the roadmap is available as non-authoritative direction and remains outside ordinary requirement coverage and context.
4. **Current requirements and roadmap direction disagree**
   - **WHEN** planning or semantic reconciliation detects a material disagreement
   - **THEN** it records the disagreement for an explicit specification or roadmap decision without silently rewriting current semantics.

Exact authored outgoing relationship:

- `requires` → `req.documentation-knowledge-architecture.historical-architecture-is-opt-in-only`.

No `refines`, `required_by`, or `refined_by` relationship exists for this requirement.

### D.2 Bounded reconciliation tests

| Test | Result |
|---|---|
| Direct subject | **Pass.** The requirement owns exact-one marker cardinality, deterministic active-roadmap selection, fail-closed ambiguity, deliberate non-authoritative selection, and disagreement reporting. No other current requirement owns that complete predicate. |
| Semantic deletion | **Pass; retain.** Deleting it would leave historical/milestone exclusion but remove marker cardinality, deterministic resolution, zero/multiple-marker failure, deliberate planning selection, and disagreement policy. |
| Two conforming implementations | **Pass; the requirement constrains a real choice.** Without it, one implementation may pick a hard-coded/first path while another requires exactly one marker and rejects ambiguity; both could still satisfy surrounding historical-exclusion requirements. |
| Closest duplicate predicate | **Pass; no duplicate.** Canonical discovery owns extraction/stable IDs; historical isolation owns opt-in archaeology; agent context owns bounded current packets; clean-room owns reconstruction. None owns the complete active-roadmap protocol. |
| Historical independence | **Pass.** Its marker, authority, coverage, context, failure, and disagreement semantics are implementable without reading v0.8, milestones, archives, handoffs, prior reviews, or conversation history. |

Disposition: **retain the existing stable ID and wording; no rename, split, merge, or semantic repair is warranted.** The requirement is implementation-owning and testable. It is not a duplicate registry entry or a non-authoritative restatement of roadmap content.

## E. Planning migration and retained-language classification

### E.1 Exact repository-native scan

Command:

```text
cargo xtask docs planning-nomenclature
```

Final structured result:

```json
{
  "command": "planning-nomenclature",
  "diagnostics": [],
  "ok": true,
  "outcome": "success",
  "result": {
    "diagnostics": [],
    "forbidden_project_planning_uses": 0,
    "ordinary_goal_language_allowed": true,
    "schema": "dwv.docs.planning-nomenclature.v1"
  },
  "schema": "dwv.docs.cli.v1"
}
```

Classification:

- forbidden retained project-planning identifiers: **0**;
- Jujutsu history: excluded from the current-tree scanner and from this count;
- dedicated negative fixtures: allowed only where `xtask/src/knowledge.rs` tests construct forbidden forms from fragments to prove rejection; no literal forbidden project-planning form remains in the maintained tree;
- ordinary English `goal`/`goals` uses: intentional and harmless. They describe product goals, learning/documentation goals, `Goal` or `Goals / Non-goals` headings, historical milestone prose, and old OpenSpec archive metadata. They are not numbered current-planning identifiers, aliases, or semantic authorities. Milestones and prior architecture are excluded from ordinary current semantic discovery by contract.

### E.2 Final milestone inventory

```text
docs/milestones/README.md
docs/milestones/m1.md
docs/milestones/m2.md
docs/milestones/m3.md
docs/milestones/m4.md
docs/milestones/m5.md
docs/milestones/m6.md
docs/milestones/m7.md
docs/milestones/m8.md
```

The predecessor planning records were moved one-to-one from `docs/handoffs/goal.md` and the retained numbered successor files under `docs/handoffs/` into `docs/milestones/m1.md` through `m7.md`; the current record is `m8.md`. No predecessor planning path remains.

### E.3 Deleted and renamed archive inventory

- Deleted as incomplete/superseded, with no tombstone or compatibility lookup: `openspec/changes/archive/2026-08-09-knowledge-normalization/`.
- Renamed one-to-one: `openspec/changes/archive/2026-08-09-close-goal-v7-acceptance-gaps/` → `openspec/changes/archive/2026-08-09-close-milestone-7-acceptance-gaps/`.
- Archived through the normal workflow: `openspec/changes/milestone-planning-cutover/` → `openspec/changes/archive/2026-08-10-milestone-planning-cutover/`.

Final classification: no predecessor handoff path, old acceptance archive path, superseded knowledge archive, old Linux acceptance runtime name, planning alias, redirect, symlink, duplicate archive, or crosswalk/lookup compatibility record remains.

## F. Linux source-snapshot provenance

### F.1 Exact procedure and boundary

`tools/linux-disk-acceptance/run.sh:20-29` creates the execution archive from repository root `.` before guest startup and before new evidence is copied back.

Excluded paths/classes:

- `.git`;
- `.jj`;
- `.omp`;
- `target`;
- `tools/macos-bridge-probe/.build`.

Included paths/classes:

- every other path present in the working tree at capture time;
- Rust source and tests;
- scripts and tool configuration;
- current canonical specs;
- archived OpenSpec changes;
- documentation and milestone records;
- pre-existing verification files.

Self-reference avoidance:

- `run.sh` closes and hashes the source archive before it starts the guest;
- new evidence and traces are copied into `verification/` only after guest completion;
- those generated files therefore cannot alter the archive executed by that run;
- because documentation and prior verification files are included, the execution-archive digest is not claimed as a content-stable digest of the later post-run working tree.

### F.2 Digest comparison and rerun decision

| Snapshot | SHA-256 | Meaning |
|---|---|---|
| Retained pre-archive execution archive | `56751be073f989b77f1d4589ec0d2c810be347facd33e5bab927bd0c1f9a2a4e` | Source used by the prior final Linux run. |
| First post-archive recomputation | `4e092d5a185f2edce3c572b9f03fdbe85f0451289e0db31aa56d3397d72e06b9` | Differed from the retained digest, so the retained run was not accepted as final-source evidence. |
| Immediate second compressed recomputation | `19c5ca5e264a7c076b5062cb0b6c855eed1e9aa84d93418f93f88ac2644d1a09` | Demonstrated gzip-header variability. |
| Uncompressed tar stream for both recomputations | `d853aedd383c0643390112edd67ba0765be6206c384f35df6035b48380ba249e` | Proved the two immediate recomputations had identical tar payloads despite differing compressed digests. |
| Final live-run execution archive | `2d8c947cfdbdc5012964b6464eb23433366cdcf91a1c35c77568578c5e5d328c` | Exact compressed archive copied into and executed by the final guest run. |

Decision: **rerun required and completed**. The digest comparison did not support blindly retaining the pre-archive Linux run.

## G. Functional Linux names and observed generated values

Source-defined functional names:

| Purpose | Value | Source |
|---|---|---|
| VM prefix | `dwv-linux-acceptance-` | `tools/linux-disk-acceptance/run.sh:6` |
| Fixture root | `/var/tmp/dwv-linux-acceptance` | `tools/linux-disk-acceptance/guest.sh:6` |
| Mountpoint | `/mnt/dwv-linux-acceptance` | `tools/linux-disk-acceptance/guest.sh:7` |
| Guest evidence path | `/tmp/dwv-linux-acceptance-evidence.json` | `tools/linux-disk-acceptance/guest.sh:275`; copied by `run.sh:36` |

Observed final run:

- VM instance: `dwv-linux-acceptance-1786384884`;
- guest: Ubuntu 26.04 arm64;
- kernel: `7.0.0-28-generic`;
- real endpoint: `/dev/ublkb0`;
- generated ownership-conflict evidence embeds `/var/tmp/dwv-linux-acceptance/recovery.sqlite3.dwv-lock`;
- generated unsupported-discard evidence embeds `/dev/ublkb0`;
- the generated JSON does not embed the ephemeral VM name, mountpoint, or guest evidence-output path. Their values are proven by the executed scripts and captured process output rather than invented after the run.

## H. Final Linux evidence and replay

Machine-readable evidence:

- path: `verification/linux-ublk-ext4-acceptance.json`;
- schema: `dwv.verification.linux-ublk-ext4.v1`;
- size: 9,975 bytes;
- SHA-256: `4443201278b62bf6070590f8bf2008dcf14891c6230a6adb5b771a79cf2cda6e`;
- embedded source archive SHA-256: `2d8c947cfdbdc5012964b6464eb23433366cdcf91a1c35c77568578c5e5d328c`.

Retained traces:

| Path | Schema | Records | Bytes | SHA-256 | Root CLI replay |
|---|---|---:|---:|---|---|
| `verification/linux-ublk-trace-first.json` | `dwv.ublk.trace.v2` | 456 | 448,488 | `f01f8c2fc1b7e1a5d7fb2e792d59fb69406e2f0722361f2b2016990df7c72192` | clean; `record_count: 456` |
| `verification/linux-ublk-trace-second.json` | `dwv.ublk.trace.v2` | 83 | 81,494 | `b9524022e53b7cc12e2025d90383f40abc05b4a8c9169922a1ac8efc248decf8` | clean; `record_count: 83` |

Both traces recorded queue depth 8, maximum transfer 131,072 bytes, maximum 4,096 records, and zero exhausted records. Both replay commands returned `ok: true`, `outcome: success`, `clean: true`, root CLI contract/schema `dwv.cli.v0`, and the expected record count.

Observed workload and data facts:

- ext4 format and mount;
- create, fsync, overwrite, rename, directory sync, read, delete, unmount;
- clean shutdown, restart, and read-only remount;
- durable content SHA-256 before/after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`;
- data and parity payload SHA-256: `61c7461a00d6395a25930af9e6d98a30f2105d76c2c0769f1f0858c63a794d10`;
- byte equality passed;
- the ordinary data member mounted independently read-only and exposed the same content;
- both ublk runs reached lifecycle state `stopped` only after drain, checkpoint, endpoint removal, and replay checks.

Negative cases failed closed for bounded-resource admission, unsupported topology, ownership conflict, conflicting cleanup, stale readiness, missing recovery authority, unsupported discard, unknown endpoint cleanup, owner-process death, and partial multi-store fence coverage.

Claim boundary remains narrow: this is not production durability, physical power-loss safety, multi-device atomic publication, daemon recovery, production FUA, discard/write-zeroes support, online topology mutation, raw-device deployment, or hardware certification. SQLite remains evaluation-only.

## I. Final canonical graph and schemas

Counts:

- capabilities: **25**;
- requirements: **153**;
- relationships: **69**;
- `requires`: **59**;
- `refines`: **10**.

Final schemas:

| Surface | Schema |
|---|---|
| Reviewed state | `dwv.knowledge.reviewed-links.v2` |
| Readiness | `dwv.knowledge.readiness.v2` |
| Ownership | `dwv.knowledge.ownership.v1` |
| Context | `dwv.knowledge.context.v2` |
| Change impact | `dwv.knowledge.change-impact.v1` |
| Planning scan | `dwv.docs.planning-nomenclature.v1` |
| Documentation CLI envelope | `dwv.docs.cli.v1` |
| Clean-room evidence | `dwv.docs.clean-room.v1` |
| Linux acceptance | `dwv.verification.linux-ublk-ext4.v1` |
| Linux trace | `dwv.ublk.trace.v2` |
| Root CLI | `dwv.cli.v0` |

Final readiness gates:

| Gate | Count |
|---|---:|
| `historical-reference` | 0 |
| `local-fingerprint-suspect` | 0 |
| `mapping-collision` | 0 |
| `orphaned-state` | 0 |
| `retired-planning-identifier` | 0 |
| `semantic-prerequisite-changed` | 0 |
| `uncovered` | 0 |
| `unknown-reference` | 0 |

The seven required owner/composer inspections returned `dwv.knowledge.ownership.v1`, with no missing owner and no conflict. Current requirements and relationships are derived from canonical specs; milestones, handoffs, historical architecture, and archives remain excluded from ordinary current semantics.

## J. Exact final validation results

| Command/check | Exact observed result |
|---|---|
| `openspec validate --strict --all` | 25 passed, 0 failed |
| `cargo test -p xtask` | 37 passed across 3 suites |
| `cargo xtask docs knowledge readiness` | ready; 153 requirements; all eight named gates zero; no diagnostics or next actions |
| `cargo xtask docs check` | ready; no diagnostics; no review-required IDs; zero change-impact entries |
| `cargo xtask docs build` | 153 objects rendered under `target/dwv-docs/sphinx/html` |
| `cargo xtask docs clean-room` | equivalent; digest `c39f8fbe29fb144ac6cbedc7fa60d9ce0b84cc59ba5fb4b71e75f2f47c081d7f` |
| `cargo test -p dwv-core` | 15 passed across 2 suites |
| `cargo test -p dwv-store` | 15 passed across 2 suites |
| `cargo test -p dwv-service` | 35 passed across 3 suites; 1 ignored |
| `cargo test -p dwv-recovery metadata_loss` | 7 passed; 41 filtered |
| `cargo test -p dwv-frontend-ublk` | 12 passed across 2 suites |
| `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected` | 1 passed; 16 filtered |
| `cargo test --test cli_demo` | 3 passed |
| `cargo test --workspace --all-targets` | 303 passed across 22 suites; 1 ignored |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets --no-deps -- -D warnings` | passed |
| Final first-trace replay | clean; 456 records |
| Final second-trace replay | clean; 83 records |
| Final post-archive live Linux acceptance | success; replacement JSON and traces retained |

Formal/model results were retained because archive/finalization changed no proof input after the completed runs:

- PlusCal translation: passed;
- TLC 2.19: 234 generated states, 125 distinct states, complete depth 9, no errors;
- `tla-checker` 0.6.11: 125 states, 233 transitions, maximum depth 9, status `ok`;
- Kani 0.67.0 `dirty_region_mapping_covers_every_intersection_once`: 0 of 584 checks failed; 7 unreachable;
- Kani 0.67.0 `fence_coverage_requires_every_store_region_and_incarnation`: 0 of 893 checks failed; 6 unreachable.

Final proof-input SHA-256 values:

- `verification/tla/RecoveryProtocol.tla`: `378986217ce4878f06e8e7f5811f4ff00d6499db96aa8d52284bd223f1d0a054`;
- `verification/tla/RecoveryProtocol.cfg`: `305d836110d0199fbb66ddc7d235606a426af4c3af1fc9eb6d3c4095d2926c46`;
- `crates/dwv-recovery/src/lib.rs`: `4d1cf5b4bf80684e9bdda3ad2ac71c2de9a5789a16985d22ea121832add977b2`;
- `crates/dwv-transaction-ref/src/lib.rs`: `07cdcaee7d78df1786abb378fea7eee644aa6fcc99b5f6a93b4f85f13e371df9`.

These finite proofs do not certify SQLite/filesystem/controller/FUA/physical power-loss durability.

## K. Review provenance

| Checkpoint | Accurate provenance | Disposition |
|---|---|---|
| Canonical semantic ownership proposal/external-review planning artifact | Externally supplied planning/review input; not itself a post-implementation verdict | Supplied the accepted semantic model and cleanup requirements. |
| Canonical semantic ownership implementation review and follow-up files | Agent/subagent implementation review briefs and evidence packages | Initial implementation review rejected one inaccurate fingerprint description and requested bounded Linux evidence-remap verification; the correction and focused regressions were completed. These files are not relabeled as external verdicts. |
| Metadata certificate-authority implementation review file | Agent/subagent review brief and deterministic evidence package | The authority fix, tests, canonical review, and post-archive gates passed. The retained brief requested a verdict; its title does not prove external authorship. |
| Canonical request-boundary implementation review file | Agent/subagent integrated review package | Its product findings and deterministic evidence are retained. Wording that claimed external review is not used as provenance here. |
| Planning-cutover review checkpoint | Agent/subagent review plus deterministic scanner/archive/Linux checks | The superseded archive and stale evidence were corrected before archive. |
| `~/tmp/milestone8-final-completion-review.md` | Agent self-review | Retitled and marked superseded; it is explicitly not an external verdict. |
| Final completion review supplied in the current conversation | Externally supplied final implementation review | Accepted product direction, withheld final completion approval, authorized finishing/archiving the fourth change, and required this follow-up. |
| This follow-up | Agent-authored evidence summary | Does not grant external approval. |

User approval boundary: the explicit instruction in the externally supplied final review authorized completing and archiving `milestone-planning-cutover`. Repository validation cannot grant final external completion approval.

## L. Final working-copy status and diff

Exact `jj status` path set:

```text
M docs/verification/linux-ublk-ext4-acceptance.md
M docs/verification/m8.md
R openspec/changes/{milestone-planning-cutover => archive/2026-08-10-milestone-planning-cutover}/.openspec.yaml
R openspec/changes/{milestone-planning-cutover => archive/2026-08-10-milestone-planning-cutover}/design.md
R openspec/changes/{milestone-planning-cutover => archive/2026-08-10-milestone-planning-cutover}/proposal.md
R openspec/changes/{milestone-planning-cutover => archive/2026-08-10-milestone-planning-cutover}/specs/documentation-knowledge-architecture/spec.md
R openspec/changes/{milestone-planning-cutover => archive/2026-08-10-milestone-planning-cutover}/tasks.md
M verification/linux-ublk-ext4-acceptance.json
M verification/linux-ublk-trace-first.json
M verification/linux-ublk-trace-second.json
```

Exact diff stat:

```text
docs/verification/linux-ublk-ext4-acceptance.md    |   17 +-
docs/verification/m8.md                            |   84 +-
...8-10-milestone-planning-cutover}/.openspec.yaml |    0
...026-08-10-milestone-planning-cutover}/design.md |    0
...6-08-10-milestone-planning-cutover}/proposal.md |    0
...cs/documentation-knowledge-architecture/spec.md |    0
...2026-08-10-milestone-planning-cutover}/tasks.md |    2 +-
verification/linux-ublk-ext4-acceptance.json       |   34 +-
verification/linux-ublk-trace-first.json           | 5783 +++++++++++-----------
verification/linux-ublk-trace-second.json          |  830 +--
10 files changed, 3522 insertions(+), 3228 deletions(-)
```

Every remaining working-copy change:

1. `docs/verification/linux-ublk-ext4-acceptance.md` — replaced stale pre-archive run facts with the final post-archive source digest, functional path provenance, replacement artifact digests/counts, and rerun boundary.
2. `docs/verification/m8.md` — completed the planning-cutover archive record; added bounded active-roadmap reconciliation, accurate review provenance, four-archive/no-active-change state, final graph/schemas/gates, proof-input disposition, exact source-snapshot procedure, and replacement Linux evidence.
3. `.openspec.yaml` — path-only move into the dated fourth archive.
4. `design.md` — path-only move into the dated fourth archive.
5. `proposal.md` — path-only move into the dated fourth archive.
6. `specs/documentation-knowledge-architecture/spec.md` — path-only move into the dated fourth archive; the canonical delta was already present and was not reapplied.
7. `tasks.md` — moved into the dated fourth archive and changed the archival checklist from 23/24 to 24/24.
8. `verification/linux-ublk-ext4-acceptance.json` — final post-archive generated machine evidence from the live Linux run.
9. `verification/linux-ublk-trace-first.json` — replacement first live trace, 456 records.
10. `verification/linux-ublk-trace-second.json` — replacement second live trace, 83 records.

No source-code, canonical-spec, test, build configuration, or later-roadmap implementation change remains in the final working-copy delta relative to the pre-final-archive checkpoint.

## M. Residual non-claims and actual deferred post-M8 work

The following are explicit later product/maintenance decisions, not unresolved Milestone 8 defects:

- production recovery-adapter selection and the complete lost-acknowledgement/process-restart redesign;
- broader local-file identity versus topology-authority cleanup;
- transaction-engine candidate retirement;
- macOS bridge-feasibility retirement;
- trace-schema compatibility removal;
- codec redesign;
- large-module decomposition;
- broader Linux topology, concurrency, and publication support;
- raw-device deployment;
- production FUA;
- physical power-loss claims.

No implementation, OpenSpec change, or planning document for those deferred items was started. No new `goal-vN` or new numbered milestone was created.

Milestone 8 is fully implemented, all four changes are archived, and the final no-active-change source state is ready for external completion approval.