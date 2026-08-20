# DiskWeave Guide

DiskWeave is a parity engine for ordinary block-image files. It keeps data members
independently readable, stores parity separately, and uses explicit recovery state
to decide when reconstructed bytes are safe to return or write. It is not a
filesystem, and the repository does not yet provide a production virtual disk.

The shortest useful mental model is:

```text
data0.raw ─┐
data1.raw ─┼─ normalized reads and writes ── DiskWeave service
parity.raw ┘                                  │
                                              └─ recovery.sqlite3
```

- `data0.raw` and `data1.raw` are conventional member images.
- `parity.raw` contains protection bytes, not user files.
- `recovery.sqlite3` records operational authority; deleting it does not make the
  data images proprietary or unreadable.
- A successful operation, durable bytes, valid integrity evidence, and permission
  to repair are separate facts.

Read these chapters in order:

```{toctree}
:maxdepth: 1

guide/01-write-before-crash
guide/02-recovery-uncertainty
guide/03-parity-and-integrity
guide/04-boundary-and-demo
```

The first three explain the safety model with concrete byte examples. The last
chapter shows the exact disposable workflow that works today and lists what it
does not prove.

```{note}
This Guide is explanatory. Canonical OpenSpecs remain authoritative; each chapter
ends with the requirement and executable-scenario provenance used for its claims.
```

## Look up evidence or implementation

- [Requirements](requirements.md) — current canonical requirement projections;
- [Scenario Book](scenarios.md) — executable fixtures, events, and forbidden
  inferences;
- [Assurance Atlas](assurance-generated.md) — evidence scope, fault models, and
  explicit non-claims;
- [Contributor Map](contributors-generated.md) — Cargo package boundaries;
- [Rust source trace](source-trace.md) — sparse requirement-to-source links.

