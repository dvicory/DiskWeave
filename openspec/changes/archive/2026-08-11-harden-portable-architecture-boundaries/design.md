## Context

See `proposal.md` for motivation. Current semantic owners already require mechanism-independent portable behavior, exact store outcomes, recovery adapters behind `RecoveryStateStore`, and frontend conformance. The implementation diverges at three edges: the healthy service embeds `FileStore`, portable recovery exports SQLite evaluation types, and live ublk composition names `SqliteRecoveryStore`. `RandomAccessStore` exists but its token-only payload methods require concrete `FileStore` buffer registration/extraction, so the service bypasses the port.

The current service and file adapter are synchronous and runtime-independent. Operation slots own normalized requests, generational buffer tokens, child identities, admission accounting, terminal evidence, and reconciliation state. This change must preserve that ownership without designing the future async/raw-device executor.


## Goals / Non-Goals

**Goals:**

- Make the healthy service portable over the existing store semantics without losing exact effects or adding avoidable payload copies.
- Keep adapter selection at composition edges and SQLite-specific evidence in the SQLite adapter boundary.
- Complete a clean internal API cutover with no aliases or compatibility shims.

**Non-Goals:**

- Split or otherwise reorganize the high-fan-in store hub as a standalone objective.
- Select the pre-operator versus operator-result phase boundary or change CLI behavior.
- Design the future async/raw-device executor or a universal effect system.
- Add production backends, runtime polymorphism, a daemon/RPC boundary, or a new crate solely for layering.
- Change persistent formats, recovery authority, durability semantics, supported ublk profiles, or demo claims.
- Generalize disposable fixture composition into a product framework.

## Decisions

### 1. Use static service composition over the current store and recovery ports

`HealthyPortableService` will be statically parameterized by one store implementation and one recovery implementation. All members in one admitted service use the same store implementation in the current product path. File-backed production composition and a deterministic test store instantiate the same service behavior.

This keeps dispatch static and confines generic parameters to the service and composition edges. It avoids `Arc<dyn ...>`, boxed futures, factories, and a service locator.

**Alternatives considered:**

- **Keep concrete `FileStore`:** rejected because it violates the portable service contract and prevents a real conformance test through the store port.
- **Add a new application-facing store trait:** rejected because `RandomAccessStore` already owns the semantic contract.
- **Use trait objects:** rejected because the implementation set is known at composition time and runtime polymorphism adds allocation/object-safety work without a current need.
- **Use a production adapter enum:** rejected for the service because there is only one production store adapter today; the deterministic adapter is test evidence, not a runtime choice.

### 2. Pass payload bytes by borrow while operation slots retain semantic ownership

The synchronous store call will receive the exact source or destination slice needed for the child range. The operation slot continues to retain the canonical request and generational `BufferToken`; child identity and completion evidence continue to bind the effect. The store adapter does not become the owner of a private payload registry.

Reads allocate the service's final result buffer once and lend exact subranges to child reads. Writes lend exact caller payload subranges. Adapters report completed ranges and persistence evidence; they do not return success merely because a slice was available.

This removes the current concrete `register_buffer`/`take_read_buffer` side channel and the file adapter's avoidable write-payload clone.

**Alternatives considered:**

- **Add buffer insertion/extraction methods to the store trait:** rejected because it makes every store own a second payload registry and preserves the avoidable clone.
- **Make the store return `Vec<u8>`:** rejected because it conflates data transfer with completion evidence and adds per-child allocation.
- **Introduce an executor/buffer-pool framework now:** rejected because current synchronous consumers do not require it. A future asynchronous backend must establish its own explicit ownership design rather than inheriting a speculative one.

### 3. Relocate concrete SQLite evaluation vocabulary, not portable outcomes

SQLite journal, synchronization, checkpoint, connection, reset, and SQLite failure candidates move to `dwv-recovery-sqlite`. Generic semantic outcomes such as durable/rejected/lost/corrupt commit observation, recovery health, recovery disposition, and mechanism-independent recovery fault schedules remain in `dwv-recovery`.

The existing SQLite candidate matrix continues to provide adapter evaluation evidence. Moving its types does not select a production mode or broaden durability claims.

The methodless `RecoveryStateAdapter` blanket marker is removed. `RecoveryStateStore` remains the sole semantic recovery port.

### 4. Make both ublk paths call one generic admitted-service runner

The live production entry accepts an already-admitted `HealthyPortableService<S, R>` and does not name SQLite. Disposable fixture open continues to compose `FileStore` and `SqliteRecoveryStore`, but then yields its admitted service plus fixture metadata to the same generic runner.

The runner owns only frontend translation, queue/tag/buffer lifecycle, publication metadata, and shutdown. The root binary remains responsible for production file/SQLite selection. Fixture composition remains explicitly acceptance-only and cannot authorize production start.

**Alternative considered:** introduce a `BlockService` trait and box it. Rejected because both current paths already use the same service type shape; a second service abstraction would be a single-implementation port.

## Semantic ownership reconciliation

| Cluster | Current requirements | Semantic rule | Selected owner | Other requirements | Contradiction resolved | Semantic edit classification | Stable-ID consequence | Scenario/evidence migration |
|---|---|---|---|---|---|---|---|---|
| Portable store/service edge | `req.architecture-contract.portable-semantics-are-independent-of-implementation-mechanisms`; `req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence`; `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`; `req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe` | Portable composition consumes exact store effects while operation slots retain lifetime authority. | Store-operation requirements own store effects/lifetimes; healthy I/O composes them. | Architecture contract remains constitutional; service and adapters conform. | Implementation imported and bypassed the concrete file adapter despite the portable owners. | No normative edit; fix implementation nonconformance. | All IDs preserved. | Add service conformance through deterministic and file adapters; retain existing owner evidence. |
| Recovery adapter boundary | `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout`; `req.recovery-state-semantics.sqlite-remains-an-evidence-driven-adapter-decision`; `req.recovery-state-semantics.evaluation-fixtures-cover-candidate-durability-and-reset-boundaries`; `req.recovery-state-semantics.the-sqlite-prototype-remains-evaluation-only-and-storage-independent-at-the-semantic-boundary` | Portable recovery owns semantic outcomes; SQLite owns candidate configuration/evaluation mechanisms. | Recovery-state semantics own outcomes; SQLite requirements refine adapter evaluation. | SQLite adapter implements and evidences; portable recovery does not export mechanism types. | SQLite-specific public types were located in portable recovery. | No normative edit; relocate implementation with semantics preserved. | All IDs preserved. | Move fixture/evaluation code and evidence references only where ownership actually moves. |
| Linux live composition | `req.normalized-block-semantics.adapters-expose-bounded-deterministic-conformance-behavior`; `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics`; `req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow` | Frontend maps an admitted portable service and owns platform resources; it does not select recovery authority. | Linux frontend owns translation/publication; healthy service owns admission. | Root composition selects concrete adapters; fixture remains acceptance-only. | Live frontend type selected SQLite despite receiving admitted semantics. | No normative edit; fix adapter conformance. | All IDs preserved. | Preserve current Linux evidence and add compile-time/type-level decoupling checks. |

## Risks / Trade-offs

- **[Risk] Two generic parameters propagate through frontend composition.** → Keep them on service/runner boundaries only; do not genericize unrelated domain types or box them to hide readable static composition.
- **[Risk] Borrowed payload slices fit current synchronous stores but not a future asynchronous backend.** → Scope the design honestly to the current runtime-independent synchronous port; require the future executor milestone to model owned/pinned lifetimes explicitly.
- **[Risk] A partial migration could preserve both concrete and portable I/O paths.** → Use a clean cutover, migrate every caller, and delete concrete buffer side channels once focused tests pass.
- **[Risk] Moving SQLite evaluation types could accidentally weaken required evidence.** → Move tests and candidate matrices with the types; validate every declared reset/failure disposition after relocation.
- **[Risk] Generic ublk runner changes thread-safety bounds.** → Derive only the bounds required by the existing queue/thread ownership and prove the current file/SQLite instance compiles and runs; do not add `Arc` or cloning to satisfy bounds blindly.
- **[Trade-off] Internal Rust APIs break.** → Early development explicitly permits clean breaks; migrating all callers is smaller and safer than compatibility scaffolding.

## Migration Plan

1. Establish focused tests for the current file-backed service outcomes.
2. Make the store payload boundary usable and add the deterministic service adapter.
3. Migrate service/member/rebuild/degraded callers and remove the service-to-file-store dependency.
4. Relocate SQLite evaluation vocabulary and remove the unused marker trait.
5. Convert fixture and live ublk paths to the admitted-service runner, then remove concrete recovery selection from the live path.
6. Run focused, workspace, OpenSpec, knowledge, documentation, and architecture checks.

Rollback during development is by reverting the complete uncommitted change. There is no persistent migration or compatibility mode to roll back at runtime.
