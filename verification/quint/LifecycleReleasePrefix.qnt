/**
 * One-generation exhaustive profiles for the delegated LifecycleRelease relation.
 * The profile narrows only the finite evidence domain; canonical actions and
 * invariants remain imported from LifecycleRelease.qnt.
 */
module LifecycleReleasePrefix {
  import LifecycleRelease(
    OperationGenerations = Set(
      { operation: "slot-a", generation: 0 },
    ),
    ApplicableGenerations = Set(
      { operation: "slot-a", generation: 0 },
    ),
  ).* from "../../models/quint/LifecycleRelease"
}
