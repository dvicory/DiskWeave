# OS-011: Data, parity, and checksum plane

## 1. Architecture decisions and target gate

Implement D-009, D-010, D-011, D-021 and P-007’s migration seam for Gate E. Resolve V-009 only through benchmark evidence; do not freeze F-007 in this change.

## 2. Concrete outcome

Recovery state can accept a digest only for a named target extent, content generation, profile/set generation, and fenced durable bytes.

## 3. Prerequisites

Use OS-010 invalidation, OS-003 geometry, OS-005 persistence/export, and store capabilities from OS-002/012. The implementation can begin with an in-memory provider and add the selected BLAKE3 provider behind a trait after dependency/benchmark review.

## 4. Exact scope and non-scope

Add integrity modules, provider trait, records, worker scheduling semantics, and tests. Do not implement repair, scrub policy, format freeze, or a filesystem/front-end adapter.

## 5. Semantic APIs and contracts

Split the recovery/integrity code into profile, extent, record, provider, job, and migration modules. Keep `DigestProvider` synchronous and portable at the semantic seam; worker threading is an implementation detail. Use fixed-size digest values and checked range arithmetic.

## 6. State ownership and lifecycle

The durable store owns records and active set; the worker pool owns ephemeral jobs. A job captures all evidence at submission and can commit only the matching current record. Invalidation increments or replaces the content generation before mutation.

## 7. Persistent-state impact

Add serializable semantic fields with explicit algorithm/profile IDs, lengths, generations, and fence references. Preserve unknown profiles as inspectable unsupported records, not as valid current evidence. Keep control metadata separate from recovery authority.

## 8. Irreversible and durability boundaries

The invalidation boundary is OS-010’s pre-home intent. The validity boundary is a fenced target read plus a durable record commit. If either proof is missing, the result remains stale. The worker may never clear dirty state itself.

## 9. State and sequence diagrams

```text
invalidate -> stale record -> read extent -> digest -> target fence
                                             -> generation check
                                             -> durable VALID commit
```

Every arrow is a normalized trace event and can be interrupted in simulator tests.

## 10. Concurrency and resource rules

Use a deterministic key `(target, extent, profile, set generation)`. A bounded queue and buffer pool prevent checksum work from starving foreground I/O. A job result is idempotent for the same commit token and rejected after invalidation or set switch.

## 11. Failure matrix

Model read, hash, fence, generation, recovery-commit, crash, and migration failures as typed results. Preserve stale evidence for every incomplete path and keep old active sets usable during migration.

## 12. Deterministic simulator cases

Create cut points around read/fence/commit and race a writer invalidation against each job phase. Include power loss and restart with partially written metadata. Add deterministic known vectors for empty, partial, full, and zero-filled extents.

## 13. Property, model, and fuzz tests

Test digest/provider equivalence, extent partitioning, generation checks, invalidation monotonicity, set migration, bounded queues, and no-valid-without-fence. Fuzz record decoding and profile/extent parameters without accepting oversized allocations.

## 14. Integration tests

Run against in-memory recovery and file-backed stores. Compose data and parity target IDs with OS-003 reference vectors. Keep a future Q target fixture without requiring dual parity code.

## 15. Observability, security, and operator behavior

Emit job lifecycle and coverage summaries with no payloads. Treat unsupported algorithms as stale/inspectable. Exports carry algorithm IDs and lengths so independent tools can verify them.

## 16. Performance and resource bounds

Benchmark a provisional 4 MiB extent and 32-byte digest, comparing provider implementations, queue sizes, metadata overhead, and full-overwrite/readback paths. Bound all worker memory and preserve foreground priority.

## 17. Executable acceptance criteria

Use the spec’s generated races and crash schedules, golden vectors, workspace tests, format/dependency checks, and OpenSpec validation. Record V-009 evidence without promoting a provisional profile to stable format.

## 18. Forbidden outcomes

No stale result may become valid, no volatile completion may be treated as a fence, and no checksum record may authorize repair by itself. No provider identity may become an on-media format identity without an explicit profile.

## 19. Migration and compatibility consequences

Parallel checksum sets allow profile migration. Unknown/new profile records remain readable as metadata but do not authorize decisions. Recovery metadata loss reconstructs stale coverage and requires a new baseline.

## 20. Next OpenSpecs unlocked

OS-013 healthy I/O, OS-014 exhaustive verification, and OS-017 scrub/repair consume this seam. A later change may resolve BLAKE3/profile selection from V-009.
