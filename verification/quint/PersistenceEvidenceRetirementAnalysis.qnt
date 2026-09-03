/**
 * Focused retirement relation scenarios. Owner-qualified facts are inputs;
 * the model checks exact certificate/root and retirement behavior.
 */
module PersistenceEvidenceRetirementAnalysis {
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
        topology: 2,
        domain: 1,
        stores: Set(
          { store: 1, incarnation: 1, capability: 2, watermark: 1 },
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
  pure val ShapeB: FenceShape = {
    topology: 1,
    domain: 1,
    stores: Set(
      { store: 1, incarnation: 2, capability: 1, watermark: 1 },
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
  pure val NewIntegrityCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 2,
    coverage: Set(2),
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
  pure val SessionCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 3,
    coverage: Set(3),
    generation: 0,
    watermark: 0,
  }
  pure val CaptureCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 4,
    coverage: Set(1, 2),
    generation: 0,
    watermark: 0,
  }
  pure val RecoveryCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 0,
    coverage: Set(),
    generation: 0,
    watermark: 0,
  }
  pure val LegacyCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 1,
    coverage: Set(1),
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
  pure val SessionBinding: ClaimBinding = {
    kind: WritableSession,
    fact: Session({ session: 3, closeGeneration: 0 }),
    certificate: SessionCertificate,
  }
  pure val CaptureBinding: ClaimBinding = {
    kind: CodedCapture,
    fact: Capture({ capture: 4, phase: CaptureOpen }),
    certificate: CaptureCertificate,
  }
  pure val RecoveryBinding: ClaimBinding = {
    kind: UnresolvedRecoveryCommit,
    fact: Recovery({ intent: 1, priorToken: 1, proposedToken: 2 }),
    certificate: RecoveryCertificate,
  }
  pure val LegacyBinding: ClaimBinding = {
    kind: LegacyUnreconciled,
    fact: Legacy({ schema: 6, position: 0, candidates: Set(0, 1) }),
    certificate: LegacyCertificate,
  }
 
  pure val RootTwoAtNewIntegrity: Root = {
    id: 2,
    occurrence: 3,
    binding: NewIntegrityBinding,
  }
  pure val NewWatermarkBinding: ClaimBinding = {
    ...NewCleanBinding,
    certificate: WatermarkAdvancedCertificate,
  }

  pure val Intent1: RetirementIntent = { intent: 1, priorToken: 1, proposedToken: 2 }
  pure val Intent2: RetirementIntent = { intent: 1, priorToken: 2, proposedToken: 3 }
  pure val Intent3: RetirementIntent = { intent: 1, priorToken: 3, proposedToken: 4 }
  pure val Intent4: RetirementIntent = { intent: 1, priorToken: 4, proposedToken: 5 }
  pure val Intent5: RetirementIntent = { intent: 1, priorToken: 5, proposedToken: 6 }

  pure val RootOneAtNewClean: Root = {
    id: 1,
    occurrence: 2,
    binding: NewCleanBinding,
  }

  val fenceOnePresent: bool = state.snapshot.fences.exists(fence => fence.occurrence == 1)
  val fenceTwoPresent: bool = state.snapshot.fences.exists(fence => fence.occurrence == 2)
  val rootOneAtFenceOne: bool = state.snapshot.roots.exists(root =>
    root.id == 1 and root.occurrence == 1
  )
  val rootOneAtFenceTwo: bool = state.snapshot.roots.exists(root =>
    root.id == 1 and root.occurrence == 2
  )

  action planStep: bool = any {
    submitFence(1, ShapeA, Set(CleanCertificate, IntegrityCertificate)),
    submitFence(2, ShapeA, Set(NewCleanCertificate)),
    admitRoot(1, 1, CleanBinding, CleanAdmission),
    admitRoot(2, 1, IntegrityBinding, IntegrityAdmission),
    preparePlannedRetirement(
      1,
      Set(RootOneAtNewClean),
      Set({
        rootId: 1,
        successorOccurrence: 2,
        binding: NewCleanBinding,
        proof: CleanRebindProof,
      }),
      Set({ rootId: 2, proof: IntegrityDischargeProof }),
      Intent4,
    ),
    commitPrepared,
    rejectPrepared,
    rejectStale,
    reconcilePrior,
    reconcileSuccessor,
    reconcileNeither,
    advanceGeneration,
    advanceTopology,
  }

  action unknownStep: bool = any {
    submitFence(1, ShapeA, Set(RecoveryCertificate)),
    prepareUnboundRetirement(1, Intent1),
    observeRetirementUnknown(3, RecoveryBinding),
    reconcilePrior,
    reconcileSuccessor,
    reconcileNeither,
  }

  run boundedAssumptionsTest = all {
    Shapes.size() == 4,
    Shapes.contains(ShapeA),
    Shapes.contains(ShapeB),
    Shapes.contains(ShapeWatermarkAdvanced),
    Occurrences == Set(1, 2, 3, 4),
    RootIds == Set(1, 2, 3, 4),
    Epochs == Set(0, 1, 2, 3, 4, 5, 6, 7, 8),
    MaxFences == 4,
    MaxRoots == 4,
  }

  run cleanRootBlocksRetirementTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .expect(fenceOnePresent and rootOneAtFenceOne)
      .then(prepareUnboundRetirement(1, Intent2).fail())

  run dischargedCleanRootAllowsRetirementTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeA, Set()))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(preparePlannedRetirement(
        1,
        Set(),
        Set(),
        Set({ rootId: 1, proof: CleanDischargeProof }),
        Intent3,
      ))
      .then(commitPrepared)
      .expect(not(fenceOnePresent) and fenceTwoPresent and ExactRootBindings)

  run exactMixedRebindAndDischargeTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate, IntegrityCertificate)))
      .then(submitFence(2, ShapeA, Set(NewCleanCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(admitRoot(2, 1, IntegrityBinding, IntegrityAdmission))
      .then(preparePlannedRetirement(
        1,
        Set(RootOneAtNewClean),
        Set({
          rootId: 1,
          successorOccurrence: 2,
          binding: NewCleanBinding,
          proof: CleanRebindProof,
        }),
        Set({ rootId: 2, proof: IntegrityDischargeProof }),
        Intent4,
      ))
      .then(commitPrepared)
      .expect(
        not(fenceOnePresent)
          and fenceTwoPresent
          and rootOneAtFenceTwo
          and state.snapshot.roots.size() == 1
          and ExactRootBindings
      )
 
  run distinctSuccessorRebindsAreAtomicTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate, IntegrityCertificate)))
      .then(submitFence(2, ShapeA, Set(NewCleanCertificate)))
      .then(submitFence(3, ShapeA, Set(NewIntegrityCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(admitRoot(2, 1, IntegrityBinding, IntegrityAdmission))
      .then(preparePlannedRetirement(
        1,
        Set(RootOneAtNewClean, RootTwoAtNewIntegrity),
        Set({
          rootId: 1,
          successorOccurrence: 2,
          binding: NewCleanBinding,
          proof: CleanRebindProof,
        }, {
          rootId: 2,
          successorOccurrence: 3,
          binding: NewIntegrityBinding,
          proof: IntegrityRebindProof,
        }),
        Set(),
        Intent5,
      ))
      .then(commitPrepared)
      .expect(
        not(fenceOnePresent)
          and state.snapshot.fences.exists(fence => fence.occurrence == 2)
          and state.snapshot.fences.exists(fence => fence.occurrence == 3)
          and state.snapshot.roots == Set(RootOneAtNewClean, RootTwoAtNewIntegrity)
          and ExactRootBindings
      )

  run equalOccurrencesRemainDistinctTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeA, Set(CleanCertificate)))
      .then(prepareUnboundRetirement(2, Intent2))
      .then(commitPrepared)
      .expect(fenceOnePresent and not(fenceTwoPresent))

  run occurrenceIdentityCannotBeReusedTest =
    init
      .then(submitFence(1, ShapeA, Set()))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(commitPrepared)
      .expect(
        not(fenceOnePresent)
          and state.snapshot.allocatedOccurrences == Set(1)
      )
      .then(submitFence(1, ShapeA, Set()).fail())

  run partialCoveragePreservesPredecessorTest =
    init
      .then(submitFence(1, ShapeA, Set({
        ...CleanCertificate,
        coverage: Set(1, 2),
      })))
      .then(submitFence(2, ShapeA, Set(NewCleanCertificate)))
      .then(admitRoot(1, 1, {
        ...CleanBinding,
        certificate: {
          ...CleanCertificate,
          coverage: Set(1, 2),
        },
      }, CleanAdmission))
      .expect(fenceOnePresent and rootOneAtFenceOne)
      .then(preparePlannedRetirement(
        1,
        Set(RootOneAtNewClean),
        Set({
          rootId: 1,
          successorOccurrence: 2,
          binding: NewCleanBinding,
          proof: CleanRebindProof,
        }),
        Set(),
        Intent3,
      ).fail())

  run shapeMismatchPreservesPredecessorTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeB, Set({
        ...NewCleanCertificate,
        shape: ShapeB,
      })))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .expect(fenceOnePresent and rootOneAtFenceOne)
      .then(preparePlannedRetirement(
        1,
        Set({
          id: 1,
          occurrence: 2,
          binding: {
            ...NewCleanBinding,
            certificate: {
              ...NewCleanCertificate,
              shape: ShapeB,
            },
          },
        }),
        Set({
          rootId: 1,
          successorOccurrence: 2,
          binding: {
            ...NewCleanBinding,
            certificate: {
              ...NewCleanCertificate,
              shape: ShapeB,
            },
          },
          proof: CleanRebindProof,
        }),
        Set(),
        Intent3,
      ).fail())

  run perStoreWatermarkAdvanceAllowsRebindTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeWatermarkAdvanced, Set(WatermarkAdvancedCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(preparePlannedRetirement(
        1,
        Set({ id: 1, occurrence: 2, binding: NewWatermarkBinding }),
        Set({
          rootId: 1,
          successorOccurrence: 2,
          binding: NewWatermarkBinding,
          proof: CleanRebindProof,
        }),
        Set(),
        Intent3,
      ))
      .then(commitPrepared)
      .expect(not(fenceOnePresent) and rootOneAtFenceTwo and ExactRootBindings)

  run sessionRootCannotGenericRetirementTest =
    init
      .then(submitFence(1, ShapeA, Set(SessionCertificate)))
      .then(admitRoot(1, 1, SessionBinding, SessionAdmission))
      .expect(fenceOnePresent and ExactRootBindings)
      .then(prepareUnboundRetirement(1, Intent2).fail())

  run unknownRetirementRetainsPredecessorTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(observeRetirementUnknown(3, RecoveryBinding))
      .expect(
        state.phase == ReconciliationRequired
          and fenceOnePresent
          and state.pending.unresolved != NoUnresolved
          and UnknownRetainsRetirementPredecessor
          and UnknownRetainsExactIntentRoot
      )
      .then(reconcileNeither)
      .expect(state.phase == ReconciliationRequired and fenceOnePresent)
      .then(reconcilePrior)
      .expect(fenceOnePresent and state.lastOutcome == Rejected)

  run unknownRetirementCanInstallSuccessorTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(observeRetirementUnknown(3, RecoveryBinding))
      .then(reconcileSuccessor)
      .expect(not(fenceOnePresent) and state.lastOutcome == Committed)

  run staleGenerationPreservesPredecessorTest =
    init
      .then(submitFence(1, ShapeA, Set()))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(advanceGeneration)
      .then(rejectStale)
      .expect(fenceOnePresent and state.lastOutcome == Stale)

  run staleUnknownCannotInstallSuccessorTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(advanceGeneration)
      .expect(state.phase == Pending and fenceOnePresent)
      .then(observeRetirementUnknown(3, RecoveryBinding).fail())
 
  run staleTopologyCannotEnterUnknownTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(advanceTopology)
      .expect(state.phase == Pending and fenceOnePresent)
      .then(observeRetirementUnknown(3, RecoveryBinding).fail())

  run staleTopologyPreservesPredecessorTest =
    init
      .then(submitFence(1, ShapeA, Set()))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(advanceTopology)
      .then(rejectStale)
      .expect(fenceOnePresent and state.lastOutcome == Stale)

  run legacyRootRequiresTypedOwnerProofTest =
    init
      .then(submitFence(1, ShapeA, Set(LegacyCertificate)))
      .then(admitRoot(1, 1, LegacyBinding, LegacyAdmission))
      .expect(fenceOnePresent and ExactRootBindings)
      .then(prepareUnboundRetirement(1, Intent2).fail())

  run allRootKindsRemainExactTest =
    init
      .then(submitFence(1, ShapeA, Set(
        CleanCertificate,
        IntegrityCertificate,
        SessionCertificate,
        CaptureCertificate,
        RecoveryCertificate,
        LegacyCertificate,
      )))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(admitRoot(2, 1, IntegrityBinding, IntegrityAdmission))
      .then(admitRoot(3, 1, SessionBinding, SessionAdmission))
      .then(admitRoot(4, 1, CaptureBinding, CaptureAdmission))
      .expect(state.snapshot.roots.size() == 4 and ExactRootBindings)
}
