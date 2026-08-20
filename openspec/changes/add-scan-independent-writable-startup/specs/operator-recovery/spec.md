## MODIFIED Requirements

### Requirement: Start composes admission and actual publication
<!-- dwv:req req.operator-recovery.start-composes-admission-and-actual-publication -->
<!-- dwv:requires req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe -->
<!-- dwv:requires req.healthy-portable-io.scan-independent-writable-startup-admission-is-complete-and-conservative -->
<!-- dwv:requires req.linux-ublk-frontend.the-initial-linux-publication-profile-is-complete-and-narrow -->

Production start SHALL re-observe and validate current identity, topology, recovery, checksum, and admission prerequisites before invoking the selected frontend. Default start SHALL require current read-write admission. An explicit read-only request SHALL succeed only through a supported read-only publication path and SHALL NOT relabel missing recovery authority, incomplete verification, invalid checksum coverage, or other read-write blockage as read-only service. The command SHALL report the array online only after the frontend reports successful owned endpoint publication. A valid topology outside a frontend's narrow profile SHALL remain unsupported rather than topology-invalid.

After a normal custody gap, current read-write admission SHALL include the complete scan-independent startup result: all-or-nothing prerequisite store-writer and writable-recovery-authority reacquisition before durable session begin from the reviewed active `define-portable-writable-session-lifecycle` target; no pre-begin session authority; acquisition and continuous holding/fresh revalidation of the exact current claim token and stable backing-resource/alias binding for every required store and writable-recovery authority; successful typed store stabilization bound to each selected capability/profile and its declared and evidence-supported physical reachability universe; exact checksum-owner comparison; the recovery-produced range-role basis snapshot; separate coded-range-authority and recovery `CLEAN` owner observations preserved without combination or relabeling; durable session begin; durable new protection-epoch admission; and continued authority/binding revalidation from the observed epoch commit through admission return, frontend handoff, and successful publication. The physical no-late result proves quiescence only for covered pre-boundary effects within the selected profile's universe; it is not a persistence fence and makes no claim about routes or actors explicitly outside that universe.

The shared operator result SHALL preserve accepted lineage separately from custody continuity, admission authority, exact per-store current claim/incarnation/domain bindings, stable backing-resource identities and alias sets, writable-recovery authority held and freshly revalidated from the observed epoch commit through admission return, frontend handoff, and successful publication, predecessor physical-reachability/closure scopes, exact checksum-owner evidence, the recovery-produced range-role basis snapshot and source bindings, separate coded-range-authority and recovery `CLEAN` capture/state/uncertainty observations, session-begin and epoch evidence, frontend publication, and owner/cause information. The basis snapshot preserves canonical `current`, `prior`, `unprotected`, `indeterminate`, and `not-yet-interpretable` values without promotion or inference. A successful scan-independent admission SHALL claim only the stable assignment set, exact authority, canonical owner evidence, and physical no-late/stabilization/session/epoch facts it carries; predecessor coverage supports only that current admission binding and never transfers authority.

If publication is unsupported or fails after durable session and epoch admission, start SHALL preserve those durable facts and the frontend owner's next action; later authority loss or publication failure blocks the post-commit publication gate without erasing or rolling back the durable session/epoch. It SHALL not rollback, auto-close, auto-rebegin, or report an online endpoint. The file-backed/store ownership owner and writable-recovery owner retain their distinct canonical contracts: a separately identified known partial multi-store acquisition consumes the file-backed owner’s current definite release-all result for every acquired partial lease, while this full-set/later publication path assumes no release-all result and preserves file-backed ownership/outcome pending the separately reconciled file-backed-owner semantic prerequisite; the writable-recovery owner separately determines its authority release, retention, or reconciliation. Failed or unknown release remains unresolved under that prerequisite, without retry or reacquisition. Existing semantic outcomes remain distinct: success, semantic refusal, blocked/unavailable, unsupported capability, reconciliation-required uncertainty, and operational failure.

#### Scenario: Admission succeeds and frontend publishes

- **WHEN** every current read-write prerequisite, including complete scan-independent admission and its durable new protection epoch observed as committed, succeeds, the exact current claim token and stable-resource/alias binding for every required store and writable-recovery authority are freshly revalidated from that observed commit through admission return and at frontend handoff and remain held through successful publication, exact checksum and basis source evidence are accepted, separate coded-range-authority and recovery `CLEAN` observations are accepted without combination, the selected frontend composes the required publication/currentization and basis action gates, and it publishes its owned endpoint successfully
- **THEN** start reports the exact published endpoint, preserves bounded lineage, custody, authority, per-store no-late/stabilization, exact checksum-owner evidence, the recovery-produced range-role basis snapshot, separate coded-range-authority and recovery `CLEAN` observations, session/epoch evidence, basis, checksum, owner/cause, and non-claim fields, and remains attached until the frontend shuts down or fails

#### Scenario: Scan-independent admission succeeds without payload enumeration

- **WHEN** a complete stable assignment set after a custody gap has current prerequisite writer and writable-recovery authority, every required store's exact current claim token and stable-resource/alias binding and the writable-recovery authority remain held and are freshly revalidated after stabilization and each owner gate through durable session begin, the epoch commit and observation of its result, admission return, and frontend handoff, the store-operation owner supplies successful exactly current-bound stabilization evidence for every store binding its selected capability/profile and declared physical reachability universe and covering every in-profile predecessor route/effect still capable of reaching the stable backing resource, exact checksum-owner evidence and the recovery-produced range-role basis snapshot are accepted under their exact owner bindings, separate coded-range-authority and recovery `CLEAN` owner observations are accepted without combination, the durable session and epoch observations are successful, and the selected frontend's independent gates pass
- **THEN** start may invoke a supported read/write frontend only through its separate publication/currentization and basis action gates while reporting that operation outcome, durability, integrity, custody continuity, range basis, current protection, and release remain independently owned

#### Scenario: Admission is refused before publication

- **WHEN** identity/topology is ambiguous, a required store or authority claim is unavailable or only partially known, an exact current claim token or writable-recovery authority is lost or mismatched, a stable resource is replaced/remapped or gains an alias, any required store lacks successful exact stabilization or complete predecessor closure within its selected profile's declared physical reachability universe, stabilization is unsupported/failed/uncertain/incomplete/stale/mismatched/non-monotonic, a new incarnation leaves in-profile predecessor effects unclosed, checksum-owner evidence is absent or mismatched in exact target/range, topology epoch, digest profile, checksum-set generation, target content generation, or persistence evidence, recovery basis source evidence or bindings are absent/stale/conflicting/insufficient/mismatched, coded-range-authority observation is unresolved, recovery `CLEAN` capture/state is unresolved, session or epoch commit is rejected/uncertain, or a later publication prerequisite is unavailable
- **THEN** the shared result reports semantic refusal, blockage, or reconciliation-required uncertainty with the owning causal dimension and owner/cause fact and publishes no endpoint; failed or unknown file-backed release remains blocked pending the separately reconciled file-backed-owner prerequisite


#### Scenario: Writer claim exists without stabilization

- **WHEN** startup holds the required current claim tokens and stable-resource/alias bindings, including for a new incarnation or ordering domain, but the store-operation owner has not established successful exact stabilization covering every route/effect within the selected profile's declared physical reachability universe that is still capable of reaching the same stable backing resource for one required store
- **THEN** start refuses writable admission because current exclusion and identity alone cannot prove that predecessor or unattributed physical effects have completed or that the binding remains valid

#### Scenario: Recovery history cannot substitute for stabilization

- **WHEN** recovery or operation history records prior identity, intent, disposition, or uncertainty but a required store lacks successful exactly current-bound stabilization evidence with exact predecessor physical reachability/closure coverage within the selected profile's declared universe
- **THEN** start refuses writable admission without scanning payload or requiring a persistent per-operation history ledger, and the causal blocker remains the missing physical store evidence

#### Scenario: A predecessor domain remains physically reachable

- **WHEN** the current claim, incarnation, and ordering domain bind successfully but the adapter cannot enumerate and close, or otherwise certify closure of, one predecessor claim, incarnation, or ordering domain whose effects may still reach the same stable backing resource
- **THEN** start reports `unsupported`, `failed`, or `uncertain` as applicable, publishes no endpoint, and predecessor coverage supports only the current admission binding without transferring authority

#### Scenario: An unknown physical route remains reachable

- **WHEN** the current claim token, stable resource, and selected capability/profile bind successfully but the adapter cannot identify and close, or otherwise certify closure of, an old kernel/device queue, DMA, raw or out-of-band path, alias/remap, unattributed effect, or any other route within that profile's declared physical reachability universe that may still reach the resource
- **THEN** start reports `unsupported` or `uncertain` as applicable, publishes no endpoint, and no current authority is inferred from predecessor coverage

#### Scenario: Session begin is durable but epoch admission is rejected

- **WHEN** the lifecycle owner durably establishes session begin but recovery rejects epoch admission as a known precondition failure
- **THEN** start preserves separate session-begin and prior-epoch evidence, publishes no endpoint, admits no new writable epoch, and leaves exact close, abort, and lifecycle/recovery decisions to their owners. Any file-backed store-authority release at this full-set or later gate remains an unresolved external file-backed-owner semantic prerequisite; start preserves its ownership/outcome and performs no inferred release, retry, or reacquisition, while writable-recovery authority follows its separate owner without inferred release or automatic retry

#### Scenario: Session or epoch commit is unresolved

- **WHEN** an unknown session or epoch outcome may have become durable and its observation is lost, corrupt, or unclassifiable
- **THEN** start preserves separate prior/proposed evidence, reports existing reconciliation-required uncertainty with owner/cause and next action, publishes no writable endpoint, and does not retry automatically, rollback session begin, or create a second begin

#### Scenario: Stabilization is interrupted or claim ownership is lost

- **WHEN** process loss, store disappearance, timeout, claim loss, writable-recovery-authority loss, resource replacement or remap, alias-set change, or an unclassifiable adapter result interrupts stabilization or later premise revalidation
- **THEN** start preserves `uncertain` evidence, publishes no endpoint, and any later attempt must reacquire and continuously hold/freshly revalidate the exact current claim token and resource/alias binding and writable-recovery authority and obtain fresh evidence. This later-premise scenario does not establish a file-backed release-all result; startup preserves the file-backed owner’s ownership/outcome and blocks pending the separately reconciled file-backed-owner semantic prerequisite. The writable-recovery owner separately determines its disposition without automatic upgrade, reacquisition, or retry


#### Scenario: Portable admission succeeds but publication fails

- **WHEN** portable admission succeeds and the frontend cannot publish, reports unsupported capability, or reports reconciliation-required state
- **THEN** start returns the frontend outcome, preserves durable session/epoch admission and the frontend next action, and the file-backed owner’s and writable-recovery owner’s separate contracts remain in force. A failed or unknown full-set/later file-backed release remains an unresolved external file-backed-owner semantic prerequisite; start preserves ownership/outcome and does not report an online or read-write endpoint, infer release, rollback, auto-close, auto-reacquire, or auto-rebegin

#### Scenario: Binding changes after durable admission

- **WHEN** the exact claim token or writable-recovery authority is lost, released, reacquired, or mismatched, or the stable resource is replaced, remapped, or gains a different alias after epoch admission and before or during frontend handoff or publication
- **THEN** start preserves durable session/epoch facts, invalidates the stale stabilization premise, publishes no endpoint, and requires fresh stabilization under a newly exact binding and writable-recovery authority or owner reconciliation; failed/unknown file-backed release remains blocked pending its separately reconciled prerequisite, and startup does not infer release or allow a second begin

#### Scenario: Read-only publication is requested without support

- **WHEN** the operator requests read-only start and no supported frontend path can enforce the required read-only service semantics
- **THEN** start reports unsupported without publishing an endpoint or converting read-write blockage into read-only success

#### Scenario: Valid topology exceeds frontend profile

- **WHEN** current topology and portable startup admission are valid but wider than the selected frontend's supported publication profile
- **THEN** start reports a frontend capability limitation and preserves topology and admission classifications as valid
