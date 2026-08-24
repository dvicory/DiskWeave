// -*- mode: Bluespec; -*-
/**
 * Bounded healthy-service lifecycle composition relation.
 *
 * The model composes seven external owner-approved observations for an exact
 * operation generation. Applicability is an external finite admission set;
 * cleanup is a bounded observation after semantic authorization. Different
 * generations may coexist in this abstraction so generation correlation is
 * checked without importing operation-slot reuse policy.
 */
// dwv:req req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts
module LifecycleRelease {
  const OperationGenerations: Set[OperationGeneration]
  const ApplicableGenerations: Set[OperationGeneration]

  type OperationGeneration = {
    operation: str,
    generation: int,
  }

  type MediaEffect = MediaUnresolved | MediaTerminal | MediaReconciled
  type ChildOutcome = ChildrenUnresolved | ChildrenTerminal
  type TransactionRequirement = TransactionUnresolved | TransactionSatisfied
  type RecoveryObservation = RecoveryUnresolved | RecoveryAuthoritative
  type BasisConformance = BasisUnresolved | BasisConsumed | BasisDiscarded | BasisReconciled
  type CleanupObservation = CleanupNotRequested | CleanupFailed | CleanupSucceeded

  type OwnerFacts = {
    mediaEffect: MediaEffect,
    children: ChildOutcome,
    requiredReconciliation: bool,
    reclaimable: bool,
    transaction: TransactionRequirement,
    recovery: RecoveryObservation,
    basis: BasisConformance,
  }

  type ReleaseState = {
    active: Set[OperationGeneration],
    facts: OperationGeneration -> OwnerFacts,
    authorizations: Set[OperationGeneration],
    cleanup: OperationGeneration -> CleanupObservation,
    cleanupRequests: Set[OperationGeneration],
  }

  var state: ReleaseState

  pure val emptyFacts: OwnerFacts = {
    mediaEffect: MediaUnresolved,
    children: ChildrenUnresolved,
    requiredReconciliation: false,
    reclaimable: false,
    transaction: TransactionUnresolved,
    recovery: RecoveryUnresolved,
    basis: BasisUnresolved,
  }

  pure def ownerFactsComplete(facts: OwnerFacts): bool =
    (facts.mediaEffect == MediaTerminal or facts.mediaEffect == MediaReconciled)
      and facts.children == ChildrenTerminal
      and facts.requiredReconciliation
      and facts.reclaimable
      and facts.transaction == TransactionSatisfied
      and facts.recovery == RecoveryAuthoritative
      and (facts.basis == BasisConsumed
        or facts.basis == BasisDiscarded
        or facts.basis == BasisReconciled)

  action init: bool = all {
    state' = {
      active: Set(),
      facts: OperationGenerations.mapBy(_ => emptyFacts),
      authorizations: Set(),
      cleanup: OperationGenerations.mapBy(_ => CleanupNotRequested),
      cleanupRequests: Set(),
    },
  }

  // Enter only generations whose external owner has already classified them
  // as in-scope for lifecycle release composition. This is not slot admission.
  action enterApplicable(id: OperationGeneration): bool = all {
    ApplicableGenerations.contains(id),
    not(state.active.contains(id)),
    not(state.authorizations.contains(id)),
    state' = {
      ...state,
      active: state.active.union(Set(id)),
      facts: state.facts.put(id, emptyFacts),
      cleanup: state.cleanup.put(id, CleanupNotRequested),
      cleanupRequests: state.cleanupRequests.exclude(Set(id)),
    },
  }

  action observeMediaTerminal(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), mediaEffect: MediaTerminal}),
    },
  }

  action observeMediaReconciled(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), mediaEffect: MediaReconciled}),
    },
  }

  action observeChildrenTerminal(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), children: ChildrenTerminal}),
    },
  }

  action recordRequiredReconciliation(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), requiredReconciliation: true}),
    },
  }

  action observeReclaimable(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), reclaimable: true}),
    },
  }

  action observeTransactionSatisfied(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), transaction: TransactionSatisfied}),
    },
  }

  action observeRecoveryAuthoritative(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), recovery: RecoveryAuthoritative}),
    },
  }

  action observeBasisConformance(id: OperationGeneration, observation: BasisConformance): bool = all {
    state.active.contains(id),
    observation != BasisUnresolved,
    state' = {
      ...state,
      facts: state.facts.put(id, {...state.facts.get(id), basis: observation}),
    },
  }

  action establishReleaseAllowed(id: OperationGeneration): bool = all {
    state.active.contains(id),
    ApplicableGenerations.contains(id),
    ownerFactsComplete(state.facts.get(id)),
    not(state.authorizations.contains(id)),
    state' = {
      ...state,
      authorizations: state.authorizations.union(Set(id)),
    },
  }

  action observeReleaseAllowed(id: OperationGeneration): bool = all {
    state.authorizations.contains(id),
    state' = state,
  }

  action requestCleanup(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state.authorizations.contains(id),
    state.cleanup.get(id) != CleanupSucceeded,
    not(state.cleanupRequests.contains(id)),
    state' = {
      ...state,
      cleanupRequests: state.cleanupRequests.union(Set(id)),
    },
  }

  action observeCleanupFailed(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state.authorizations.contains(id),
    state.cleanupRequests.contains(id),
    state.cleanup.get(id) != CleanupSucceeded,
    state' = {
      ...state,
      cleanup: state.cleanup.put(id, CleanupFailed),
      cleanupRequests: state.cleanupRequests.exclude(Set(id)),
    },
  }

  action observeCleanupSucceeded(id: OperationGeneration): bool = all {
    state.active.contains(id),
    state.authorizations.contains(id),
    state.cleanupRequests.contains(id),
    state.cleanup.get(id) != CleanupSucceeded,
    state' = {
      ...state,
      active: state.active.exclude(Set(id)),
      cleanup: state.cleanup.put(id, CleanupSucceeded),
      cleanupRequests: state.cleanupRequests.exclude(Set(id)),
    },
  }

  action step: bool = {
    nondet id = ApplicableGenerations.oneOf()
    any {
      enterApplicable(id),
      observeMediaTerminal(id),
      observeMediaReconciled(id),
      observeChildrenTerminal(id),
      recordRequiredReconciliation(id),
      observeReclaimable(id),
      observeTransactionSatisfied(id),
      observeRecoveryAuthoritative(id),
      observeBasisConformance(id, BasisConsumed),
      observeBasisConformance(id, BasisDiscarded),
      observeBasisConformance(id, BasisReconciled),
      establishReleaseAllowed(id),
      observeReleaseAllowed(id),
      requestCleanup(id),
      observeCleanupFailed(id),
      observeCleanupSucceeded(id),
    }
  }

  val ReleaseAllowed: Set[OperationGeneration] = state.authorizations

  val ReleaseCandidates: Set[OperationGeneration] =
    state.active.filter(id => ownerFactsComplete(state.facts.get(id)))

  val AuthorizationReachable: bool = ReleaseAllowed != Set()
  val CleanupRequestReachable: bool = state.cleanupRequests != Set()
  val CleanupFailureReachable: bool =
    OperationGenerations.exists(id => state.cleanup.get(id) == CleanupFailed)
  val CleanupSuccessReachable: bool =
    OperationGenerations.exists(id => state.cleanup.get(id) == CleanupSucceeded)
  val AuthoritativeReconciliationReachable: bool =
    OperationGenerations.exists(id =>
      state.facts.get(id).mediaEffect == MediaReconciled
    )
  val GenerationIsolationReachable: bool =
    state.authorizations.exists(old =>
      state.active.exists(current =>
        old.operation == current.operation and old.generation != current.generation
      )
    )
  val UnrelatedOperationReachable: bool =
    state.authorizations.exists(auth =>
      state.active.exists(current => auth.operation != current.operation)
    )

  val ApplicabilityDomainBounded: bool =
    ApplicableGenerations.subseteq(OperationGenerations)

  val TypeInvariant: bool =
    ApplicabilityDomainBounded
      and state.active.subseteq(ApplicableGenerations)
      and state.authorizations.subseteq(OperationGenerations)
      and state.cleanupRequests.subseteq(OperationGenerations)
      and state.cleanupRequests.subseteq(state.active)
      and state.cleanupRequests.subseteq(state.authorizations)
      and state.active.forall(id => state.cleanup.get(id) != CleanupSucceeded)

  val AuthorizationRequiresApplicability: bool =
    state.authorizations.forall(id => ApplicableGenerations.contains(id))

  val AuthorizationRequiresAllOwnerFacts: bool =
    state.authorizations.forall(id => ownerFactsComplete(state.facts.get(id)))

  val MissingOrIncompleteEvidenceWithholds: bool =
    ApplicableGenerations.forall(id =>
      not(ownerFactsComplete(state.facts.get(id))) implies not(state.authorizations.contains(id))
    )

  val CleanupRequestPreservesAuthorization: bool =
    state.cleanupRequests.forall(id =>
      state.authorizations.contains(id) and state.active.contains(id)
    )

  val CleanupFailurePreservesAuthorization: bool =
    OperationGenerations.forall(id =>
      state.cleanup.get(id) == CleanupFailed implies (
        state.authorizations.contains(id) and state.active.contains(id)
      )
    )

  val CleanupSuccessPreservesAuthorization: bool =
    OperationGenerations.forall(id =>
      state.cleanup.get(id) == CleanupSucceeded implies (
        state.authorizations.contains(id) and not(state.active.contains(id))
      )
    )

  val CleanupResultsRequireAuthorization: bool =
    OperationGenerations.forall(id =>
      state.cleanup.get(id) != CleanupNotRequested implies state.authorizations.contains(id)
    )

  val AuthorizationUsesExactIdentity: bool =
    state.authorizations.forall(id =>
      OperationGenerations.contains(id)
        and ownerFactsComplete(state.facts.get(id))
    )
}
