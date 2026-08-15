## MODIFIED Requirements

### Requirement: Kernel tags and operation resources remain bounded and generation-safe
<!-- dwv:req req.linux-ublk-frontend.kernel-tags-and-operation-resources-remain-bounded-and-generation-safe -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->

The frontend SHALL publish finite queue, depth, transfer, buffer, operation-slot, child-operation, trace, and shutdown bounds. Its trace SHALL be a fixed-capacity rolling retention window of 4,096 records: before semantic admission or protected mutation, each request SHALL reserve a trace slot, and admission SHALL fail explicitly before semantic admission when no slot can be safely reserved. A retained record SHALL be retired and reused only after its represented operation is fully terminal and the operation, tag, buffer, child-operation, durability, recovery, and reconciliation owners have released their resources under their own contracts; trace retirement SHALL never evict an incomplete reservation or release any live ownership. Stale or duplicate completions SHALL NOT complete a reused tag or reclaim live resources. The retained window SHALL preserve each retained record's original deterministic sequence and normalized request/terminal mapping even when its first sequence is greater than one. Export SHALL disclose when earlier completed records were retired and SHALL identify the result as partial-session evidence rather than claiming complete-session replay. Retired-record accounting SHALL be bounded and saturating with an explicit saturation indication; saturation alone SHALL neither refuse ordinary service nor imply that the trace has a further lifetime ceiling. Shutdown or crash with an incomplete reservation SHALL remain non-reclaimable and SHALL produce an explicit incomplete or reconciliation-required disposition rather than fabricated terminal evidence. The existing experimental trace schema MAY migrate or refuse explicitly, but this requirement makes no stable-format or compatibility guarantee. Deterministic adapter evidence SHALL cover admission exhaustion, explicit abandonment, stale and duplicate completion, renewable trace retirement, partial-session export, replay of a retained window beginning after sequence one, accounting saturation, and incomplete-reservation shutdown or crash.

#### Scenario: Admission is exhausted

- **WHEN** no configured tag, buffer, or semantic operation slot is available
- **THEN** the request is backpressured or fails with bounded resource exhaustion before protected mutation

#### Scenario: A stale completion names a reused tag

- **WHEN** a completion carries an earlier generation than the tag's current admitted operation
- **THEN** it is rejected without completing or reclaiming the current operation

#### Scenario: An admitted request is abandoned

- **WHEN** frontend ownership ends before a safe terminal completion is observed
- **THEN** the tag remains unavailable until explicit reconciliation releases it and the trace records an abandoned terminal result

#### Scenario: The 4,097th ordinary request arrives after terminal records fill the window

- **WHEN** 4,096 retained records are present, every record eligible for retirement is fully terminal with its owned resources safely released, and another ordinary request is submitted
- **THEN** the frontend retires and reuses one safe record slot before semantic admission, admits the new request under the same finite bound, and preserves the new record's original sequence without refusing solely because the lifetime count has reached 4,096

#### Scenario: Only a terminal record can be reused

- **WHEN** the trace window contains active or incomplete reservations together with at least one fully terminal record whose operation-resource reconciliation is complete
- **THEN** only the fully terminal record may be retired and reused; every active or incomplete reservation remains retained and all live operation, tag, buffer, child, durability, recovery, and reconciliation ownership remains intact

#### Scenario: The trace bound is exhausted

- **WHEN** the fixed trace capacity is occupied by incomplete reservations, non-terminal operations, or terminal records whose required ownership or reconciliation has not completed
- **THEN** the frontend returns explicit bounded resource exhaustion before semantic admission or protected mutation and does not evict, truncate, or reinterpret any existing record

#### Scenario: Retired records are disclosed as a partial session

- **WHEN** an export is produced after one or more completed records have been retired from the rolling window
- **THEN** the export reports the bounded retained window and explicitly states that earlier completed records were retired, so replay evidence is partial-session evidence and does not claim complete-session coverage

#### Scenario: Replay begins with a sequence greater than one

- **WHEN** a retained trace window starts at sequence 2 or any later original sequence and all retained records pass normalized-request and terminal-mapping checks
- **THEN** replay preserves the retained original sequence values, deterministically validates the retained records in order, and succeeds for that retained window without treating the absent earlier prefix as a divergence or silently renumbering records

#### Scenario: Retired-record accounting saturates

- **WHEN** the bounded count of retired records reaches its representable maximum and later requests continue to find safely reclaimable terminal records
- **THEN** export or status exposes an explicit saturated accounting state, later terminal records remain reusable, and ordinary service continues without refusal caused only by the saturated counter

#### Scenario: Shutdown or crash leaves an incomplete reservation

- **WHEN** shutdown or process loss occurs while a trace reservation has not received terminal evidence
- **THEN** the reservation remains non-reclaimable, the export or shutdown result reports incomplete or reconciliation-required state, and no fabricated terminal record, ownership release, clean complete-session claim, or slot reuse is produced

#### Scenario: A retained trace is replayed after retirement

- **WHEN** a bounded retained frontend trace is imported after earlier completed records have been retired
- **THEN** replay revalidates every retained normalized request and terminal mapping in original sequence order without backing I/O, reports the first retained-record divergence, and preserves the explicit partial-session disclosure
