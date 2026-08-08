## Context

The repository now contains portable topology, parity, file-store, service, verification, rebuild, and SQLite recovery seams, but the root package has only a placeholder binary. The architecture handoff defines `dwv` as an administrative boundary and `dwvd` as a separate parity process, while the first macOS-capable path is intentionally file-backed and offline. See `proposal.md` for the motivation and `macos-demo-cli/spec.md` for the observable contract.

## Goals / Non-Goals

**Goals:**

- Provide one runnable `dwv` binary for the experimental macOS file-backed demo.
- Compose existing service, verification, recovery, and store APIs without moving semantic authority into argument parsing.
- Persist demo recovery state through the existing semantic SQLite adapter while keeping data payloads ordinary files.
- Make the end-to-end fixture deterministic, bounded, inspectable, and safe to rerun only through explicit fixture ownership.
- Keep CLI output, plans, and error classes stable enough for acceptance tests while marking the contract experimental.

**Non-Goals:**

- A live `dwvd` service, local control socket, FSKit/DiskImages bridge, macFUSE integration, or Linux frontend.
- A production command tree, stable CLI ABI, stable data format, or general-purpose array manager.
- New parity, recovery, topology, or durability semantics.
- Arbitrary host-path discovery or direct physical-device management.

## Decisions

### Use the existing root package as the first `dwv` binary

Configure the root package to emit the `dwv` binary while retaining the package's workspace role. Keep argument parsing in a small CLI boundary and keep the existing library crates as the semantic implementation. Use the standard library for argument parsing rather than adding a command-line framework for this experimental command surface. Use the workspace's existing JSON dependency directly only at the CLI boundary.

The first command set is deliberately small:

```text
dwv demo init --root <fixture-root> --size <bytes>
dwv demo run --root <fixture-root>
dwv demo status --root <fixture-root>
dwv demo inspect --root <fixture-root>
dwv demo capabilities --root <fixture-root>
dwv demo plan --root <fixture-root>
dwv demo rebuild --root <fixture-root> --plan <plan-file> --confirm <token> [--stop-after <chunks>]
```

`demo init` creates the bounded disposable fixture and its management manifest. `demo run` exercises the complete deterministic scenario. The remaining commands expose useful intermediate boundaries and are independently testable. The command names are experimental and may be replaced by a later control-plane change.

### Keep a small management manifest outside data payloads

Store a versioned fixture manifest under the owned fixture root. It records protected length, block size, logical slot and role identities, relative payload filenames, recovery-state filename, reference/replacement roles, and the CLI contract version. It is management state, not a required member-data format. Every manifest path is resolved relative to the fixture root and validated as a regular file within that root before use.

The manifest is intentionally simpler than a production array configuration. It must not be used as parity or recovery authority; topology and recovery truth remain in the existing semantic objects and recovery store.

### Build initial state through existing semantic APIs

Fixture initialization creates deterministic synthetic data bytes, computes single-XOR parity through the existing codec contract, and writes ordinary bounded files. It constructs the validated topology and recovery manifest through existing core/recovery types, then creates the SQLite-backed recovery store from that semantic manifest. No CLI code computes an alternative parity equation or serializes internal SQLite rows.

`demo run` opens the fixture through the existing healthy service for a small write/read/flush cycle, closes and reopens it, then uses the existing known-erasure authorization and rebuild adapters with the survivor and parity stores plus a distinct replacement target. The workflow persists rebuild receipts through the existing generation-checked recovery adapter and compares the replacement with the preserved reference bytes after all stores close.

The first implementation may use a single deterministic fixture and bounded fixed-size ranges. General array discovery, arbitrary geometry, background scheduling, and daemon lifecycle belong to later changes.

### Use one result envelope and a small exit taxonomy

All commands construct one bounded semantic result before rendering. Successful JSON output contains the versioned contract, command identity, `ok: true`, an outcome, empty-or-bounded diagnostics, and command-specific evidence fields. Errors contain the same contract plus a stable error class, numeric exit class, and bounded message.

Payload bytes, private handles, SQLite statements, and unrestricted host paths are excluded. Pretty JSON is the current human-readable renderer over the same result; a later CLI change may add another renderer without changing the semantic fields.

Use a minimal exit taxonomy: zero for success; a usage/input class for invalid invocation; a blocked/unsupported class for unavailable capability or platform; a refused/uncertain class for stale, ambiguous, or insufficient evidence; and an operation-failed class for an attempted operation that could not complete. The exact numeric values are part of the OpenSpec acceptance tests, not scattered through handlers.

### Make state-changing execution plan-bound

`rebuild plan` writes a bounded plan document under the fixture root. The plan contains the fixture and member identities, source topology epoch, recovery generation, protected ranges, evidence classification, replacement identity, expected persistent-state transitions, and rollback limits. It also contains a confirmation token derived from the canonical plan representation.

`rebuild execute` reads the plan, validates that the plan path is owned by the fixture, reopens current state, compares every identity/generation/range field, and requires the matching token. It then calls the existing rebuild orchestration. A mismatch returns before payload mutation. No handler accepts an implicit target, inferred missing slot, or unbound confirmation.

### Keep daemon interaction deferred but explicit

The first CLI is offline-capable. It does not pretend to be a daemon client. If a later macOS frontend requires `dwvd`, the local protocol is a separate versioned boundary with peer authorization and structured semantic results. The CLI must not use SQLite rows or Rust enum serialization as that future wire protocol.

### Test at the process boundary

Add root-package integration tests that invoke the built `dwv` binary against a unique temporary fixture root. Cover help/usage, JSON envelope and exit classes, fixture initialization, status/inspect/verify, complete demo execution, reopen evidence, plan confirmation mismatch, source/replacement alias refusal, and source/parity preservation. Keep the existing library tests as the detailed semantic coverage; CLI tests prove composition and operator-visible behavior.

## Risks / Trade-offs

- A manual parser is less flexible than a framework, but it avoids a new dependency and keeps the experimental surface small. Replace it only when measured command complexity justifies the change.
- The fixture manifest is additional management state, but it provides deterministic ownership and reproducibility without contaminating member payloads. It must remain explicitly non-authoritative.
- The first demo performs bounded work synchronously and uses a deterministic fixture. This is appropriate for evidence and smoke testing, not a throughput or background-job claim.
- The SQLite adapter's process lease and host `sqlite3` execution remain provisional durability evidence. The CLI must report that boundary rather than promoting the demo to physical power-loss safety.
- A single offline binary gives a usable macOS reference sooner, but it does not prove live virtual block exposure. That claim remains behind the separate bridge/frontend OpenSpec gates.
