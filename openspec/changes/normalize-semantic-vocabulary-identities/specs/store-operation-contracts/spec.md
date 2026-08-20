## MODIFIED Requirements

### Requirement: Store write watermarks are real monotonic evidence
<!-- dwv:req req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->

Each store incarnation and ordering domain SHALL assign a monotonic watermark to every accepted write. Write completion SHALL report the exact assigned watermark. Flush evidence SHALL name the same store incarnation and SHALL report a synchronized-through watermark no greater than the highest write that the store actually synchronized. Sentinel, guessed, future, stale, partial, or cross-store watermarks SHALL NOT authorize a fence, recovery state `CLEAN` transition, or resource release.

#### Scenario: A store synchronizes accepted writes

- **WHEN** writes receive watermarks W1 through Wn and a successful flush synchronizes through Wk
- **THEN** the resulting evidence names that store and Wk, where Wk is an accepted watermark and no later write is implied durable

#### Scenario: Fence evidence cites an invalid watermark

- **WHEN** evidence cites a future watermark, a watermark from another store/incarnation, or incomplete synchronization
- **THEN** fence composition fails closed and affected recovery state remains dirty or indeterminate
