## Context

The production binary is currently a demo-only, binary-local CLI: it hand-parses `Vec<String>`, couples dispatch directly to JSON construction, always renders JSON despite the canonical human-output requirement, derives error command identity heuristically, and concentrates unrelated fixture behavior in a large `demo.rs`. This is not an acceptable production CLI boundary. The accepted Linux fixture already provides a bounded one-data/one-P file-backed topology and real ublk publication path, while `dwv-verify`, metadata-loss planning, identity/topology types, and portable service admission already own their individual semantics.

`SqliteRecoveryStore::open` acquires a writable claim and calls initialization/migration before loading. `RecoverySnapshot` persists generic integrity records but not the active checksum profile/set and expected target/extent coverage needed to interpret a complete post-recovery baseline. `HealthyPortableService::open` currently constructs absent in-memory checksum records on every reopen.

## Goals / Non-Goals

**Goals:**

- Keep one semantic result between application composition and human/JSON rendering.
- Make observation physically unable to initialize or migrate SQLite recovery state.
- Reuse the canonical verifier, metadata-loss planner/authorization, recovery store, service admission, and frontend publisher rather than copying their predicates into the CLI.
- Preserve exact current authority at apply and baseline commit boundaries with bounded synchronous work.
- Keep uncertain commit reconciliation semantic and conservative.
- Replace the current ad hoc demo CLI implementation with `clap`, typed commands, and one testable application/result/rendering path while retaining accepted demo evidence.
- Use one versioned declarative array-policy file for repeated offline targeting without promoting configuration or locators into current topology, recovery, or identity authority.

**Non-Goals:**

- General array discovery/catalogs, a daemon or local protocol, background jobs, retained correctness certificates, automatic repair, generic force, new degraded-read semantics, wider Linux publication, macOS publication, or stronger SQLite durability claims.
- Compatibility with pre-M9 recovery manifests; an older semantic manifest is observationally migration-required and requires explicit writable migration/recreation rather than read-time mutation.

## Decisions

### One change and one off-the-shelf parser

Use this one cross-capability change because inspection, recovery publication, baseline persistence, service admission, and frontend publication form one correctness boundary. Use `clap` derive in the root command application for typed parsing and generated usage. Keep `src/main.rs` as the process adapter and place application composition, bounded result envelopes, outcome-to-exit mapping, and human/JSON rendering in focused root modules. Do not create a CLI library or custom parsing framework. No correctness predicate belongs in the parser or renderer.

The current hand-written `demo` dispatch is treated as integration-fixture debt, not a production architecture to extend. Retained demo and Linux acceptance commands move behind the same `clap` command tree and result/rendering boundary and continue calling the same portable/application adapters; they do not keep a second parser or result envelope. `demo.rs` remains fixture workflow code only, with no parser, output-policy, or production semantic ownership.

The initial production locator is `--array <array.json>`. This bounded versioned declarative policy names the recovery-state locator, candidate member locators, expected array/member observations, desired slot/coding-position bindings, and narrow frontend selection. It is operator-supplied desired context, not current topology or recovery truth: every command re-observes members and composes canonical identity, topology, and recovery decisions; disagreement is reported and mutation or publication refuses. The existing fixture manifest remains test-fixture state rather than becoming a production format. Acceptance fixtures may generate array policy, but production code does not parse the ublk fixture schema.

### Observational SQLite inspection

Add `SqliteRecoveryStore::inspect(path)` returning a portable bounded inspection enum and optional validated semantic manifest. It runs SQLite with `-readonly`, never acquires the writer lock, never calls `initialize`, and first distinguishes missing/non-file state. It reads physical schema and the single complete manifest without DDL, validates the stored digest and complete semantic value, and classifies:

- absent;
- supported;
- corrupt/unreadable;
- unsupported storage or semantic schema;
- migration required for a known older semantic schema; or
- reconciliation required for conflicting/indeterminate state.

Writable `open` remains the only migration/ownership path. Tests compare file bytes and metadata before/after every inspection class.

### Exact uncertain-commit reconciliation

Use complete `RecoveryManifest` equality after ordinary semantic validation, not generation equality or a stored boolean. A reconciliation function accepts validated prior/proposed manifests plus a reopened inspection and returns `Prior`, `Proposed`, or `ReconciliationRequired`. The SQLite adapter tests inject definite rejection, lost acknowledgement with prior durable state, lost acknowledgement with proposed durable state, and an equal-generation semantic conflict. Callers release the uncertain store before inspection and perform no dependent mutation until this decision.

### Persisted baseline descriptor plus existing integrity records

Add a bounded `ChecksumBaseline` descriptor to `RecoverySnapshot` containing topology epoch, supported digest profile, checksum-set generation, provenance, and the exact expected `ChecksumExtent` list. Existing recovery-owned `IntegrityRecord` values remain the authoritative per-extent validity/digest/fence state; the descriptor supplies target/range/profile/set interpretation rather than duplicating digests.

Fresh metadata-loss state creates the descriptor with every current data and P extent and post-recovery provenance, but no valid records. A pure recovery helper derives `NotRequired`, `Required`, `Partial`, `Complete`, or `Invalid` from the audit obligation, active topology, descriptor, and current integrity records. Completeness requires exact extent-set equality and current valid evidence; no persisted completion bit owns the result.

Baseline continuation opens payload members with exclusive crash-releasing claims, revalidates identities/topology/recovery, skips only already-current valid extents, reads and hashes one extent at a time, obtains typed store fence evidence without a payload write, and commits the fence plus digest through one generation-checked recovery transaction per extent. Per-extent commits make interruption naturally partial and resumable. Newly calculated provenance remains explicit.

### Service admission reconstructs integrity authority

`HealthyPortableService::open` derives baseline status from the loaded snapshot before serving. A current mandatory baseline that is not complete returns a distinct recovery/admission error before payload I/O. If complete, the service constructs its checksum authority from persisted descriptor and records instead of recreating absent records. Arrays without the obligation retain the existing construction and admission behavior.

### Recovery proposal and apply

The plan ID is a BLAKE3 digest of one canonical serialized proposal containing fixture identity/topology facts, recovery inspection class/fingerprint, canonical metadata-loss case/disposition, and baseline consequence. It is bounded explanation and stale-plan detection, not a certificate.

`recover --apply <id>` reacquires exclusive member and recovery publication claims, recomputes the proposal, rejects a mismatch, and runs a new exhaustive `dwv-verify` scan while claims remain held. Only a complete all-match report is converted to `MetadataLossVerification::ExhaustiveMatches` and passed to canonical authorization. Fresh SQLite state is created at a candidate path and validated before publication. A pre-existing untrusted artifact is retained under a bounded sibling name when replacement is practical. Data and parity hashes before/after provide zero-payload-write evidence. The command then releases claims and returns a new observational assessment.

### CLI results and publication

Add production commands:

- `dwv status --array <array.json> [--json]`
- `dwv members --array <array.json> [--json]`
- `dwv scrub --array <array.json> [--json]`
- `dwv damage --array <array.json> [--json]`
- `dwv recover --array <array.json> [--apply <plan-id>] [--json]`
- `dwv baseline --array <array.json> [--max-extents <n>] [--json]`
- `dwv start --array <array.json> [--read-only] [--device-id <n>] [--json]`

`--max-extents` is a deterministic interruption/resume evidence hook and safe bounded-work control, not a correctness shortcut. The application returns the same `dwv.operator.v1` semantic result envelope for every production command. The process adapter selects the default human renderer or explicit JSON and maps the typed outcome to an exit code. Retained demo commands use the same `clap` tree and result infrastructure with their existing bounded command-specific bodies; no second parser or envelope remains.

`start` evaluates application assessment and portable admission first. Read-only publication is currently unsupported. On non-Linux or outside the narrow one-data/one-P profile it returns `not-supported`. On supported Linux it delegates actual publication to the current ublk frontend; the frontend emits a published result only from its publication callback after the endpoint exists. Status treats an endpoint as online only when the frontend adapter validates its owned live publication, never from a stale readiness file alone.
This preserves the intended daemon path without introducing it early. A future `dwvd` may load the same desired policy, own live claims/publication, and expose a versioned local protocol so `dwv` can resolve configured array names through reconstructible control state. Offline inspection and recovery retain explicit `--array` operation when the daemon is unavailable; neither daemon state nor a friendly name becomes recovery authority.

## Semantic ownership reconciliation

| Current requirements | Semantic rule | Selected owner | Other requirements | Contradiction resolved | Semantic edit classification | Stable-ID consequence | Scenario / implementation / evidence migration |
|---|---|---|---|---|---|---|---|
| `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout` | Non-mutating recovery inspection and conservative state classes | Existing recovery-state requirement | Operator assessment requires it; SQLite conforms as adapter | Existing open could mutate while status claimed observation | semantic broadening | Preserve ID; the requirement still owns recovery export/health and now includes observation | Add absent/corrupt/unsupported/migration/reconciliation adapter fixtures and CLI composition evidence |
| `req.recovery-state-semantics.recovery-adapters-report-conservative-commit-observations` | Exact prior/proposed reopen after uncertain acknowledgement | Existing recovery-state requirement | Operator apply composes it | Current requirement stopped at reconciliation-required without the conservative reopen decision | semantic broadening | Preserve ID; same commit-observation obligation becomes executable | Add prior/proposed/neither reopen fixtures, including equal-generation conflict |
| Checksum coverage and validity requirements | What constitutes a current persisted complete baseline | New `req.checksum-plane.current-baseline-completion-is-persisted-and-exact` | Metadata-loss requires new work; recovery persists; service consumes | No canonical owner defined complete persisted coverage across reopen | missing canonical behavior added | New ID | Persist descriptor, derive completion, test interruption/reopen and invalid binding |
| `req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe` | Outstanding mandatory baseline blocks read/write admission | Existing healthy service composition requirement | Checksum owner defines completeness | Service rebuilt absent in-memory checksums and ignored the post-recovery obligation | semantic narrowing | Preserve ID; same admission owner gains one required prerequisite | Add service open tests for required absent/partial/complete and unrelated healthy arrays |
| `req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow` | Published result exists only after actual endpoint publication | Existing Linux adapter requirement | Operator start consumes it | Admission and publication result were not explicitly separated | clarification/no semantic change | Preserve ID | Add pre-publication/failure result tests; retain narrow live evidence |
| No current application owner | Production assessment/action result, typed command parsing, rendering, proposal freshness, and outcome mapping | New `operator-recovery` requirements implemented by the root command application with `clap` | Requires existing identity, scrub, recovery, checksum, admission, frontend, and demo-boundary owners | The hand-written demo CLI otherwise remains a second, low-quality semantic/output stack | missing canonical behavior added plus implementation nonconformance repair | New operator IDs; existing demo requirement IDs remain unchanged | Add parser/render/result tests, migrate retained demo dispatch, and place no generic semantic predicates in renderers |

Dependent review:

- `req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit` remains valid: it owns creation of the obligation; checksum-plane owns its completion.
- `req.degraded-read-offline-rebuild.degraded-read-eligibility-is-explicit-and-fail-closed` remains valid: observational recovery refinement does not broaden degraded reads.
- `req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution` remains valid: baseline completion does not authorize repair.
- `req.linux-ublk-frontend.live-ext4-acceptance-evidence-is-bounded-and-scope-accurate` remains valid: publication profile stays one-data/one-P and existing live evidence remains scope-bounded.

## Risks / Trade-offs

- SQLite command-line read-only inspection is still evaluation/file-backed evidence, not physical power-loss certification. Tests bound the claim to no observed file mutation and semantic classification.
- Per-extent recovery commits are slower than one bulk commit but directly provide resumable partial state without a job framework.
- Candidate replacement cannot make two filesystem names atomically exchange on every platform. The implementation validates the candidate first, preserves the old artifact, reports any ambiguous publication as reconciliation-required, and never derives authority from process-local success.
- This intentionally breaks older semantic manifests. Observation reports migration-required rather than mutating; early-development clean cutover avoids compatibility shims.
