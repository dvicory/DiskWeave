/**
 * Wider direct scenarios for the reviewed retirement relation. This instance
 * broadens root, coverage, and owner-class combinations without putting them
 * in the small exhaustive checker.
 */
module PersistenceEvidenceRetirementWideAnalysis {
  import PersistenceEvidenceRetirement(
    Shapes = Set(
      {
        topology: 1,
        domain: 1,
        stores: Set(
          { store: 1, incarnation: 1, capability: 1, watermark: 0 },
          { store: 2, incarnation: 1, capability: 1, watermark: 0 },
          { store: 3, incarnation: 1, capability: 1, watermark: 0 },
        ),
      },
      {
        topology: 1,
        domain: 1,
        stores: Set(
          { store: 1, incarnation: 2, capability: 1, watermark: 1 },
          { store: 2, incarnation: 1, capability: 1, watermark: 0 },
          { store: 3, incarnation: 1, capability: 1, watermark: 0 },
        ),
      },
      {
        topology: 2,
        domain: 2,
        stores: Set(
          { store: 1, incarnation: 1, capability: 2, watermark: 1 },
          { store: 2, incarnation: 1, capability: 1, watermark: 0 },
          { store: 3, incarnation: 1, capability: 1, watermark: 0 },
        ),
      },
    ),
    Occurrences = Set(1, 2, 3, 4, 5),
    RootIds = Set(1, 2, 3, 4, 5, 6, 7, 8),
    Epochs = Set(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16),
    MaxFences = 5,
    MaxRoots = 8,
  ).* from "../../models/quint/PersistenceEvidenceRetirement"

  pure val ShapeA: FenceShape = {
    topology: 1,
    domain: 1,
    stores: Set(
      { store: 1, incarnation: 1, capability: 1, watermark: 0 },
      { store: 2, incarnation: 1, capability: 1, watermark: 0 },
      { store: 3, incarnation: 1, capability: 1, watermark: 0 },
    ),
  }
  pure val ShapeB: FenceShape = {
    topology: 1,
    domain: 1,
    stores: Set(
      { store: 1, incarnation: 2, capability: 1, watermark: 1 },
      { store: 2, incarnation: 1, capability: 1, watermark: 0 },
      { store: 3, incarnation: 1, capability: 1, watermark: 0 },
    ),
  }

  pure val CleanCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 1,
    coverage: Set(1, 2),
    generation: 0,
    watermark: 0,
  }
  pure val NewCleanCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 1,
    coverage: Set(1, 2),
    generation: 1,
    watermark: 1,
  }
  pure val ShapeBNewCleanCertificate: CertificateBinding = {
    shape: ShapeB,
    target: 1,
    coverage: Set(1, 2),
    generation: 1,
    watermark: 1,
  }
  pure val IntegrityCertificate: CertificateBinding = {
    shape: ShapeA,
    target: 2,
    coverage: Set(2, 3),
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
    coverage: Set(1, 2, 3),
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
    target: 5,
    coverage: Set(1, 2),
    generation: 0,
    watermark: 0,
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
  pure val ShapeBNewCleanBinding: ClaimBinding = {
    ...NewCleanBinding,
    certificate: ShapeBNewCleanCertificate,
  }
  pure val IntegrityBinding: ClaimBinding = {
    kind: ValidIntegrity,
    fact: Integrity({
      extent: 2,
      profile: 3,
      setGeneration: 1,
      contentGeneration: 4,
      digest: 9,
    }),
    certificate: IntegrityCertificate,
  }
  pure val SessionBinding: ClaimBinding = {
    kind: WritableSession,
    fact: Session({ session: 3, closeGeneration: 4 }),
    certificate: SessionCertificate,
  }
  pure val CaptureBinding: ClaimBinding = {
    kind: CodedCapture,
    fact: Capture({ capture: 4, phase: CaptureCommitPending }),
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

  pure val Intent1: RetirementIntent = { intent: 1, priorToken: 1, proposedToken: 2 }
  pure val Intent3: RetirementIntent = { intent: 1, priorToken: 3, proposedToken: 4 }
  pure val Intent4: RetirementIntent = { intent: 1, priorToken: 4, proposedToken: 5 }
  pure val Intent10: RetirementIntent = { intent: 1, priorToken: 10, proposedToken: 11 }

  pure val RootNewClean: Root = { id: 1, occurrence: 2, binding: NewCleanBinding }

  run wideMixedRootPlanTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate, IntegrityCertificate)))
      .then(submitFence(2, ShapeA, Set(NewCleanCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(admitRoot(2, 1, IntegrityBinding, IntegrityAdmission))
      .then(preparePlannedRetirement(
        1,
        Set(RootNewClean),
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
        not(state.snapshot.fences.exists(fence => fence.occurrence == 1))
          and state.snapshot.fences.exists(fence => fence.occurrence == 2)
          and state.snapshot.roots == Set(RootNewClean)
          and ExactRootBindings
      )

  run wideOwnerClassesRemainIndependentTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeA, Set(IntegrityCertificate)))
      .then(submitFence(3, ShapeA, Set(CaptureCertificate)))
      .then(submitFence(4, ShapeA, Set(RecoveryCertificate)))
      .then(submitFence(5, ShapeA, Set(LegacyCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .then(admitRoot(2, 2, IntegrityBinding, IntegrityAdmission))
      .then(admitRoot(3, 3, CaptureBinding, CaptureAdmission))
      .then(admitRoot(4, 4, RecoveryBinding, RecoveryAdmission))
      .then(admitRoot(5, 5, LegacyBinding, LegacyAdmission))
      .then(preparePlannedRetirement(
        3,
        Set(
          { id: 1, occurrence: 1, binding: CleanBinding },
          { id: 2, occurrence: 2, binding: IntegrityBinding },
          { id: 4, occurrence: 4, binding: RecoveryBinding },
          { id: 5, occurrence: 5, binding: LegacyBinding },
        ),
        Set(),
        Set({ rootId: 3, proof: CaptureDischargeProof }),
        Intent10,
      ))
      .then(commitPrepared)
      .expect(
        state.snapshot.fences.size() == 4
          and state.snapshot.roots.size() == 4
          and ExactRootBindings
      )

  run wideSessionCannotBeRetiredGenericallyTest =
    init
      .then(submitFence(1, ShapeA, Set(SessionCertificate)))
      .then(submitFence(2, ShapeA, Set()))
      .then(admitRoot(1, 1, SessionBinding, SessionAdmission))
      .expect(ExactRootBindings)
      .then(preparePlannedRetirement(
        1,
        Set(),
        Set(),
        Set({ rootId: 1, proof: CleanDischargeProof }),
        Intent3,
      ).fail())

  run wideExactCoverageAndShapeMismatchTest =
    init
      .then(submitFence(1, ShapeA, Set(CleanCertificate)))
      .then(submitFence(2, ShapeB, Set(ShapeBNewCleanCertificate)))
      .then(admitRoot(1, 1, CleanBinding, CleanAdmission))
      .expect(ExactRootBindings)
      .then(preparePlannedRetirement(
        1,
        Set({ id: 1, occurrence: 2, binding: ShapeBNewCleanBinding }),
        Set({
          rootId: 1,
          successorOccurrence: 2,
          binding: ShapeBNewCleanBinding,
          proof: CleanRebindProof,
        }),
        Set(),
        Intent3,
      ).fail())

  run wideUnknownRecoveryRetainsPredecessorTest =
    init
      .then(submitFence(1, ShapeA, Set(RecoveryCertificate)))
      .then(prepareUnboundRetirement(1, Intent1))
      .then(observeRetirementUnknown(6, RecoveryBinding))
      .then(reconcileNeither)
      .expect(
        state.phase == ReconciliationRequired
          and state.snapshot.fences.exists(fence => fence.occurrence == 1)
          and state.pending.unresolved != NoUnresolved
      )
      .then(reconcileSuccessor)
      .expect(not(state.snapshot.fences.exists(fence => fence.occurrence == 1)))
}
