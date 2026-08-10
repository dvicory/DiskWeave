## 1. Canonical request ownership

- [x] 1.1 Change operation-slot reservation and snapshots to retain the exact `BlockRequest` and real optional buffer token atomically.
- [x] 1.2 Add operation-slot regressions for every canonical field, abandonment ownership, stale generation, duplicate completion, and safe reclamation.
- [x] 1.3 Change `OperationEvidence` to expose the exact admitted canonical request.

## 2. Stable member binding

- [x] 2.1 Replace positional `MemberStore` assembly with explicit `MemberBinding` identity fields and one member collection.
- [x] 2.2 Validate exact assignment coverage and reject missing, extra, duplicate, stale, mismatched, or aliased bindings before admission.
- [x] 2.3 Resolve request targets and parity members through topology slot/role/coding identities and make checksum extent identity independent of assignment order.
- [x] 2.4 Add multi-data reordered-member and positional-mismatch regressions proving correct store selection and fail-closed behavior.

## 3. Request model cutover

- [x] 3.1 Change service read, write, and flush APIs to accept `BlockRequest`; pass write bytes separately as `&[u8]`.
- [x] 3.2 Preserve every canonical request field through validation, admission, operation execution, and returned evidence.
- [x] 3.3 Delete `PortableRequest`, `RequestOperation`, conversion constructors, and re-exports after migrating every caller.
- [x] 3.4 Migrate demo, fixture, service tests, and other callers without compatibility wrappers.

## 4. Linux frontend boundary

- [x] 4.1 Dispatch translated `BlockRequest` directly to the service and retain the ublk buffer through synchronous completion.
- [x] 4.2 Remove the request-boundary write payload clone and add a pointer/length regression for borrowed payload translation.
- [x] 4.3 Exercise unsupported operations, preflush, FUA, malformed ranges, invalid payload length, and flush behavior through the canonical boundary.

## 5. Reconciliation and validation

- [x] 5.1 Resolve every effective stale reviewed requirement individually and verify ownership/readiness gates.
- [x] 5.2 Run strict OpenSpec validation, focused crate tests, ublk tests, CLI demo integration, trace replay where applicable, workspace tests, formatting, Clippy, docs projections, and clean-room reconstruction.
- [x] 5.3 Record exact API before/after, stable binding, field preservation, ordering/durability, abandonment/resource, reordered-member, payload-copy, relationship, and validation evidence in the implementation review.
- [x] 5.4 Leave the change active and unarchived pending external approval; do not start milestone-planning-cutover.
