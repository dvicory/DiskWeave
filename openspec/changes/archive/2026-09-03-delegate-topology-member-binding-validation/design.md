## Context

See `proposal.md` for motivation. Current product behavior is owned by `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments`; `req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe` consumes and repeats part of that rule while adding service orchestration and refusal timing.

The current implementation splits the checks across checked constructors, `TopologySnapshot::validate`, semantic lookup helpers, and service assembly:

| Current check | Production seam | Current ownership reading | Delegation decision |
| --- | --- | --- | --- |
| Profile has nonzero data/parity counts and a representable total | `CodingProfile::new`; unchecked `Deserialize` bypasses it | Input-construction obligation; current canonical exact ingress behavior is not fully represented by `TopologySnapshot::validate` | Do not delegate constructor or deserialization validity; project only owner-accepted profiles and use their declared role/position relation. |
| Protected/parity lengths are positive/large enough and lengths align to a nonzero logical block size | `ProtectedGeometry::{new,with_parity_length}`; unchecked `Deserialize` bypasses them | “invalid protected lengths” remains current, but the validator does not recheck the constructor invariant | Do not delegate well-formedness; project only owner-accepted geometry and retain the reachable deserialization mismatch as an explicit non-delegated conformance gap. |
| Assignment count equals the profile total | `TopologySnapshot::validate` | Complete profile-relative snapshot interpretation | Delegate; this is not the implementation-only maximum-assignment ceiling. |
| Every role accepts its coding position | `CodingProfile::accepts` through `TopologySnapshot::validate` | Selected-profile role/coding interpretation | Delegate. |
| Logical slots, coding positions, and assignment instances are unique | `TopologySnapshot::validate` | Unambiguous snapshot assignments | Delegate. |
| Snapshot has at most `MAX_TOPOLOGY_ASSIGNMENTS` | `TopologySnapshot::validate` | Concrete resource ceiling not stated as exact product topology meaning | Do not delegate; compose it as an implementation bound. |
| Candidate array equals the active array; candidate epoch is newer; profile is unchanged | `TopologyAuthority::prepare` | Topology-transition policy | Do not delegate. |
| Service supports one parity assignment and every store supports read/write/durable flush | `validate_assembly` | Healthy-service capability policy | Do not delegate. |
| Binding count equals assignment count and every assignment has exactly one binding | `validate_assembly` | Missing/extra/duplicate binding rejection | Delegate. |
| Binding slot resolves semantically and role, position, assignment instance, generation, and topology epoch equal the resolved snapshot assignment/snapshot | `validate_assembly` | Assignment-mismatch and stale-generation/epoch rejection | Delegate. |
| Binding-declared store identity and epoch equal the opened store's reported identity and epoch | `validate_assembly` | Exact stable binding to the supplied opened store | Delegate equality of supplied facts; do not decide whether discovery or an operator was authorized to choose that store. |
| Opened store length/block-size facts equal snapshot protected geometry | `validate_assembly` | Binding geometry compatibility | Delegate equality of supplied facts; capability evidence production remains external. |
| Binding slots and store identities are unique | `validate_assembly` | Duplicate/aliased binding rejection | Delegate. |
| Every pair of complete identity-observation sets compares as `Changed` | `IdentityObservationSet::compare` and `validate_assembly` | Alias/ambiguity rejection, with assessment meaning owned separately | Delegate rejection from an owner-supplied `Distinct`, `Same`, or `Ambiguous` comparison; do not reimplement assessment precedence or provenance. |
| Assignment/binding order does not change slot/position lookup | `assignment_for_slot`, `assignment_for_position`, and service slot lookup | Stable semantic lookup | Delegate permutation-invariant validity and lookup projection; do not prescribe storage order or search algorithm. |

Three apparent checks are deliberately not filled in from implementation inference. First, a standalone snapshot contains one array identity, so “mismatched array identities” has no second value to compare until a current caller supplies an expected/candidate pair. The only current direct array comparison is transition-owned `TopologyAuthority::prepare`; the model must not absorb it. Second, a `TopologyAssignment` does not contain an expected `StoreId`; the current assembly check proves that a binding's declared store identity equals its opened store's report and that store identities are unique, not that the selected store was authorized for the assignment. Third, serde deserialization can construct `CodingProfile` and `ProtectedGeometry` values without their checked constructors, while `TopologySnapshot::validate` does not revalidate those invariants and service assembly does not inspect parity length. The model therefore accepts profile/geometry only as owner-accepted trusted inputs and owns equality to their supplied values, not their construction or deserialization validity. These retained obligations and conformance gaps remain non-delegated unless a later canonical change identifies the authority-bearing inputs and deliberately changes the affected behavior.

## Goals / Non-Goals

**Goals:**

- Represent the accepted snapshot/member-binding input as one small parameterized pure relation with independently named predicates.
- Keep the model result equivalent to the current boolean admission outcome for every mapped production input.
- Make every delegated predicate traceable to current prose and a production check.
- Make positive validity, each rejection family, semantic lookup, and permutation invariance non-vacuous under finite analysis.
- Make a missing or inverted predicate detectable by a dedicated valid-baseline mutation canary.
- Keep canonical semantics, finite analysis configuration, production correspondence, and evidence records visibly separate.

**Non-Goals:**

- Model a protocol, state transition, scheduler, discovery process, identity resolver, operator workflow, service lifecycle, topology authority, payload, parity equation, recovery transaction, persistent format, or physical store.
- Define a new error taxonomy or require production to expose all failed predicates. Existing public failure classes/messages remain outside the semantic correspondence claim; only accept/reject and semantic lookup are compared.
- Move profile or geometry construction/deserialization validity, `MAX_TOPOLOGY_ASSIGNMENTS`, integer representation, allocation behavior, the single-parity service restriction, read/write/flush support, or other service capability policy into the canonical model.
- Treat an identity observation, path, store label, accepted profile/geometry value, or pairwise comparison as trustworthy merely because it appears in model input; the named external owner must accept it first.
- Claim exhaustive behavior for arbitrary identifier, cardinality, integer, geometry, deserialized-input, or Rust input spaces.

## Decisions

### 1. Use a parameterized pure module, not a state machine

`models/quint/TopologyMemberBindingValidation.qnt` will contain semantic record/sum types and pure definitions for snapshot validity, binding validity, combined validity, and stable lookup. It will have no protocol phase, action scheduler, transition history, or mutable product state. A one-shot state machine would add artificial behavior and invite transition policy into a validation question.

The public semantic surface is conceptually:

- `snapshotValid(snapshot)`;
- `bindingsValid(snapshot, bindings, pairwiseIdentityComparisons)`;
- `valid(snapshot, bindings, pairwiseIdentityComparisons) = snapshotValid(snapshot) and bindingsValid(...)`;
- `assignmentForSlot` and `bindingForSlot`, defined by semantic identity rather than collection position.

The canonical result is valid/invalid. Independently named predicate definitions support review and mutation without making first-error ordering or a new reason set normative.

Alternative rejected: model topology preparation/publication as actions. That would import array-transition, monotonic-epoch, profile-change, verification, durability, and publication policy owned by other requirements.

### 2. Keep trusted facts explicit and small

The model input records mirror semantic facts, not Rust layout:

- `Profile`: owner-accepted data-role and parity-role counts whose role/position interpretation is supplied;
- `Geometry`: owner-accepted protected length, parity length, and logical block size;
- `Snapshot`: opaque array identity, topology epoch, accepted profile, accepted geometry, and assignments;
- `Assignment`: slot, role, coding position, assignment instance, and generation;
- `Binding`: captured assignment fields, declared store identity, opened-store-reported identity and epoch, and opened-store-reported protected length/block size;
- `comparison(leftStore, rightStore)`: the identity owner's result `Distinct`, `Same`, or `Ambiguous` for each unordered pair of bound stores.

Opaque identities are compared only for equality. The model neither parses nor orders identity bytes. Raw observations may be retained by a correspondence harness to reconstruct the owner comparison, but the canonical relation invokes the supplied owner-qualified comparison projection for each bound pair. It does not validate a serialized comparison table, record multiplicity, or assessment provenance. This prevents the model from becoming a second identity assessor or an adapter-format validator.

The array identity remains part of the snapshot and stable projections but has no standalone validity comparison inside this boundary because no second array claim exists in the current snapshot/binding input. Profile and geometry values likewise enter only after external owner acceptance; unchecked deserialized values are outside the mapped relation until their current ingress gap is reconciled. Adding any of those comparisons is a semantic change requiring a new delta and review.

Alternative rejected: pass booleans such as `topologyOkay` or `bindingOkay`. That would make the model vacuous and hide the structural relation. The only preclassified input is the per-pair identity-owner comparison because its derivation is independently owned and explicitly excluded.

### 3. Define the conjunction once

The canonical module will compose these named predicates:

1. **Snapshot coverage:** assignment count equals the supplied accepted profile total; every assignment role accepts its position under that profile; slots, positions, and assignment instances are pairwise unique.
2. **Binding bijection:** binding count equals assignment count; every binding resolves to exactly one assignment by slot; every assignment has exactly one binding.
3. **Captured-field equality:** resolved assignment role, coding position, assignment instance, and generation equal the binding fields; binding epoch equals the snapshot epoch.
4. **Opened-store equality:** declared and reported store identities match; reported store epoch equals the binding/snapshot epoch; reported protected length and block size equal the supplied accepted snapshot geometry; declared store identities are pairwise unique.
5. **Owner-qualified distinctness:** the identity owner's comparison projection returns `Distinct` for every unordered pair of bound stores; `Same` or `Ambiguous` rejects.
6. **Order independence:** the validity result and stable slot/position projections depend only on semantic values, not list order.

Profile/geometry owners and checked constructors remain responsible for accepting those inputs before projection. The current unchecked-deserialization gap is outside correspondence and is not silently treated as validation. Service capability checks compose after the relation and therefore may reject a model-valid multi-parity or capability-poor input without contradicting the model.

Alternative rejected: duplicate the conjunction in Rust tests, Connect adapters, and deterministic model scenarios. Each consumer instead maps to the canonical named definitions and compares observable outcomes.

### 4. Separate invariant, witness, scenario, and mutation evidence

The finite analysis must check:

- `ValidIffAllDelegatedPredicates`: combined validity is exactly the conjunction above;
- `AcceptedSnapshotIsUnambiguous`: accepted assignments have unique slots, positions, and instances and complete coverage relative to the supplied accepted profile;
- `AcceptedBindingsAreBijective`: every accepted assignment and binding map one-to-one;
- `AcceptedBindingsMatchCapturedFacts`: accepted bindings match assignment, epoch, opened-store, and supplied-geometry facts;
- `AcceptedBindingsAreDistinct`: accepted bound store identities are unique and the owner comparison returns `Distinct` for every bound pair;
- `PermutationInvariant`: permuting assignments and bindings preserves validity and semantic lookup when the same comparison projection is supplied;
- `LookupIsUnique`: accepted slot/position lookup returns the same unique semantic object in every permutation.

Non-vacuity witnesses/scenarios must exhibit:

- one valid one-data/one-parity input;
- one valid two-data/one-parity input;
- one structurally valid multi-parity input that the model accepts while healthy-service capability policy may reject;
- reordered valid assignments and bindings with identical stable lookup;
- one reachable rejection for each named predicate family;
- both `Same` and `Ambiguous` owner comparisons being rejected;
- matching and mismatched opened-store geometry facts against one supplied owner-accepted geometry.

Deterministic `run` scenarios establish concrete examples. Exhaustive bounded checks establish the invariants only for their declared finite domains. Connect, if used, establishes mapped production correspondence only.

### 5. Use two finite profiles and state exact non-claims

The focused exhaustive profile will use at least three slot identities, three assignment identities, three store identities, accepted data/parity counts sufficient for one-data/one-parity and two-data/one-parity shapes, assignment/binding lengths that include missing and extra cases, epochs and generations with at least two values, two owner-accepted geometry values, opened-store geometry matches/mismatches, and all three owner comparison outcomes. It must enumerate every delegated predicate truth value and produce at least one valid and one invalid input. Comparison-function generation belongs to the analysis harness; malformed encoded comparison tables are not product inputs or model defects.

A second bounded profile will widen either identifier/cardinality or supplied-geometry values without weakening the focused profile. If exhaustive exploration of the widened cross-product is impractical, it becomes deterministic/sampled evidence and records that limitation rather than silently reducing the claim.

Explicit non-claims are: profile/geometry construction or deserialization validity; arbitrary cardinality or integer proof; host integer overflow behavior; the `MAX_TOPOLOGY_ASSIGNMENTS` ceiling; identity observation correctness or precedence; comparison-table encoding; discovery completeness; authorization of a chosen store; array transition or epoch monotonicity; supported service profile/capabilities; request admission; topology publication; payload/parity correctness; recovery/persistence/durability; Rust memory safety; and physical I/O.

### 6. Require predicate-specific mutation canaries

`verification/quint/TopologyMemberBindingValidationMutants.qnt` will contain disposable altered predicates or inputs; it will not be imported by the canonical module. Every input-defect canary must:

1. construct or import a baseline for which the unmodified canonical combined relation is valid;
2. assert that baseline validity before injection;
3. inject exactly the named defect needed to challenge one guard, preserving unrelated source values where mathematically possible;
4. assert that the named canonical predicate is false after injection;
5. demonstrate that the corresponding deliberately weakened/inverted mutant is detected by the invariant or deterministic scenario.

Required input-defect canary families are: assignment undercount and overcount; role/position mismatch under the supplied profile; duplicate slot; duplicate coding position; duplicate assignment instance; missing binding; extra binding; duplicate binding slot; assignment role mismatch; position mismatch; assignment-instance mismatch; generation mismatch; binding/snapshot epoch mismatch; declared/reported store-ID mismatch; duplicate store identity; opened-store epoch mismatch; opened-store length mismatch; opened-store block-size mismatch; accepted `Same`; and accepted `Ambiguous`.

The positional/order-dependent lookup canary is relational rather than an invalid-input injection: it starts from two valid permutations, asserts canonical validity and equal semantic lookup for both, and then demonstrates divergence only under the positional mutant. Some input defects imply a downstream bijection failure as well as their named local predicate. Each such canary must prove the intended local predicate changed and must not claim logical isolation when predicates necessarily compose. A canary that begins invalid, changes no named predicate, fails only because of an unrelated malformed fixture, or tests a malformed comparison-table encoding is invalid evidence.

### 7. Map correspondence to observable production facts

| Model surface | Production projection | Required comparison | Non-claim |
| --- | --- | --- | --- |
| Profile/geometry ingress | Existing checked construction paths and explicit owner acceptance before projection | harness asserts the production values entered through the accepted domain; no model predicate compares constructor validity | unchecked deserialization and arbitrary host-width validity |
| Snapshot coverage/uniqueness | `CodingProfile::{total_slots,accepts}` and `TopologySnapshot::validate` | same valid/invalid outcome for every bounded snapshot built with an owner-accepted profile/geometry | exact first `TopologyValidationError` under multiple defects |
| Stable assignment lookup | `assignment_for_slot`, `assignment_for_position` | same unique assignment after each tested permutation | storage/search algorithm |
| Binding bijection/captured fields | `validate_assembly` through the smallest accessible service/test seam | same assembly valid/invalid outcome for bounded inputs whose non-delegated service prerequisites are satisfied | service lifecycle or request admission |
| Opened-store equality/supplied geometry | binding fields and `RandomAccessStore` identity, epoch, and capabilities observed by `validate_assembly` | same valid/invalid outcome with real or existing focused test stores | validity or physical truth of supplied geometry/capability evidence |
| Pairwise distinctness | `IdentityObservationSet::compare` invoked directly for each pair by `validate_assembly` | bridge projects the production comparison function and compares rejection when any pair is not `Changed`/`Distinct` | correctness of identity assessment/provenance or encoded pairwise tables |
| Combined result | `TopologySnapshot::validate` plus `validate_assembly` and unchanged external checks | all production accepts in the owner-accepted mapped domain are model-valid; mapped model-invalid inputs are rejected before admission/mutation | unchecked deserialized input, formal Rust correctness, or assignment-to-store authorization |

Direct deterministic Rust tests are preferred for pure validation tables. They must construct mapped profiles/geometries through accepted production paths and separately retain the unchecked-deserialization gap as a non-claim. Add `quint-connect` only if it can drive the real production assembly seam and compare observable results without manufacturing identity assessment, store facts, or admission. If no practical Connect seam exists, record that mapping limitation and use direct table-driven correspondence; do not add a generic production API solely for the bridge.

Reverse-direction evidence must sample or exhaust every production input in the declared owner-accepted bounded projection and confirm model acceptance. Forward negative evidence must inject each mapped defect through a real validation/service input and observe rejection. Calling a validator is not enough; the harness records the returned accept/reject outcome and, for assembly, confirms no service is returned and no member mutation occurred where the existing seam exposes that fact.

### 8. Use the existing importer and currentize evidence only after review

The repository knowledge importer already recognizes `models/quint/*.qnt`, `verification/quint/*.qnt`, and `// dwv:req ...` markers. Do not add importer code or a new registry. During active implementation the model remains proposed/non-authoritative; source-local current delegation markers are added only at the canonical authority transition so an unreviewed model cannot appear as current delegated semantics.

After both review gates, evidence currentization will:

- add the canonical source marker for `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments` at the authority transition;
- register the bounded executable-model and, if implemented, Connect evidence in `verification/manifest.toml` with exact commands, bounds, observations, tier, fault model, and non-claims;
- update the maintained portable verification evidence with the mapping table, bounded results, mutation results, and closest unsupported claim boundary;
- replace the requirement's deferred evidence target in `docs/reviewed-requirements.toml` only after the artifact-owned evidence is complete and individually reviewed;
- run the normal knowledge affected/currentization workflow after canonical sync, without treating the model checks as implementation proof.

Evidence-only analysis, mutant, and Connect modules carry no canonical owner marker. If current importer behavior cannot express this distinction, that is a blocker to reconcile rather than permission to mark every module delegated.

### 9. Enforce two independent adversarial gates

Gate A occurs after the canonical model, finite profiles, deterministic scenarios, mutations, and proposed production map are complete but before any prose is removed. A reviewer must audit source ownership, all excluded policy, predicate completeness, non-vacuity, finite scopes, each canary's valid baseline/named defect, and correspondence feasibility. Every blocking finding is repaired and the full Gate A review repeats after a material boundary change until `GO`.

Only after Gate A `GO` may the active delta remove exact predicate narration made redundant by the accepted model. The cutover retains the stable owner ID, supplied-input trust boundary, result meaning, refusal timing, all existing scenarios, finite-scope warning, and every non-delegated decision. The healthy-service delta removes only duplicate validation predicates and retains orchestration, capability/profile policy, recovery/checksum barriers, admission, child-I/O/mutation timing, and request evidence.

Gate B is a fresh review of the exact post-cutover model, delta specs, design, tasks, production/evidence diff, and review record. It must return `GO` before evidence currentization, canonical sync, archive, or workspace adoption. Any later semantic, model-boundary, predicate, trust-input, or correspondence change invalidates Gate B and requires another exact-snapshot review. Mechanical evidence-result updates may proceed only if they do not alter those reviewed surfaces; otherwise review repeats.

## Risks / Trade-offs

- **[Risk]** The model accidentally absorbs transition array/epoch policy because `TopologyAuthority::prepare` is nearby. **Mitigation:** no candidate/active transition pair or monotonicity operation exists in the canonical module; the mapping table marks those checks non-delegated.
- **[Risk]** Alias rejection becomes a second identity assessor. **Mitigation:** consume only owner-qualified pairwise comparison outcomes and test that both `Same` and `Ambiguous` reject.
- **[Risk]** A finite proof is reported as general topology or Rust correctness. **Mitigation:** keep bounds in evidence modules/manifest and state arbitrary-width, implementation, and physical non-claims beside every result.
- **[Risk]** Unchecked deserialization bypasses profile/geometry constructors and could be mistaken for modeled input validity. **Mitigation:** exclude constructor/deserialization validity from the relation, assert owner acceptance at projection, retain the current gap explicitly, and require a separate semantic change before changing those outcomes.
- **[Risk]** A boolean relation hides a production first-error mismatch. **Mitigation:** first-error identity is an explicit non-claim; preserving admission outcomes requires only exact valid/invalid correspondence. Add a reason set later only if a canonical product consumer needs it.
- **[Risk]** A mutation passes because its fixture was already invalid. **Mitigation:** assert canonical baseline validity before every injection and named-predicate falsity afterward.
- **[Risk]** Clean cutover deletes timing or ownership context along with duplicated predicates. **Mitigation:** Gate B compares each removed clause/scenario with the accepted delegated surface and requires an owner for every retained decision.
- **[Risk]** A current `dwv:req` marker makes the model authoritative before the delta is canonical. **Mitigation:** add the marker only at the reviewed sync boundary and rerun exact review if that step changes more than authority metadata.

## Migration Plan

1. Implement the unmarked proposed canonical pure module and evidence-only bounded profiles, deterministic scenarios, and predicate-specific mutants. Do not edit current canonical prose.
2. Build direct production correspondence and add Connect only where it can observe the real validator without invented facts or a new product API.
3. Complete bounded verification and mutation evidence with exact finite-scope non-claims.
4. Obtain Gate A adversarial model `GO`; repair and repeat after any material boundary change.
5. Apply the reviewed model boundary to the active delta only: remove exact duplicated predicate prose while retaining context, scenarios, timing, exclusions, and independently owned policy.
6. Obtain Gate B fresh exact-snapshot adversarial `GO`; repair and repeat as required.
7. Currentize evidence records without changing the reviewed relation, but do not add a current delegated-source marker or importer classification yet. If evidence currentization changes a reviewed semantic surface, repeat Gate B.
8. Synchronize the accepted delta into current specs and add the canonical source marker in the same authority transition; then verify importer classification, run required affected-owner and focused verification gates, and archive only when all tasks and evidence are complete.
9. Adopt the completed revision into the intended workspace only after sync/archive evidence is durable and review remains current.

Rollback before synchronization deletes the proposed model/evidence implementation and restores the active delta without changing current semantics. After synchronization, rollback requires a new OpenSpec change that removes the explicit delegation and restores complete exact prose; deleting or ignoring the canonical model alone is not a valid rollback.
