/**
 * Bounded production correspondence profile for exact fence retirement.
 *
 * The imported retention relation remains the semantic authority. This profile
 * exercises one validated CLEAN-owner root, exact occurrence identity, and an
 * unbound successor retirement. Private pending intent and owner-fact
 * construction are not projected as state.
 */
module PersistenceEvidenceRetirementConnect {
  import PersistenceEvidenceRetirement(
    Shapes = Set({
      topology: 1,
      domain: 1,
      stores: Set({ store: 1, incarnation: 1, capability: 1, watermark: 0 }),
    }),
    Occurrences = Set(1, 2, 3, 4),
    RootIds = Set(1, 2, 3, 4),
    Epochs = Set(0, 1, 2, 3, 4),
    MaxFences = 4,
    MaxRoots = 4,
  ) as retention from "../../models/quint/PersistenceEvidenceRetirement"

  pure val ShapeA: retention::FenceShape = {
    topology: 1,
    domain: 1,
    stores: Set({ store: 1, incarnation: 1, capability: 1, watermark: 0 }),
  }

  pure val CleanCertificate: retention::CertificateBinding = {
    shape: ShapeA,
    target: 1,
    coverage: Set(1),
    generation: 1,
    watermark: 0,
  }

  pure val CleanBinding: retention::ClaimBinding = {
    kind: retention::CleanRegion,
    fact: retention::Clean({ region: 1, cleanGeneration: 1 }),
    certificate: CleanCertificate,
  }

  pure val Intent3: retention::RetirementIntent = {
    intent: 1,
    priorToken: 3,
    proposedToken: 4,
  }

  action connectInit: bool = retention::init

  action appendFirstFence: bool = retention::submitFence(1, ShapeA, Set(CleanCertificate))

  action admitCleanRoot: bool = all {
    retention::state.snapshot.generation == 1,
    retention::admitRoot(1, 1, CleanBinding, retention::CleanAdmission),
  }

  action appendSecondFence: bool = all {
    retention::state.snapshot.generation == 2,
    retention::state.snapshot.roots.size() == 1,
    retention::submitFence(2, ShapeA, Set()),
  }

  action prepareSecondRetirement: bool = all {
    retention::state.snapshot.generation == 3,
    retention::state.snapshot.roots.size() == 1,
    retention::prepareUnboundRetirement(2, Intent3),
  }

  action commitRetirement: bool = retention::commitPrepared

  action connectStep: bool = any {
    appendFirstFence,
    admitCleanRoot,
    appendSecondFence,
    prepareSecondRetirement,
    commitRetirement,
  }
}
