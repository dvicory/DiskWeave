# Concepts and Terminology

This page keeps the product's consequential distinctions searchable. It is a
maintained explanatory projection, not a requirements registry and not semantic
authority. Current OpenSpecs own behavior. The delegated
`RecoveryProtocol` model owns only the parameterized state, abstract actions,
guards, outcomes, invariants, and release relation it declares.

## One write, several facts

| Fact | What it establishes | What it does not establish | Owner |
| --- | --- | --- | --- |
| Submitted write | DiskWeave accepted a request at its declared scope. | That any bytes were written or persisted. | `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics` |
| Completed write | The operation reached its declared completion result. | Persistence, recovery `CLEAN`, or range release. | `req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred` |
| Durable data/parity write | The represented write has the delegated relation's bounded durable-write outcome. | The concrete store evidence needed for a product claim. | `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent` |
| Recovery `CLEAN` | The recovery owner accepted current evidence for the affected state. | Ownership release or proof that every broader claim is valid. | `req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence` |
| Released range | The owning lifecycle permits reuse of the range. | A new write's admission or its durability. | `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent` |

A write-recovery record is the durable prerequisite for a protected
data/parity write. It records dirty regions and invalidated integrity evidence
before that write when existing coverage does not already satisfy the
requirement. It does not mean that the data/parity write occurred, became
durable, made recovery `CLEAN`, or released the range.

## Evidence has an owner and a scope

Persistence evidence is not a generic success flag. A claim may need evidence
for the exact store incarnation, ordering domain, accepted and synchronized
watermarks, topology epoch, affected range or region, capability, and relevant
generations. The owner rejects evidence that is missing, volatile, future,
stale, partial, cross-store, or mismatched.

A low-level fence or flush remains a mechanism. It becomes claim-supporting
persistence evidence only when the owning requirement accepts its scope and
bindings. I/O completion alone is not persistence evidence.

## Unknown is not failure

A known rejection can leave the prior semantic state authoritative. A lost,
corrupt, or otherwise unclassifiable commit observation establishes no safe
result for a dependent write and requires reconciliation. An indeterminate
data/parity effect remains visible; it is not silently converted into a known
failure, a clean state, or proof that no write occurred.

## Ownership outlives request delivery

Frontend request interest, backend operation-slot lifetime, correctness-work
lifetime, and range ownership are separate. Abandoning a response does not
cancel an operation that may still complete. An interrupted write remains owned
until its allowed reconciliation and completion or abort predicates finish.
A completed or aborted write still requires explicit `releaseRange` before the
range can be reused.

Reconciliation examines the allowed evidence and determines what DiskWeave may
safely claim next. It is not retry, rollback, repair, or a substitute for
completion.

## The delegated model boundary

`models/quint/RecoveryProtocol.qnt` describes one parameterized write relation
with explicit write-recovery-record state, data/parity-write state, recovery
state, write lifecycle, observations, attempted and known-applied regions,
persistence-evidence observations, recovery `CLEAN`, and explicit range
release. `verification/quint/RecoveryProtocolAnalysis.qnt` instantiates two
regions and two stores for bounded evidence. The declared checker depth is 12.
The Connect projection separately exercises release followed by reuse.

These checks establish only the declared finite model and bridge evidence. They
do not establish arbitrary-width recovery, Rust implementation correctness,
exact concrete range or checksum mapping, persistence-engine behavior,
operation-slot lifetime, frontend behavior, or physical durability.

## Maintenance rule

Each entry on this page points to its current semantic owner. When an owner
changes, review only the affected entry and its provenance. Do not regenerate
unaffected entries wholesale. If a concept is not supported by a current
requirement or the delegated model's declared surface, record the gap instead
of inventing a definition.

## Traceable requirements

```{needlist}
:filter: "type == 'req' and capability in ['documentation-knowledge-architecture', 'architecture-contract', 'explicit-transaction-machine', 'recovery-state-semantics', 'store-operation-contracts']"
```

**Provenance:** `req.documentation-knowledge-architecture.concepts-and-terminology-is-maintained-incrementally`; `req.architecture-contract.durable-authority-and-uncertainty-are-not-inferred`; `req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent`; `req.recovery-state-semantics.clean-and-valid-claims-require-persistence-evidence`; `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`; model `models/quint/RecoveryProtocol.qnt`; finite evidence `verification/quint/RecoveryProtocolAnalysis.qnt`.
