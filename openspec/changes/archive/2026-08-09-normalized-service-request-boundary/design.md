## Context

`dwv-core::BlockRequest` already owns the canonical request semantics, but `dwv-service::PortableRequest` copies most fields, embeds write bytes, replaces `SlotId` with `usize`, forces `FrontendId(0)`, and loses submission identity. `HealthyPortableService` also binds opened members by data/parity vector position even though topology assignments already carry stable slot, coding-position, assignment-instance, and generation identities. `OperationSlotTable` retains only an `OperationId` made from `RequestId` plus epoch and synthesizes buffer identities. The Linux path therefore translates a correct `BlockRequest`, copies the write payload into the duplicate request, and then drops canonical identity fields.

## Goals / Non-Goals

**Goals:**
- Make `BlockRequest` the only service request contract.
- Keep payload ownership separate: writes accept `&[u8]`; reads return owned bytes.
- Bind every opened member explicitly to a topology assignment and resolve by stable slot identity.
- Retain the exact canonical request and its real buffer token in operation-slot state and terminal evidence.
- Preserve current ordering/durability rejection and operation-lifetime behavior.
- Prove reordered member collections, positional mismatch rejection, exact field preservation, stale completion rejection, abandonment ownership, and borrowed ublk payload translation.

**Non-Goals:**
- Generalize codecs, profiles, SQLite, CLI structure, trace formats, or module layout.
- Add asynchronous service APIs or a new buffer framework.
- Implement unsupported FUA, preflush, discard, write-zeroes, or zoned operations.
- Replace backend `OperationId`, which remains the store-operation identity used by `StoreRequest` and is not a second logical request contract.

## Decisions

### 1. One request type, operation-specific payload argument

`HealthyPortableService::{read,write,flush}` accept `BlockRequest` directly. `write` additionally accepts `&[u8]`. Request validation uses the existing `BlockRequest::validate` plus service capability and topology checks. No service-local operation enum, convenience request constructor, or conversion layer remains.

This avoids a lifetime parameter on the canonical request and keeps a frontend-owned ublk buffer borrowed through the current synchronous service call. Read output remains `Vec<u8>` because ownership crosses back to the frontend.

### 2. Exact assignment bindings, order-independent lookup

Replace `MemberStore` and the separate data/parity fields with one `MemberBinding` collection. Each binding contains:

- `SlotId`
- `AssignmentInstanceId`
- `AssignmentGeneration`
- `StoreId`
- `FileStore`

Assembly validates exactly one binding for each topology assignment, no extras, no duplicates, exact assignment instance/generation, compatible store capabilities, current store identity, and no aliasing. Role and coding position remain canonical in `TopologySnapshot`; request execution resolves `request.slot_id` to its assignment and then its binding. Parity selection is derived from the topology's parity role. Collection order is never consulted for semantic resolution.

Checksum extent IDs use `CodingPosition`, not assignment vector position, so reordered snapshots do not alter semantic extent identity.

### 3. Canonical request is operation-slot identity

`OperationSlotTable::reserve` accepts and stores the exact `BlockRequest`. `SlotSnapshot` exposes that request, and the slot acquires the request's exact optional `BufferToken` during atomic reservation. Synthetic service buffer tokens are removed. Backend `OperationId` remains only in `StoreRequest`; child operations remain identified by the stronger generation-bearing `ChildOperationId`.

Abandonment continues to set delivery interest to false without making the slot reclaimable. Child terminal and reconciliation rules remain the only reclamation gate.

### 4. Terminal service evidence includes the request

`OperationEvidence` carries the exact `BlockRequest` alongside completion and transaction trace. Successful read, write, and flush paths return the admitted request unchanged. Operation-slot snapshots provide the same identity for failure, uncertain, abandoned, and reconciled states before reclamation.

### 5. Linux dispatch borrows write bytes

`translate_request` continues to produce `BlockRequest`. `OpenFixture::execute` passes that request directly to the service and passes the ublk write slice separately. A small payload validation helper returns the same slice; its pointer/length test proves the translation boundary does not clone payload storage. The synchronous call keeps the ublk `IoBuf` alive until service completion.

## Risks / Trade-offs

- Linear binding lookup is deliberate: member counts are already bounded and small, and it avoids a second indexed authority. Add an index only if measured profiles require it.
- `OperationSlotTable` becomes explicitly coupled to the canonical core request. This is intended: the slot owns logical operation lifetime, while `StoreRequest` remains the lower backend contract.
- Existing tests and demo callers must construct full `BlockRequest` values. Test helpers may reduce repetition but must not become another public request type.

## Migration Plan

1. Update canonical specs through this change.
2. Introduce `MemberBinding` and order-independent assembly/resolution.
3. Move operation admission to exact `BlockRequest` ownership.
4. Delete `PortableRequest`, `RequestOperation`, constructors, conversion code, and re-exports.
5. Migrate service, demo, fixture, ublk dispatch, and tests to `BlockRequest` plus borrowed payloads.
6. Run focused suites, full workspace validation, trace replay, knowledge/docs gates, and external implementation review.

Rollback is a source revert before archive; no stable persistent format or compatibility shim is retained.

## Open Questions

None. The current canonical specs already choose stable `SlotId`, assignment identity, generation, and synchronous portable service behavior.
