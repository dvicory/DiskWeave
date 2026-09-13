/**
 * Focused exhaustive profile over the explicitly enumerated ValidationCases.
 * Bounds: slots {1,2,3,4}; instances {11,12,13,14}; stores {1,2,3,4};
 * assignment cardinalities {2,3}; binding cardinalities {2,3,4};
 * generations {1,2}; epochs {4,5}; profiles {(1,1),(2,1),(1,2)};
 * owner-accepted geometries {(4096,4096,512),(8192,8192,512)}.
 * Each element is a complete supplied input, including one owner comparison
 * outcome for every bound pair. Exhaustiveness is over this finite case set.
 */
module TopologyMemberBindingValidationAnalysis {
  import TopologyMemberBindingValidation.* from "../../models/quint/TopologyMemberBindingValidation"
  import TopologyMemberBindingValidationScenarios.* from "./TopologyMemberBindingValidationScenarios"

  type ComparisonMode = AllDistinct | FirstSame | FirstAmbiguous | LastSame | LastAmbiguous

  type ValidationCase = {
    snapshot: Snapshot,
    bindings: List[Binding],
    comparisonMode: ComparisonMode,
  }

  pure def suppliedComparison(
    mode: ComparisonMode,
    pair: Set[int],
  ): IdentityComparison =
    match mode {
      | AllDistinct => Distinct
      | FirstSame => sameFirstPair(pair)
      | FirstAmbiguous => ambiguousFirstPair(pair)
      | LastSame => sameLastPair(pair)
      | LastAmbiguous => ambiguousLastPair(pair)
    }

  pure def caseValid(case: ValidationCase): bool = valid(
    case.snapshot,
    case.bindings,
    pair => suppliedComparison(case.comparisonMode, pair),
  )

  pure val GeometryB: Geometry = {
    protectedLength: 8192,
    parityLength: 8192,
    logicalBlockSize: 512,
  }

  pure val Snapshot21GeometryB: Snapshot = {...Snapshot21, geometry: GeometryB}
  pure val Bindings21GeometryB: List[Binding] = List(
    {...Binding1, openedProtectedLength: 8192},
    {...Binding2, openedProtectedLength: 8192},
    {...Binding3, openedProtectedLength: 8192}
  )

  pure val Binding4: Binding = {
    ...Binding3,
    slot: 4,
    position: 3,
    instance: 14,
    store: 4,
    openedStore: 4,
  }

  pure val FocusedCases: Set[ValidationCase] = Set(
    { snapshot: Snapshot11, bindings: Bindings11, comparisonMode: AllDistinct },
    { snapshot: Snapshot21, bindings: Bindings21, comparisonMode: AllDistinct },
    { snapshot: Snapshot12, bindings: Bindings12, comparisonMode: AllDistinct },
    { snapshot: Snapshot21GeometryB, bindings: Bindings21GeometryB, comparisonMode: AllDistinct },
    {
      snapshot: {...Snapshot21, assignments: List(Assignment1, Assignment2)},
      bindings: Bindings21,
      comparisonMode: AllDistinct,
    },
    {
      snapshot: {...Snapshot21, assignments: List(Assignment1, {...Assignment2, role: Parity}, Assignment3)},
      bindings: Bindings21,
      comparisonMode: AllDistinct,
    },
    {
      snapshot: {...Snapshot21, assignments: List(Assignment1, {...Assignment2, slot: 1}, Assignment3)},
      bindings: Bindings21,
      comparisonMode: AllDistinct,
    },
    { snapshot: Snapshot21, bindings: List(Binding1, Binding2), comparisonMode: AllDistinct },
    { snapshot: Snapshot21, bindings: List(Binding1, Binding2, Binding3, Binding4), comparisonMode: AllDistinct },
    {
      snapshot: Snapshot21,
      bindings: List({...Binding1, generation: 2}, Binding2, Binding3),
      comparisonMode: AllDistinct,
    },
    {
      snapshot: Snapshot21,
      bindings: List({...Binding1, openedStore: 2}, Binding2, Binding3),
      comparisonMode: AllDistinct,
    },
    { snapshot: Snapshot21, bindings: Bindings21, comparisonMode: FirstSame },
    { snapshot: Snapshot21, bindings: Bindings21, comparisonMode: FirstAmbiguous },
    { snapshot: Snapshot21, bindings: Bindings21, comparisonMode: LastSame },
    { snapshot: Snapshot21, bindings: Bindings21, comparisonMode: LastAmbiguous }
  )

  type PermutationCase = {
    first: ValidationCase,
    second: ValidationCase,
    slot: int,
    position: int,
  }

  pure val ReorderedSnapshot11: Snapshot = {
    ...Snapshot11,
    assignments: List(Snapshot11.assignments.nth(1), Snapshot11.assignments.nth(0)),
  }
  pure val ReorderedBindings11: List[Binding] =
    List(Bindings11.nth(1), Bindings11.nth(0))
  pure val ReorderedSnapshot12: Snapshot = {
    ...Snapshot12,
    assignments: List(
      Snapshot12.assignments.nth(2),
      Snapshot12.assignments.nth(0),
      Snapshot12.assignments.nth(1)
    ),
  }
  pure val ReorderedBindings12: List[Binding] =
    List(Bindings12.nth(1), Bindings12.nth(2), Bindings12.nth(0))
  pure val ReorderedSnapshot21GeometryB: Snapshot = {
    ...ReorderedSnapshot21,
    geometry: GeometryB,
  }
  pure val ReorderedBindings21GeometryB: List[Binding] = List(
    {...Binding2, openedProtectedLength: 8192},
    {...Binding3, openedProtectedLength: 8192},
    {...Binding1, openedProtectedLength: 8192}
  )

  pure val PermutationCases: Set[PermutationCase] = Set(
    {
      first: { snapshot: Snapshot11, bindings: Bindings11, comparisonMode: AllDistinct },
      second: { snapshot: ReorderedSnapshot11, bindings: ReorderedBindings11, comparisonMode: AllDistinct },
      slot: 1,
      position: 0,
    },
    {
      first: { snapshot: Snapshot21, bindings: Bindings21, comparisonMode: AllDistinct },
      second: { snapshot: ReorderedSnapshot21, bindings: ReorderedBindings21, comparisonMode: AllDistinct },
      slot: 2,
      position: 1,
    },
    {
      first: { snapshot: Snapshot12, bindings: Bindings12, comparisonMode: AllDistinct },
      second: { snapshot: ReorderedSnapshot12, bindings: ReorderedBindings12, comparisonMode: AllDistinct },
      slot: 3,
      position: 2,
    },
    {
      first: { snapshot: Snapshot21GeometryB, bindings: Bindings21GeometryB, comparisonMode: AllDistinct },
      second: {
        snapshot: ReorderedSnapshot21GeometryB,
        bindings: ReorderedBindings21GeometryB,
        comparisonMode: AllDistinct,
      },
      slot: 2,
      position: 1,
    },
    {
      first: {
        snapshot: Snapshot21,
        bindings: List({...Binding1, generation: 2}, Binding2, Binding3),
        comparisonMode: LastSame,
      },
      second: {
        snapshot: ReorderedSnapshot21,
        bindings: List(Binding2, Binding3, {...Binding1, generation: 2}),
        comparisonMode: LastSame,
      },
      slot: 1,
      position: 0,
    }
  )

  var chosen: ValidationCase
  var permutation: PermutationCase

  action init: bool = {
    nondet candidate = FocusedCases.oneOf()
    nondet selectedPermutation = PermutationCases.oneOf()
    all {
      chosen' = candidate,
      permutation' = selectedPermutation,
    }
  }

  action step: bool = all {
    chosen' = chosen,
    permutation' = permutation,
  }

  val ValidIffAllDelegatedPredicates: bool =
    caseValid(chosen) == all {
      snapshotCoverage(chosen.snapshot),
      snapshotUniqueness(chosen.snapshot),
      bindingBijection(chosen.snapshot, chosen.bindings),
      capturedFieldsMatch(chosen.snapshot, chosen.bindings),
      openedStoresMatch(chosen.snapshot, chosen.bindings),
      ownerComparisonsDistinct(
        chosen.bindings,
        pair => suppliedComparison(chosen.comparisonMode, pair),
      ),
    }

  val AcceptedSnapshotIsUnambiguous: bool =
    not(caseValid(chosen)) or all {
      snapshotCoverage(chosen.snapshot),
      snapshotUniqueness(chosen.snapshot),
    }

  val AcceptedBindingsAreBijective: bool =
    not(caseValid(chosen)) or bindingBijection(chosen.snapshot, chosen.bindings)

  val AcceptedBindingsMatchCapturedFacts: bool =
    not(caseValid(chosen)) or all {
      capturedFieldsMatch(chosen.snapshot, chosen.bindings),
      openedStoresMatch(chosen.snapshot, chosen.bindings),
    }

  val AcceptedBindingsAreDistinct: bool =
    not(caseValid(chosen)) or ownerComparisonsDistinct(
      chosen.bindings,
      pair => suppliedComparison(chosen.comparisonMode, pair),
    )

  val PermutationInvariant: bool =
    caseValid(permutation.first) == caseValid(permutation.second)

  val LookupIsUnique: bool =
    not(caseValid(permutation.first)) or not(caseValid(permutation.second)) or all {
      assignmentForSlot(permutation.first.snapshot, permutation.slot).length() == 1,
      assignmentForSlot(permutation.second.snapshot, permutation.slot).length() == 1,
      assignmentForPosition(permutation.first.snapshot, permutation.position).length() == 1,
      assignmentForPosition(permutation.second.snapshot, permutation.position).length() == 1,
      bindingForSlot(permutation.first.bindings, permutation.slot).length() == 1,
      bindingForSlot(permutation.second.bindings, permutation.slot).length() == 1,
      assignmentForSlot(permutation.first.snapshot, permutation.slot)
        == assignmentForSlot(permutation.second.snapshot, permutation.slot),
      assignmentForPosition(permutation.first.snapshot, permutation.position)
        == assignmentForPosition(permutation.second.snapshot, permutation.position),
      bindingForSlot(permutation.first.bindings, permutation.slot)
        == bindingForSlot(permutation.second.bindings, permutation.slot),
    }

  val ValidInputReachable: bool = caseValid(chosen)
  val InvalidInputReachable: bool = not(caseValid(chosen))
  val SnapshotCoverageRejectionReachable: bool = not(snapshotCoverage(chosen.snapshot))
  val SnapshotUniquenessRejectionReachable: bool = not(snapshotUniqueness(chosen.snapshot))
  val BindingBijectionRejectionReachable: bool = not(bindingBijection(chosen.snapshot, chosen.bindings))
  val CapturedFieldRejectionReachable: bool = not(capturedFieldsMatch(chosen.snapshot, chosen.bindings))
  val OpenedStoreRejectionReachable: bool = not(openedStoresMatch(chosen.snapshot, chosen.bindings))
  val OwnerComparisonRejectionReachable: bool = not(ownerComparisonsDistinct(
    chosen.bindings,
    pair => suppliedComparison(chosen.comparisonMode, pair),
  ))
  val OneDataOneParityAcceptedReachable: bool = chosen.snapshot == Snapshot11 and caseValid(chosen)
  val TwoDataOneParityAcceptedReachable: bool = chosen.snapshot == Snapshot21 and caseValid(chosen)
  val MultiParityAcceptedReachable: bool = chosen.snapshot == Snapshot12 and caseValid(chosen)
  val SecondGeometryAcceptedReachable: bool = chosen.snapshot == Snapshot21GeometryB and caseValid(chosen)
  val SameRejectedReachable: bool = chosen.comparisonMode == FirstSame and not(caseValid(chosen))
  val AmbiguousRejectedReachable: bool = chosen.comparisonMode == FirstAmbiguous and not(caseValid(chosen))
  val LaterPairRejectedReachable: bool = (
    chosen.comparisonMode == LastSame or chosen.comparisonMode == LastAmbiguous
  ) and not(caseValid(chosen))

  run focusedBoundsTest = all {
    FocusedCases.size() == 15,
    PermutationCases.size() == 5,
    caseValid({ snapshot: Snapshot11, bindings: Bindings11, comparisonMode: AllDistinct }),
    caseValid({ snapshot: Snapshot21, bindings: Bindings21, comparisonMode: AllDistinct }),
    caseValid({ snapshot: Snapshot12, bindings: Bindings12, comparisonMode: AllDistinct }),
  }
}
