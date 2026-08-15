## Why

The Linux frontend's bounded trace currently behaves as a lifetime ceiling: once all 4,096 record positions have ever been reserved, later ordinary requests are refused even when completed records could be safely retired. Make the existing frontend resource contract renewable now so bounded diagnostics remain available across a long-lived session without weakening operation ownership, replay determinism, or conservative shutdown behavior.

## What Changes

- **Modify** the existing Linux frontend resource requirement so trace admission reserves a fixed-capacity rolling window before semantic admission or mutation.
- Permit reuse only of a fully terminal trace record whose operation, tag, buffer, child, durability, recovery, and reconciliation ownership is already released by its owning contracts; never evict an incomplete reservation or an in-flight record.
- Refuse explicitly before semantic admission when no trace slot is safely reclaimable, while accounting for retired completed records with a bounded saturating counter and an explicit saturation state that does not become a second lifetime service ceiling.
- Define retained-window export and replay semantics: preserve exact deterministic sequence and normalized replay when the first retained sequence is greater than one, and state explicitly when earlier completed records were retired so a partial-session export is not presented as complete-session replay.
- Cover incomplete reservations during shutdown or crash conservatively; export may report reconciliation-required rather than fabricating completion or releasing ownership.
- Allow the existing experimental frontend trace schema to migrate or refuse explicitly as implementation requires; make no stable-format or compatibility guarantee.
- Keep this change limited to the existing `linux-ublk-frontend` capability. It does not add durable trace segmentation, a full audit history, a generic retention/GC/lifecycle framework, or a new security-boundary policy.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `linux-ublk-frontend`: modify the existing `kernel-tags-and-operation-resources-remain-bounded-and-generation-safe` requirement so its finite trace bound is renewable without changing the ownership rules for operation resources or normalized request semantics.

## Impact

The planned implementation will be localized to the Linux frontend trace/reservation path (`crates/dwv-frontend-ublk/src/trace.rs` and its admission/export flow in `linux.rs`) and its focused evidence. The delta composes with normalized request semantics, operation-slot lifetime and generation ownership, frontend abandonment semantics, and the cross-cutting bounded-admission constraint; those requirements remain owners of their respective policies. No canonical `openspec/specs/*` file or product code is changed by this planning artifact.
