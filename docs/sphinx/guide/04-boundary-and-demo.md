# What works today

The repository has a runnable disposable file-backed workflow. It does not attach a
virtual disk, but it exercises the portable storage path with ordinary files.

From the repository root:

```sh
ROOT="$(mktemp -d /tmp/dwv-demo.XXXXXX)"
cargo run -q --bin dwv -- demo init --root "$ROOT"
cargo run -q --bin dwv -- demo run --root "$ROOT"
```

`demo init` creates:

```text
data0.raw          ordinary data-member image
data1.raw          ordinary data-member image
parity.raw         single-XOR parity
reference.raw      known-good disposable comparison
replacement.raw    empty rebuild target
recovery.sqlite3   semantic recovery state
fixture.json       versioned fixture identities and geometry
```

The current fixture protects 16 KiB with 512-byte logical blocks.

## What `demo run` actually exercises

The command uses the real file stores, service, recovery, and rebuild paths to:

1. perform a healthy write, read, and host-file flush;
2. close and reopen the fixture and confirm the bytes still match;
3. treat one data member as a known erasure;
4. perform an authorized degraded read and compare it with `reference.raw`;
5. rebuild four chunks into `replacement.raw`;
6. read back and verify the replacement;
7. confirm the replacement is byte-equal to the reference;
8. confirm source parity was not changed.

On the current repository state it reports:

```text
healthy read                 matched
healthy write                success
reopen                       matched
degraded read                matched reference
rebuild chunks               4
final replacement check      passed
source parity                preserved
replacement                  byte-equal to reference
```

Because the data members are ordinary images, they remain directly readable after
the DiskWeave service closes. The recovery database is control authority, not a
hidden format required to interpret their payload bytes.

## What this does not provide

The same command explicitly reports these unsupported claims:

- a live FSKit/DiskImages bridge;
- a Linux frontend;
- certified physical power-loss durability;
- P/Q parity;
- degraded writes.

The demo is therefore useful for developing and inspecting the portable semantic
core. It is not yet a mountable storage product. A simulator pass or host-file
flush cannot be promoted into platform or hardware evidence.

## Where to inspect details

- [Scenario Book](../scenarios.md) — fixture events and forbidden inferences;
- [Assurance Atlas](../assurance-generated.md) — evidence scope and non-claims;
- [Rust source trace](../source-trace.md) — linked implementation owners;
- `docs/verification/macos-demo-v0.md` — the longer operator workflow, including
  resumable rebuild commands.

## Traceable requirements

```{needlist}
:filter: "type == 'req' and capability in ['evidence-boundaries', 'normalized-block-semantics', 'macos-demo-cli']"
```

**Provenance:** `req.architecture-contract.diskweave-protects-conventional-member-images-at-block-level`; `req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms`; `req.healthy-portable-io.portable-members-remain-ordinary-and-control-state-is-disposable`; `req.normalized-block-semantics.portable-evidence-does-not-imply-platform-certification`; `req.macos-demo-cli.the-offline-demo-exercises-healthy-and-recoverable-behavior`; `req.evidence-boundaries.evidence-scope-is-explicit`; scenario `scenario.normalized-recovery`.

