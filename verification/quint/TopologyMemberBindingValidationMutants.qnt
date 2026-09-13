/**
 * Predicate-specific mutation canaries. Every canary starts from the same
 * canonical valid baseline, injects the named defect, proves the named
 * predicate false, and proves the corresponding omitted guard would accept.
 */
module TopologyMemberBindingValidationMutants {
  import TopologyMemberBindingValidation.* from "../../models/quint/TopologyMemberBindingValidation"
  import TopologyMemberBindingValidationScenarios.* from "./TopologyMemberBindingValidationScenarios"

  pure val Assignment4: Assignment = {
    slot: 4,
    role: Data,
    position: 3,
    instance: 14,
    generation: 1,
  }

  pure val Binding4: Binding = {
    slot: 4,
    role: Data,
    position: 3,
    instance: 14,
    generation: 1,
    epoch: 4,
    store: 4,
    openedStore: 4,
    openedEpoch: 4,
    openedProtectedLength: 4096,
    openedLogicalBlockSize: 512,
  }

  pure def snapshotCoverageOmitted(snapshot: Snapshot): bool = true
  pure def snapshotUniquenessOmitted(snapshot: Snapshot): bool = true
  pure def bindingBijectionOmitted(snapshot: Snapshot, bindings: List[Binding]): bool = true
  pure def capturedFieldsOmitted(snapshot: Snapshot, bindings: List[Binding]): bool = true
  pure def openedStoresOmitted(snapshot: Snapshot, bindings: List[Binding]): bool = true
  pure def distinctnessOmitted(
    bindings: List[Binding],
    comparison: (Set[int]) => IdentityComparison,
  ): bool = true

  run assignmentUndercountMutantTest = {
    val defect = {...Snapshot21, assignments: List(Assignment1, Assignment2)}
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(snapshotCoverage(defect)),
      snapshotCoverageOmitted(defect),
    }
  }

  run assignmentOvercountMutantTest = {
    val defect = {...Snapshot21, assignments: List(Assignment1, Assignment2, Assignment3, Assignment4)}
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(snapshotCoverage(defect)),
      snapshotCoverageOmitted(defect),
    }
  }

  run rolePositionMismatchMutantTest = {
    val defect = {...Snapshot21, assignments: List(Assignment1, {...Assignment2, role: Parity}, Assignment3)}
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(snapshotCoverage(defect)),
      snapshotCoverageOmitted(defect),
    }
  }

  run duplicateSlotMutantTest = {
    val defect = {...Snapshot21, assignments: List(Assignment1, {...Assignment2, slot: 1}, Assignment3)}
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(snapshotUniqueness(defect)),
      snapshotUniquenessOmitted(defect),
    }
  }

  run duplicatePositionMutantTest = {
    val defect = {...Snapshot21, assignments: List(Assignment1, {...Assignment2, position: 0}, Assignment3)}
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(snapshotUniqueness(defect)),
      snapshotUniquenessOmitted(defect),
    }
  }

  run duplicateAssignmentInstanceMutantTest = {
    val defect = {...Snapshot21, assignments: List(Assignment1, {...Assignment2, instance: 11}, Assignment3)}
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(snapshotUniqueness(defect)),
      snapshotUniquenessOmitted(defect),
    }
  }

  run missingBindingMutantTest = {
    val defect = List(Binding1, Binding2)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(bindingBijection(Snapshot21, defect)),
      bindingBijectionOmitted(Snapshot21, defect),
    }
  }

  run extraBindingMutantTest = {
    val defect = List(Binding1, Binding2, Binding3, Binding4)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(bindingBijection(Snapshot21, defect)),
      bindingBijectionOmitted(Snapshot21, defect),
    }
  }

  run duplicateBindingSlotMutantTest = {
    val defect = List(Binding1, {...Binding2, slot: 1}, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(bindingBijection(Snapshot21, defect)),
      bindingBijectionOmitted(Snapshot21, defect),
    }
  }

  run capturedRoleMismatchMutantTest = {
    val defect = List({...Binding1, role: Parity}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(capturedFieldsMatch(Snapshot21, defect)),
      capturedFieldsOmitted(Snapshot21, defect),
    }
  }

  run capturedPositionMismatchMutantTest = {
    val defect = List({...Binding1, position: 1}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(capturedFieldsMatch(Snapshot21, defect)),
      capturedFieldsOmitted(Snapshot21, defect),
    }
  }

  run capturedAssignmentInstanceMismatchMutantTest = {
    val defect = List({...Binding1, instance: 99}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(capturedFieldsMatch(Snapshot21, defect)),
      capturedFieldsOmitted(Snapshot21, defect),
    }
  }

  run capturedGenerationMismatchMutantTest = {
    val defect = List({...Binding1, generation: 2}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(capturedFieldsMatch(Snapshot21, defect)),
      capturedFieldsOmitted(Snapshot21, defect),
    }
  }

  run bindingSnapshotEpochMismatchMutantTest = {
    val defect = List({...Binding1, epoch: 5}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(capturedFieldsMatch(Snapshot21, defect)),
      capturedFieldsOmitted(Snapshot21, defect),
    }
  }

  run declaredOpenedStoreMismatchMutantTest = {
    val defect = List({...Binding1, openedStore: 2}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(openedStoresMatch(Snapshot21, defect)),
      openedStoresOmitted(Snapshot21, defect),
    }
  }

  run duplicateStoreIdentityMutantTest = {
    val defect = List(Binding1, {...Binding2, store: 1, openedStore: 1}, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(openedStoresMatch(Snapshot21, defect)),
      openedStoresOmitted(Snapshot21, defect),
    }
  }

  run openedStoreEpochMismatchMutantTest = {
    val defect = List({...Binding1, openedEpoch: 5}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(openedStoresMatch(Snapshot21, defect)),
      openedStoresOmitted(Snapshot21, defect),
    }
  }

  run openedStoreLengthMismatchMutantTest = {
    val defect = List({...Binding1, openedProtectedLength: 8192}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(openedStoresMatch(Snapshot21, defect)),
      openedStoresOmitted(Snapshot21, defect),
    }
  }

  run openedStoreBlockMismatchMutantTest = {
    val defect = List({...Binding1, openedLogicalBlockSize: 4096}, Binding2, Binding3)
    all {
      valid(Snapshot21, Bindings21, distinctComparison),
      not(openedStoresMatch(Snapshot21, defect)),
      openedStoresOmitted(Snapshot21, defect),
    }
  }

  run acceptedSameMutantTest = all {
    valid(Snapshot21, Bindings21, distinctComparison),
    not(ownerComparisonsDistinct(Bindings21, sameFirstPair)),
    distinctnessOmitted(Bindings21, sameFirstPair),
  }

  run acceptedAmbiguousMutantTest = all {
    valid(Snapshot21, Bindings21, distinctComparison),
    not(ownerComparisonsDistinct(Bindings21, ambiguousFirstPair)),
    distinctnessOmitted(Bindings21, ambiguousFirstPair),
  }

  run acceptedLaterPairSameMutantTest = all {
    valid(Snapshot21, Bindings21, distinctComparison),
    not(ownerComparisonsDistinct(Bindings21, sameLastPair)),
    distinctnessOmitted(Bindings21, sameLastPair),
  }

  run acceptedLaterPairAmbiguousMutantTest = all {
    valid(Snapshot21, Bindings21, distinctComparison),
    not(ownerComparisonsDistinct(Bindings21, ambiguousLastPair)),
    distinctnessOmitted(Bindings21, ambiguousLastPair),
  }

  pure def positionalAssignment(snapshot: Snapshot, physicalIndex: int): Assignment =
    snapshot.assignments.nth(physicalIndex)

  run positionalLookupMutantTest = all {
    valid(Snapshot21, Bindings21, distinctComparison),
    valid(ReorderedSnapshot21, ReorderedBindings21, distinctComparison),
    assignmentForSlot(Snapshot21, 2) == assignmentForSlot(ReorderedSnapshot21, 2),
    positionalAssignment(Snapshot21, 1) != positionalAssignment(ReorderedSnapshot21, 1),
  }
}
