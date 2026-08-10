## Why

The canonical `BlockRequest` already owns normalized request meaning, but the portable service still accepts a second, weaker request model and selects members by collection position. This can discard canonical identity, ordering, durability, and topology fields, redirect I/O when member order changes, and copy Linux write payloads at the frontend/service boundary.

## What Changes

- **BREAKING** Make `dwv_core::BlockRequest` the only semantic request accepted by portable service read, write, and flush entry points; pass write payload bytes separately by borrow.
- **BREAKING** Delete `PortableRequest`, `RequestOperation`, their conversion/constructor paths, and obsolete exports after every caller is migrated.
- Bind opened stores explicitly to validated topology assignments by stable slot, role, coding position, assignment instance/generation, topology epoch, and store identity; never infer semantic identity from collection or discovery order.
- Preserve every canonical request field through validation, admission, operation ownership, and terminal evidence while keeping frontend/request identity distinct from internal operation-slot and child-operation identity.
- Preserve preflush and durability as independent intents, reject unsupported operations or intent before mutation, and preserve abandonment as loss of delivery interest without cancelling admitted semantic I/O or releasing resources early.
- Remove the Linux frontend request-boundary write-payload copy while retaining the frontend buffer through terminal reconciliation.
- Add deterministic conformance coverage for reordered members, positional mismatch, canonical fields, identities/generations, unsupported operations/intents, abandonment points, stale completion, payload mismatch, and copy-free frontend translation.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `normalized-block-semantics`: Require admitted canonical request fields to remain inspectable in terminal evidence.
- `linux-ublk-frontend`: Require write translation to borrow the bounded frontend payload instead of creating a duplicate semantic-request payload.

## Impact

- Affected crates: `dwv-core`, `dwv-store`, `dwv-service`, and `dwv-frontend-ublk`, plus direct CLI/demo callers and conformance tests.
- Existing persistent formats and recovery authority are unchanged.
- Existing canonical topology, operation-slot lifetime, ordering/durability, abandonment, healthy-service composition, and Linux adapter owners remain authoritative; this change primarily corrects implementation nonconformance and adds two bounded owner-local requirements.
- No new dependency, runtime framework, compatibility shim, generic request wrapper, formal model, production SQLite claim, or Milestone 8 planning cutover is introduced.

## Non-Goals

- Changing parity/update/recovery algorithms, persistent schemas, metadata-certificate authority, production durability claims, or Linux publication scope.
- Replacing per-chunk service write buffers used by the transaction engine.
- Refactoring unrelated module layout, identity types, codecs, traces, CLI structure, or documentation machinery.
- Archiving this change or beginning `milestone-planning-cutover` before external implementation review.
