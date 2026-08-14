## Context

The current canonical system already owns explicit XOR geometry, exhaustive read-only parity verification, bounded reports, evidence scope, fail-closed uncertainty, and bounded resource admission. The missing boundary is an independently invocable tool that can exercise those owners without production service or recovery-state private data. The campaign's broad C8/U26b tool family remains open; this change selects only the equation-verification slice and does not depend on C8a/U26a or U11.

## Goals / Non-Goals

**Goals:**

- Provide one standalone experimental command that verifies explicit single-XOR payloads over a bounded range.
- Preserve one owner for equation meaning, mismatch classification, evidence authority, and recovery decisions.
- Make all input, output, non-mutation, evidence-tier, and process-status behavior implementation-ready.
- Keep the tool useful when production service, frontend, SQLite, and recovery-state inspection are unavailable.

**Non-Goals:**

- No recovery-state inspection, manifest interpretation, migration, or format graduation.
- No current/historical lineage, custody, protection-basis, checksum-validity, clean-state, or durability claim.
- No candidate export, recovery-plan explanation, damaged/unknown drill, parity repair, parity build, rebuild, publication, or payload mutation.
- No production command composition, shutdown/session lifecycle, startup/currentization, retention/history, or operator private-state dependency.
- No P/Q or multi-profile behavior; the first boundary is single-XOR only.

## Decisions

### Keep detailed semantic ownership in existing requirements

The new requirement owns only independent invocation, descriptor binding, bounded process/report behavior, and the experimental claim boundary.

| Current requirements | Semantic rule | Selected owner | Other relationship | Repair classification | Consequence |
|---|---|---|---|---|---|
| `req.xor-reference-model.xor-parity-uses-explicit-protected-geometry` | Explicit protected geometry, logical ranges, and zero extension define XOR inputs | Existing XOR owner | New tool `requires` it | Preserve; adapter conformance | No geometry policy is restated in the new owner |
| `req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan`; `req.parity-verification-repair.reports-are-bounded-and-preserve-authority-boundaries` | Exact reads produce bounded per-region verification results without writes | Existing parity-verification owners | New tool `requires` them | Preserve; adapter conformance | The tool maps input/output and does not redefine verification |
| `req.evidence-boundaries.evidence-scope-is-explicit`; `req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed` | Evidence tier and uncertainty limit claims | Existing evidence owners | New tool `requires` them | Preserve; evidence constraint | File-backed/model observations cannot authorize stronger action |
| `req.security-boundaries.hostile-inputs-and-resources-are-bounded-before-admission` | Input, path, range, allocation, and report bounds precede work | Existing security owner | New tool `requires` it | Preserve; adapter conformance | Invalid or exhausted bounds refuse before reads |
| None | An independent executable can invoke the above without production private state | `req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing` | New owner composes the rows above | Semantic broadening at an adapter boundary | C8b becomes independently useful without duplicating policy |

The previously existing operator-recovery `scrub` command is not made the owner. Its production assessment/result boundary remains separate; this tool is intentionally independent and does not require `req.operator-recovery.*`, C0a implementation, C8a implementation, or U11 implementation. A parity mismatch is never classified as a repair candidate locally; that remains the responsibility of the canonical integrity/evidence composition.

### Use a bounded experimental descriptor, not recovery state

The command accepts an experimental JSON descriptor with a fixed version marker such as `dwv.independent-parity.v1`. The descriptor contains only:

- single-XOR profile selection;
- protected logical length;
- selected offset and length, or an explicit whole-protected-range request;
- positive region size and maximum region count;
- one distinct bounded data-payload path per data slot;
- one distinct bounded parity-payload path;
- optional output mode selection.

It does not contain array identity, topology epoch, assignment generation, recovery generation, checksum records, writable claims, or migration instructions. Unknown versions and fields that would change the interpretation are unsupported. The tool rejects duplicate paths and aliased opened-file identities before payload reads. The descriptor is an experimental command input, not a stable persistent DiskWeave format.

### Reuse the smallest existing implementation seam

Add the executable beside the existing portable verification code rather than adding a recovery parser, service dependency, frontend dependency, or command framework. The implementation may use the standard library for bounded file reads and existing `dwv-codec` geometry/XOR logic. JSON parsing/rendering may reuse the repository's established serde/serde_json dependencies without introducing a new third-party package. The binary's import/evidence boundary must exclude `dwv-service`, `diskweave` operator code, `dwv-frontend-ublk`, SQLite adapters, and recovery-state inspection.

Each region is read exactly once per configured payload for the selected logical range. A data payload shorter than the declared protected length supplies logical zeros only beyond its declared length; a short read inside its declared length is incomplete. The tool never opens payloads writable and never rewrites the descriptor or result source.

### Make process status and renderings deterministic

The JSON result and human rendering are projections of one bounded semantic result. Use these outcome classes:

- completed observation: all requested regions have a disposition, including `matched`, `mismatched`, `incomplete`, or `unknown`;
- invalid/unsupported input: descriptor or bounds reject before payload reads;
- operational failure: the tool cannot produce the bounded result (for example, output failure).

The implementation must assign fixed process statuses for those classes and test the mapping. A completed observation is not a command failure merely because a region mismatches or is incomplete; the result itself carries the conservative disposition. Every rendering states `experimental`, the file-backed/model evidence tier, and the claims explicitly refused.

### Keep evidence focused and tier-honest

Focused evidence creates disposable data/parity fixtures and records pre/post hashes plus descriptor identity. It covers matching, mismatch, short read, invalid/aliased input, zero-tail geometry, bounds, human/JSON equivalence, deterministic statuses, and absence of service/recovery dependencies. It does not claim hardware durability, filesystem correctness, current protection, historical recovery, repair authorization, or stable format. No existing evidence asset is modified to weaken a claim.

## Risks / Trade-offs

- An input descriptor can describe bytes without proving array identity or generation. The explicit experimental marker, evidence tier, and non-claims make that ceiling visible rather than silently promoting it.
- File-backed reads cannot establish platform durability. The result records the tier and never reports a durable or current claim.
- A full scan can be expensive. Positive range, region-count, buffer, and report bounds refuse unsafe work instead of silently sampling or truncating.
- Adding an executable to the existing verification package keeps the diff small but requires import-graph evidence that the binary does not acquire production or recovery authority. That evidence is a required child task, not an assumption.
- The broader C8/U26b toolchain remains open. This slice must not grow into export, migration, recovery-plan, damaged/unknown drills, or stable-format governance.
