/**
 * Candidate finite model for exact fence-retirement semantics.
 *
 * The delegated surface is deliberately narrow: given an owner-qualified
 * root registry and an already admissible successor plan, this relation owns
 * exact occurrence identity, root-specific rebind/discharge, atomic
 * predecessor/successor outcomes, and conservative unknown reconciliation.
 * Owner creation/liveness, serialized ordering, export-size calculation,
 * legacy migration, physical durability, and production recovery remain
 * OpenSpec-owned inputs outside this relation.
 *
 * All identity, shape, and epoch domains are finite parameters. Exhaustive
 * checks therefore cover a declared finite instance, never product cardinality.
 */

module PersistenceEvidenceRetirement {
  type CapturePhase = CaptureOpen | CaptureCommitPending | CaptureCommitUnknown | CaptureCleanKnown | CaptureRetired

  type RootKind =
    CleanRegion
    | ValidIntegrity
    | WritableSession
    | CodedCapture
    | UnresolvedRecoveryCommit
    | LegacyUnreconciled

  type OwnerFact =
    Clean({ region: int, cleanGeneration: int })
    | Integrity({ extent: int, profile: int, setGeneration: int, contentGeneration: int, digest: int })
    | Session({ session: int, closeGeneration: int })
    | Capture({ capture: int, phase: CapturePhase })
    | Recovery({ intent: int, priorToken: int, proposedToken: int })
    | Legacy({ schema: int, position: int, candidates: Set[int] })

  type StoreFact = {
    store: int,
    incarnation: int,
    capability: int,
    watermark: int,
  }

  type FenceShape = {
    topology: int,
    domain: int,
    stores: Set[StoreFact],
  }

  // Immutable certificate facts are separate from mutable owner facts.
  type CertificateBinding = {
    shape: FenceShape,
    target: int,
    coverage: Set[int],
    generation: int,
    watermark: int,
  }

  type ClaimBinding = {
    kind: RootKind,
    fact: OwnerFact,
    certificate: CertificateBinding,
  }

  type Fence = {
    occurrence: int,
    shape: FenceShape,
    claims: Set[CertificateBinding],
  }

  type Root = {
    id: int,
    occurrence: int,
    binding: ClaimBinding,
  }

  type Snapshot = {
    generation: int,
    topology: int,
    allocatedOccurrences: Set[int],
    allocatedRoots: Set[int],
    fences: Set[Fence],
    roots: Set[Root],
  }

  type RetirementIntent = {
    intent: int,
    priorToken: int,
    proposedToken: int,
  }

  type UnresolvedState =
    NoUnresolved
    | Unresolved({ rootId: int, binding: ClaimBinding })

  type PendingState = {
    successor: Snapshot,
    retire: Set[int],
    expectedGeneration: int,
    expectedTopology: int,
    intent: RetirementIntent,
    unresolved: UnresolvedState,
  }

  type Phase = Stable | Pending | ReconciliationRequired
  type Outcome = NoOutcome | FenceCommitted | RootCommitted | Committed | Rejected | Unknown | Stale

  type Admission =
    CleanAdmission
    | IntegrityAdmission
    | SessionAdmission
    | CaptureAdmission
    | RecoveryAdmission
    | LegacyAdmission

  type RebindProof =
    CleanRebindProof
    | IntegrityRebindProof
    | CaptureRebindProof
    | RecoveryRebindProof
    | LegacyRebindProof

  type DischargeProof =
    CleanDischargeProof
    | IntegrityDischargeProof
    | CaptureDischargeProof
    | RecoveryDischargeProof
    | LegacyDischargeProof

  type RebindRequest = {
    rootId: int,
    successorOccurrence: int,
    binding: ClaimBinding,
    proof: RebindProof,
  }

  type DischargeRequest = {
    rootId: int,
    proof: DischargeProof,
  }

  type RetentionState = {
    snapshot: Snapshot,
    phase: Phase,
    pending: PendingState,
    lastOutcome: Outcome,
  }

  var state: RetentionState

  const Shapes: Set[FenceShape]
  const Occurrences: Set[int]
  const RootIds: Set[int]
  const Epochs: Set[int]
  const MaxFences: int
  const MaxRoots: int

  pure def factKind(fact: OwnerFact): RootKind = match fact {
    | Clean(_) => CleanRegion
    | Integrity(_) => ValidIntegrity
    | Session(_) => WritableSession
    | Capture(_) => CodedCapture
    | Recovery(_) => UnresolvedRecoveryCommit
    | Legacy(_) => LegacyUnreconciled
  }

  pure def certificateIsWellFormed(certificate: CertificateBinding): bool = all {
    Shapes.contains(certificate.shape),
    Epochs.contains(certificate.generation),
    Epochs.contains(certificate.watermark),
    certificate.shape.stores.map(store => store.store).size() == certificate.shape.stores.size(),
  }

  pure def bindingIsWellFormed(binding: ClaimBinding): bool =
    binding.kind == factKind(binding.fact)
      and certificateIsWellFormed(binding.certificate)

  pure def admissionMatches(kind: RootKind, admission: Admission): bool = match admission {
    | CleanAdmission => kind == CleanRegion
    | IntegrityAdmission => kind == ValidIntegrity
    | SessionAdmission => kind == WritableSession
    | CaptureAdmission => kind == CodedCapture
    | RecoveryAdmission => kind == UnresolvedRecoveryCommit
    | LegacyAdmission => kind == LegacyUnreconciled
  }

  pure def rebindProofMatches(kind: RootKind, proof: RebindProof): bool = match proof {
    | CleanRebindProof => kind == CleanRegion
    | IntegrityRebindProof => kind == ValidIntegrity
    | CaptureRebindProof => kind == CodedCapture
    | RecoveryRebindProof => kind == UnresolvedRecoveryCommit
    | LegacyRebindProof => kind == LegacyUnreconciled
  }

  pure def dischargeProofMatches(kind: RootKind, proof: DischargeProof): bool = match proof {
    | CleanDischargeProof => kind == CleanRegion
    | IntegrityDischargeProof => kind == ValidIntegrity
    | CaptureDischargeProof => kind == CodedCapture
    | RecoveryDischargeProof => kind == UnresolvedRecoveryCommit
    | LegacyDischargeProof => kind == LegacyUnreconciled
  }

  pure def rootMatchesFence(root: Root, fence: Fence): bool =
    root.occurrence == fence.occurrence
      and root.binding.certificate.shape == fence.shape
      and fence.claims.contains(root.binding.certificate)

  pure def rootsBindToFences(roots: Set[Root], fences: Set[Fence]): bool =
    roots.forall(root => fences.exists(fence => rootMatchesFence(root, fence)))

  pure def rootsReach(roots: Set[Root], occurrence: int): bool =
    roots.exists(root => root.occurrence == occurrence)

  pure def removeFence(fences: Set[Fence], occurrence: int): Set[Fence] =
    fences.filter(fence => fence.occurrence != occurrence)

  pure def freshOccurrence(allocated: Set[int], occurrence: int): bool =
    Occurrences.contains(occurrence)
      and not(allocated.contains(occurrence))
      and allocated.forall(previous => occurrence > previous)

  pure def freshRootId(allocated: Set[int], rootId: int): bool =
    RootIds.contains(rootId)
      and not(allocated.contains(rootId))
      and allocated.forall(previous => rootId > previous)

  pure def emptyPending(snapshot: Snapshot): PendingState = {
    successor: snapshot,
    retire: Set(),
    expectedGeneration: snapshot.generation,
    expectedTopology: snapshot.topology,
    intent: { intent: 0, priorToken: 0, proposedToken: 0 },
    unresolved: NoUnresolved,
  }

  pure def clearPending(s: RetentionState, outcome: Outcome): RetentionState = {
    ...s,
    phase: Stable,
    pending: emptyPending(s.snapshot),
    lastOutcome: outcome,
  }

  pure def initialSnapshot: Snapshot = {
    generation: 0,
    topology: 1,
    allocatedOccurrences: Set(),
    allocatedRoots: Set(),
    fences: Set(),
    roots: Set(),
  }

  pure def initialState: RetentionState = {
    snapshot: initialSnapshot,
    phase: Stable,
    pending: emptyPending(initialSnapshot),
    lastOutcome: NoOutcome,
  }

  pure def intentMatchesSnapshot(snapshot: Snapshot, intent: RetirementIntent): bool = all {
    intent.intent > 0,
    intent.priorToken == snapshot.generation,
    intent.proposedToken == snapshot.generation + 1,
    Epochs.contains(intent.proposedToken),
  }

  pure def shapeCovers(old: FenceShape, new: FenceShape): bool = all {
    old.topology == new.topology,
    old.domain == new.domain,
    old.stores.map(store => store.store).size() == old.stores.size(),
    new.stores.map(store => store.store).size() == new.stores.size(),
    old.stores.map(store => store.store) == new.stores.map(store => store.store),
    old.stores.forall(oldStore => new.stores.exists(newStore =>
      newStore.store == oldStore.store
        and newStore.incarnation == oldStore.incarnation
        and newStore.capability == oldStore.capability
        and newStore.watermark >= oldStore.watermark
    )),
  }

  pure def claimCovers(old: CertificateBinding, new: CertificateBinding): bool = all {
    shapeCovers(old.shape, new.shape),
    old.target == new.target,
    old.coverage == new.coverage,
    new.generation > old.generation,
    new.watermark >= old.watermark,
  }

  pure def rebindCompatible(old: Root, new: Root): bool =
    bindingIsWellFormed(new.binding)
      and claimCovers(old.binding.certificate, new.binding.certificate)
      and (
        new.binding.kind == old.binding.kind
          or (
            old.binding.kind == LegacyUnreconciled
              and new.binding.kind != WritableSession
              and new.binding.kind != LegacyUnreconciled
          )
      )

  pure def rootIds(roots: Set[Root]): Set[int] = roots.map(root => root.id)

  pure def validPlan(
    snapshot: Snapshot,
    predecessor: int,
    proposedRoots: Set[Root],
    rebinds: Set[RebindRequest],
    discharges: Set[DischargeRequest],
  ): bool = {
    val oldRoots = snapshot.roots.filter(root => root.occurrence == predecessor)
    val oldIds = rootIds(oldRoots)
    val rebindIds = rebinds.map(request => request.rootId)
    val dischargeIds = discharges.map(request => request.rootId)
    all {
      snapshot.fences.exists(fence => fence.occurrence == predecessor),
      oldRoots != Set(),
      rebindIds.size() == rebinds.size(),
      dischargeIds.size() == discharges.size(),
      rebindIds.intersect(dischargeIds) == Set(),
      oldIds == rebindIds.union(dischargeIds),
      rootIds(proposedRoots).size() == proposedRoots.size(),
      proposedRoots.forall(root => snapshot.allocatedRoots.contains(root.id)),
      proposedRoots.filter(root => not(oldIds.contains(root.id))) ==
        snapshot.roots.filter(root => not(oldIds.contains(root.id))),
      rebinds.forall(request =>
        request.successorOccurrence != predecessor
          and oldRoots.exists(old =>
            old.id == request.rootId
              and rebindProofMatches(old.binding.kind, request.proof)
              and snapshot.fences.exists(fence =>
                fence.occurrence == request.successorOccurrence
                  and fence.claims.contains(request.binding.certificate)
              )
              and proposedRoots.exists(proposed =>
                proposed.id == request.rootId
                  and proposed.occurrence == request.successorOccurrence
                  and proposed.binding == request.binding
                  and rebindCompatible(old, proposed)
              )
          )
      ),
      discharges.forall(request =>
        oldRoots.exists(old =>
          old.id == request.rootId
            and dischargeProofMatches(old.binding.kind, request.proof)
            and not(proposedRoots.exists(proposed => proposed.id == request.rootId))
        )
      ),
      rootsBindToFences(
        proposedRoots,
        snapshot.fences.filter(fence => fence.occurrence != predecessor),
      ),
    }
  }

  pure def recoveryBindingMatches(binding: ClaimBinding, intent: RetirementIntent): bool =
    binding.kind == UnresolvedRecoveryCommit
      and binding.fact == Recovery({
        intent: intent.intent,
        priorToken: intent.priorToken,
        proposedToken: intent.proposedToken,
      })
 
  pure def unresolvedHasExactPredecessor(
    unresolved: UnresolvedState,
    snapshot: Snapshot,
    pending: PendingState,
  ): bool = match unresolved {
    | NoUnresolved => false
    | Unresolved(candidate) =>
      recoveryBindingMatches(candidate.binding, pending.intent)
        and pending.retire.exists(occurrence =>
          snapshot.fences.exists(fence =>
            fence.occurrence == occurrence
              and fence.claims.contains(candidate.binding.certificate)
          )
        )
  }
  pure def intentFor(snapshot: Snapshot): RetirementIntent = {
    intent: 1,
    priorToken: snapshot.generation,
    proposedToken: snapshot.generation + 1,
  }

  action init: bool = all {
    state' = initialState,
  }

  action submitFence(
    occurrence: int,
    shape: FenceShape,
    claims: Set[CertificateBinding],
  ): bool = all {
    state.phase == Stable,
    Shapes.contains(shape),
    freshOccurrence(state.snapshot.allocatedOccurrences, occurrence),
    state.snapshot.fences.size() < MaxFences,
    Epochs.contains(state.snapshot.generation + 1),
    claims.forall(certificate =>
      certificateIsWellFormed(certificate)
        and certificate.shape == shape
    ),
    state' = {
      ...state,
      snapshot: {
        ...state.snapshot,
        generation: state.snapshot.generation + 1,
        allocatedOccurrences: state.snapshot.allocatedOccurrences.union(Set(occurrence)),
        fences: state.snapshot.fences.union(Set({
          occurrence: occurrence,
          shape: shape,
          claims: claims,
        })),
      },
      lastOutcome: FenceCommitted,
    },
  }

  action createAnyFence: bool = {
    nondet shape = Shapes.oneOf()
    nondet occurrence = Occurrences
      .filter(candidate => freshOccurrence(state.snapshot.allocatedOccurrences, candidate))
      .oneOf()
    all {
      state.phase == Stable,
      state.snapshot.fences.size() < MaxFences,
      Epochs.contains(state.snapshot.generation + 1),
      state' = {
        ...state,
        snapshot: {
          ...state.snapshot,
          generation: state.snapshot.generation + 1,
          allocatedOccurrences: state.snapshot.allocatedOccurrences.union(Set(occurrence)),
          fences: state.snapshot.fences.union(Set({
            occurrence: occurrence,
            shape: shape,
            claims: Set(),
          })),
        },
        lastOutcome: FenceCommitted,
      },
    }
  }

  action admitRoot(
    rootId: int,
    occurrence: int,
    binding: ClaimBinding,
    admission: Admission,
  ): bool = all {
    state.phase == Stable,
    freshRootId(state.snapshot.allocatedRoots, rootId),
    bindingIsWellFormed(binding),
    admissionMatches(binding.kind, admission),
    state.snapshot.fences.exists(fence =>
      fence.occurrence == occurrence
        and rootMatchesFence({ id: rootId, occurrence: occurrence, binding: binding }, fence)
    ),
    state.snapshot.roots.size() < MaxRoots,
    Epochs.contains(state.snapshot.generation + 1),
    state' = {
      ...state,
      snapshot: {
        ...state.snapshot,
        generation: state.snapshot.generation + 1,
        allocatedRoots: state.snapshot.allocatedRoots.union(Set(rootId)),
        roots: state.snapshot.roots.union(Set({
          id: rootId,
          occurrence: occurrence,
          binding: binding,
        })),
      },
      lastOutcome: RootCommitted,
    },
  }

  action prepareUnboundRetirement(occurrence: int, intent: RetirementIntent): bool = all {
    state.phase == Stable,
    intentMatchesSnapshot(state.snapshot, intent),
    state.snapshot.fences.exists(fence => fence.occurrence == occurrence),
    not(rootsReach(state.snapshot.roots, occurrence)),
    state' = {
      ...state,
      phase: Pending,
      pending: {
        successor: {
          ...state.snapshot,
          fences: removeFence(state.snapshot.fences, occurrence),
        },
        retire: Set(occurrence),
        expectedGeneration: state.snapshot.generation,
        expectedTopology: state.snapshot.topology,
        intent: intent,
        unresolved: NoUnresolved,
      },
      lastOutcome: NoOutcome,
    },
  }

  action preparePlannedRetirement(
    predecessor: int,
    proposedRoots: Set[Root],
    rebinds: Set[RebindRequest],
    discharges: Set[DischargeRequest],
    intent: RetirementIntent,
  ): bool = all {
    state.phase == Stable,
    intentMatchesSnapshot(state.snapshot, intent),
    validPlan(state.snapshot, predecessor, proposedRoots, rebinds, discharges),
    state' = {
      ...state,
      phase: Pending,
      pending: {
        successor: {
          ...state.snapshot,
          fences: removeFence(state.snapshot.fences, predecessor),
          roots: proposedRoots,
        },
        retire: Set(predecessor),
        expectedGeneration: state.snapshot.generation,
        expectedTopology: state.snapshot.topology,
        intent: intent,
        unresolved: NoUnresolved,
      },
      lastOutcome: NoOutcome,
    },
  }

  action commitPrepared: bool = all {
    state.phase == Pending,
    intentMatchesSnapshot(state.snapshot, state.pending.intent),
    state.snapshot.generation == state.pending.expectedGeneration,
    state.snapshot.topology == state.pending.expectedTopology,
    rootsBindToFences(state.pending.successor.roots, state.pending.successor.fences),
    not(state.pending.retire.exists(occurrence =>
      rootsReach(state.pending.successor.roots, occurrence)
    )),
    Epochs.contains(state.snapshot.generation + 1),
    state' = {
      ...state,
      snapshot: {
        ...state.pending.successor,
        generation: state.snapshot.generation + 1,
      },
      phase: Stable,
      pending: emptyPending({
        ...state.pending.successor,
        generation: state.snapshot.generation + 1,
      }),
      lastOutcome: Committed,
    },
  }

  action rejectPrepared: bool = all {
    state.phase == Pending,
    state' = clearPending(state, Rejected),
  }

  action rejectStale: bool = all {
    state.phase == Pending,
    state.snapshot.generation != state.pending.expectedGeneration
      or state.snapshot.topology != state.pending.expectedTopology,
    state' = clearPending(state, Stale),
  }

  action observeRetirementUnknown(rootId: int, binding: ClaimBinding): bool = all {
    state.phase == Pending,
    intentMatchesSnapshot(state.snapshot, state.pending.intent),
    state.snapshot.topology == state.pending.expectedTopology,
    freshRootId(state.snapshot.allocatedRoots, rootId),
    bindingIsWellFormed(binding),
    recoveryBindingMatches(binding, state.pending.intent),
    state.pending.retire.exists(occurrence =>
      state.snapshot.fences.exists(fence =>
        fence.occurrence == occurrence
          and fence.claims.contains(binding.certificate)
      )
    ),
    state' = {
      ...state,
      phase: ReconciliationRequired,
      pending: {
        ...state.pending,
        unresolved: Unresolved({ rootId: rootId, binding: binding }),
      },
      lastOutcome: Unknown,
    },
  }

  action reconcilePrior: bool = all {
    state.phase == ReconciliationRequired,
    intentMatchesSnapshot(state.snapshot, state.pending.intent),
    state' = clearPending(state, Rejected),
  }

  action reconcileSuccessor: bool = all {
    state.phase == ReconciliationRequired,
    intentMatchesSnapshot(state.snapshot, state.pending.intent),
    state.snapshot.generation == state.pending.expectedGeneration,
    state.snapshot.topology == state.pending.expectedTopology,
    rootsBindToFences(state.pending.successor.roots, state.pending.successor.fences),
    not(state.pending.retire.exists(occurrence =>
      rootsReach(state.pending.successor.roots, occurrence)
    )),
    Epochs.contains(state.snapshot.generation + 1),
    state' = {
      ...state,
      snapshot: {
        ...state.pending.successor,
        generation: state.snapshot.generation + 1,
      },
      phase: Stable,
      pending: emptyPending({
        ...state.pending.successor,
        generation: state.snapshot.generation + 1,
      }),
      lastOutcome: Committed,
    },
  }

  action reconcileNeither: bool = all {
    state.phase == ReconciliationRequired,
    state' = state,
  }

  action advanceGeneration: bool = {
    nondet nextGeneration = Epochs
      .filter(candidate => candidate > state.snapshot.generation)
      .oneOf()
    all {
      state.phase == Pending,
      state' = {
        ...state,
        snapshot: {
          ...state.snapshot,
          generation: nextGeneration,
        },
      },
    }
  }

  action advanceTopology: bool = {
    nondet nextTopology = Epochs
      .filter(candidate => candidate > state.snapshot.topology)
      .oneOf()
    all {
      state.phase == Pending,
      state' = {
        ...state,
        snapshot: {
          ...state.snapshot,
          topology: nextTopology,
        },
      },
    }
  }

  action prepareAnyUnboundRetirement: bool = {
    nondet occurrence = state.snapshot.fences
      .filter(fence => not(rootsReach(state.snapshot.roots, fence.occurrence)))
      .map(fence => fence.occurrence)
      .oneOf()
    prepareUnboundRetirement(occurrence, intentFor(state.snapshot))
  }

  action step: bool = any {
    createAnyFence,
    prepareAnyUnboundRetirement,
    commitPrepared,
    rejectPrepared,
    rejectStale,
    reconcilePrior,
    reconcileSuccessor,
    reconcileNeither,
    advanceGeneration,
    advanceTopology,
  }

  val BoundedState: bool = all {
    state.snapshot.fences.size() <= MaxFences,
    state.snapshot.roots.size() <= MaxRoots,
    state.snapshot.allocatedOccurrences.subseteq(Occurrences),
    state.snapshot.allocatedRoots.subseteq(RootIds),
    state.pending.successor.fences.size() <= MaxFences,
    state.pending.successor.roots.size() <= MaxRoots,
  }

  val StableOccurrenceIdentity: bool = all {
    state.snapshot.fences.map(fence => fence.occurrence).size() == state.snapshot.fences.size(),
    state.snapshot.fences.forall(fence =>
      state.snapshot.allocatedOccurrences.contains(fence.occurrence)
    ),
    state.snapshot.roots.map(root => root.id).size() == state.snapshot.roots.size(),
    state.snapshot.roots.forall(root => state.snapshot.allocatedRoots.contains(root.id)),
  }

  val ExactRootBindings: bool = rootsBindToFences(state.snapshot.roots, state.snapshot.fences)

  val UnknownRetainsRetirementPredecessor: bool =
    state.phase == ReconciliationRequired implies
      state.pending.retire.subseteq(
        state.snapshot.fences.map(fence => fence.occurrence)
      )

  val UnknownRetainsExactIntentRoot: bool =
    state.phase == ReconciliationRequired implies
      state.pending.unresolved != NoUnresolved
        and state.pending.intent.priorToken == state.pending.expectedGeneration
        and state.pending.intent.proposedToken == state.pending.expectedGeneration + 1

  val UnknownHasExactPredecessor: bool =
    state.phase == ReconciliationRequired implies
      state.snapshot.generation == state.pending.expectedGeneration
        and state.snapshot.topology == state.pending.expectedTopology
        and unresolvedHasExactPredecessor(
          state.pending.unresolved,
          state.snapshot,
          state.pending,
        )

  val PendingPreservesUnrelatedRoots: bool = {
    val retiredRootIds = state.snapshot.roots
      .filter(root => state.pending.retire.contains(root.occurrence))
      .map(root => root.id)
    (state.phase == Pending or state.phase == ReconciliationRequired) implies
      state.snapshot.roots.filter(root => not(retiredRootIds.contains(root.id))) ==
        state.pending.successor.roots.filter(root => not(retiredRootIds.contains(root.id)))
  }

  val PlannedRetirementRemovesOnlyUnreachableRoots: bool =
    state.phase == Pending implies
      not(state.pending.retire.exists(occurrence =>
        rootsReach(state.pending.successor.roots, occurrence)
      ))

  val PendingSuccessorIsSelfContained: bool =
    state.phase != Pending and state.phase != ReconciliationRequired
      or rootsBindToFences(state.pending.successor.roots, state.pending.successor.fences)

  val SafetyInvariant: bool =
    BoundedState
      and StableOccurrenceIdentity
      and ExactRootBindings
      and UnknownRetainsRetirementPredecessor
      and UnknownRetainsExactIntentRoot
      and UnknownHasExactPredecessor
      and PendingPreservesUnrelatedRoots
      and PlannedRetirementRemovesOnlyUnreachableRoots
      and PendingSuccessorIsSelfContained
}
