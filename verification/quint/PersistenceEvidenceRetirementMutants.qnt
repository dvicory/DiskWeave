/**
 * Negative evidence for the exact persistence-evidence retirement relation.
 * Each mutant bypasses one required guard or invariant. The run assertions
 * require the resulting state to violate SafetyInvariant.
 */
module PersistenceEvidenceRetirementMutants {
  import PersistenceEvidenceRetirement(
    Shapes = Set(
      {
        topology: 1,
        domain: 1,
        stores: Set(
          { store: 1, incarnation: 1, capability: 1, watermark: 0 },
          { store: 2, incarnation: 1, capability: 1, watermark: 0 },
        ),
      },
      {
        topology: 1,
        domain: 1,
        stores: Set(
          { store: 1, incarnation: 2, capability: 1, watermark: 1 },
          { store: 2, incarnation: 1, capability: 1, watermark: 0 },
        ),
      },
      {
        topology: 1,
        domain: 1,
        stores: Set(
          { store: 1, incarnation: 1, capability: 1, watermark: 1 },
          { store: 2, incarnation: 1, capability: 1, watermark: 0 },
        ),
      },
    ),
    Occurrences = Set(1, 2, 3, 4),
    RootIds = Set(1, 2, 3, 4),
    Epochs = Set(0, 1, 2, 3, 4, 5, 6, 7, 8),
    MaxFences = 4,
    MaxRoots = 4,
  ).* from "../../models/quint/PersistenceEvidenceRetirement"

  pure val ShapeA: FenceShape = {
    topology: 1,
    domain: 1,
    stores: Set(
      { store: 1, incarnation: 1, capability: 1, watermark: 0 },
      { store: 2, incarnation: 1, capability: 1, watermark: 0 },
    ),
  }
  pure val ShapeWatermarkAdvanced: FenceShape = {
    topology: 1,
    domain: 1,
    stores: Set(
      { store: 1, incarnation: 1, capability: 1, watermark: 1 },
      { store: 2, incarnation: 1, capability: 1, watermark: 0 },
    ),
  }

  pure val CleanCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 1,
    coverage: Set(1),
    generation: 0,
    watermark: 0,
  }
  pure val NewCleanCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 1,
    coverage: Set(1),
    generation: 1,
    watermark: 1,
  }
  pure val IntegrityCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 2,
    coverage: Set(2),
    generation: 0,
    watermark: 0,
  }
  pure val NewIntegrityCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 2,
    coverage: Set(2),
    generation: 1,
    watermark: 1,
  }
  pure val RecoveryCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 0,
    coverage: Set(),
    generation: 0,
    watermark: 0,
  }
  pure val ForeignRecoveryCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 4,
    coverage: Set(4),
    generation: 0,
    watermark: 0,
  }
  pure val WatermarkAdvancedCertificate: CertificateBinding = {
    shape: ShapeWatermarkAdvanced,
    target: 1,
    coverage: Set(1),
    generation: 1,
    watermark: 1,
  }

  pure val CleanBinding: ClaimBinding = {
    kind: CleanRegion,
    fact: Clean({ region: 1, cleanGeneration: 0 }),
    certificate: CleanCertificate,
  }
  pure val NewCleanBinding: ClaimBinding = {
    kind: CleanRegion,
    fact: Clean({ region: 1, cleanGeneration: 1 }),
    certificate: NewCleanCertificate,
  }
  pure val IntegrityBinding: ClaimBinding = {
    kind: ValidIntegrity,
    fact: Integrity({
      extent: 2,
      profile: 1,
      setGeneration: 0,
      contentGeneration: 0,
      digest: 7,
    }),
    certificate: IntegrityCertificate,
  }
  pure val NewIntegrityBinding: ClaimBinding = {
    kind: ValidIntegrity,
    fact: Integrity({
      extent: 2,
      profile: 1,
      setGeneration: 1,
      contentGeneration: 1,
      digest: 8,
    }),
    certificate: NewIntegrityCertificate,
  }
  pure val RecoveryBinding: ClaimBinding = {
    kind: UnresolvedRecoveryCommit,
    fact: Recovery({ intent: 1, priorToken: 1, proposedToken: 2 }),
    certificate: RecoveryCertificate,
  }
  pure val ForeignRecoveryBinding: ClaimBinding = {
    kind: UnresolvedRecoveryCommit,
    fact: Recovery({ intent: 1, priorToken: 1, proposedToken: 2 }),
    certificate: ForeignRecoveryCertificate,
  }

  pure val Intent1: RetirementIntent = { intent: 1, priorToken: 1, proposedToken: 2 }
  pure val Intent3: RetirementIntent = { intent: 1, priorToken: 3, proposedToken: 4 }

  action mutantTopologyStaleUnknown: bool = all {
    state.phase == Pending,
    state.snapshot.topology != state.pending.expectedTopology,
    state' = {
      ...state,
      phase: ReconciliationRequired,
      pending: {
        ...state.pending,
        unresolved: Unresolved({ rootId: 3, binding: RecoveryBinding }),
      },
      lastOutcome: Unknown,
    },
  }

  action mutantGenerationStaleUnknown: bool = all {
    state.phase == Pending,
    state.snapshot.generation != state.pending.expectedGeneration,
    state' = {
      ...state,
      phase: ReconciliationRequired,
      pending: {
        ...state.pending,
        unresolved: Unresolved({ rootId: 3, binding: RecoveryBinding }),
      },
      lastOutcome: Unknown,
    },
  }

  action mutantDropsUnrelatedRoot: bool = all {
    state.phase == Stable,
    state.snapshot.roots.exists(root => root.occurrence == 1),
    state.snapshot.fences.exists(fence => fence.occurrence == 2),
    state' = {
      ...state,
      phase: Pending,
      pending: {
        successor: {
          ...state.snapshot,
          fences: removeFence(state.snapshot.fences, 2),
          roots: Set(),
        },
        retire: Set(2),
        expectedGeneration: state.snapshot.generation,
        expectedTopology: state.snapshot.topology,
        intent: intentFor(state.snapshot),
        unresolved: NoUnresolved,
      },
      lastOutcome: NoOutcome,
    },
  }

  action mutantUnknownUsesForeignCertificate: bool = all {
    state.phase == Pending,
    state.pending.retire == Set(1),
    state' = {
      ...state,
      phase: ReconciliationRequired,
      pending: {
        ...state.pending,
        unresolved: Unresolved({ rootId: 3, binding: ForeignRecoveryBinding }),
      },
      lastOutcome: Unknown,
    },
  }

  action mutantConflatesRebindDestinations: bool = all {
    state.phase == Stable,
    state.snapshot.roots.size() == 2,
    state.snapshot.fences.exists(fence => fence.occurrence == 2),
    state.snapshot.fences.exists(fence => fence.occurrence == 3),
    state' = {
      ...state,
      phase: Pending,
      pending: {
        successor: {
          ...state.snapshot,
          fences: removeFence(state.snapshot.fences, 1),
          roots: Set(
            { id: 1, occurrence: 2, binding: NewCleanBinding },
            { id: 2, occurrence: 2, binding: NewIntegrityBinding },
          ),
        },
        retire: Set(1),
        expectedGeneration: state.snapshot.generation,
        expectedTopology: state.snapshot.topology,
        intent: intentFor(state.snapshot),
        unresolved: NoUnresolved,
      },
      lastOutcome: NoOutcome,
    },
  }

  run topologyStaleUnknownMutantTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(advanceTopology)
      .then(mutantTopologyStaleUnknown)
      .expect(
        state.phase == ReconciliationRequired
          and not(UnknownHasExactPredecessor)
          and not(SafetyInvariant)
      )

  run generationStaleUnknownMutantTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(advanceGeneration)
      .then(mutantGenerationStaleUnknown)
      .expect(
        state.phase == ReconciliationRequired
          and not(UnknownHasExactPredecessor)
          and not(SafetyInvariant)
      )

  run unrelatedRootDropMutantTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeA, Set()))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(mutantDropsUnrelatedRoot)
      .expect(
        state.phase == Pending
          and not(PendingPreservesUnrelatedRoots)
          and not(SafetyInvariant)
      )

  run foreignUnknownCertificateMutantTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(mutantUnknownUsesForeignCertificate)
      .expect(
        state.phase == ReconciliationRequired
          and not(UnknownHasExactPredecessor)
          and not(SafetyInvariant)
      )

  run oneSuccessorForAllRootsMutantTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate, IntegrityCertificate)))
      .then(submitFence(2, ShapeA, Set(NewCleanCertificate)))
      .then(submitFence(3, ShapeA, Set(NewIntegrityCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(admitRoot(2, 1, IntegrityBinding, IntegrityAdmission))
      .then(mutantConflatesRebindDestinations)
      .expect(
        state.phase == Pending
          and not(PendingSuccessorIsSelfContained)
          and not(SafetyInvariant)
      )
}
