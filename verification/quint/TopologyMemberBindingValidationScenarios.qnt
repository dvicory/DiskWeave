/** Deterministic evidence for the pure topology member-binding relation. */
module TopologyMemberBindingValidationScenarios {
  import TopologyMemberBindingValidation.* from "../../models/quint/TopologyMemberBindingValidation"

  pure val GeometryA: Geometry = {
    protectedLength: 4096,
    parityLength: 4096,
    logicalBlockSize: 512,
  }

  pure def distinctComparison(pair: Set[int]): IdentityComparison = Distinct

  pure def sameFirstPair(pair: Set[int]): IdentityComparison =
    if (pair == Set(1, 2)) Same else Distinct

  pure def ambiguousFirstPair(pair: Set[int]): IdentityComparison =
    if (pair == Set(1, 2)) Ambiguous else Distinct

  pure def sameLastPair(pair: Set[int]): IdentityComparison =
    if (pair == Set(2, 3)) Same else Distinct

  pure def ambiguousLastPair(pair: Set[int]): IdentityComparison =
    if (pair == Set(2, 3)) Ambiguous else Distinct

  pure val Assignment1: Assignment = {
    slot: 1,
    role: Data,
    position: 0,
    instance: 11,
    generation: 1,
  }

  pure val Assignment2: Assignment = {
    slot: 2,
    role: Data,
    position: 1,
    instance: 12,
    generation: 1,
  }

  pure val Assignment3: Assignment = {
    slot: 3,
    role: Parity,
    position: 2,
    instance: 13,
    generation: 1,
  }

  pure val Snapshot21: Snapshot = {
    array: 7,
    epoch: 4,
    profile: { dataSlots: 2, paritySlots: 1 },
    geometry: GeometryA,
    assignments: List(Assignment1, Assignment2, Assignment3),
  }

  pure val Binding1: Binding = {
    slot: 1,
    role: Data,
    position: 0,
    instance: 11,
    generation: 1,
    epoch: 4,
    store: 1,
    openedStore: 1,
    openedEpoch: 4,
    openedProtectedLength: 4096,
    openedLogicalBlockSize: 512,
  }

  pure val Binding2: Binding = {
    slot: 2,
    role: Data,
    position: 1,
    instance: 12,
    generation: 1,
    epoch: 4,
    store: 2,
    openedStore: 2,
    openedEpoch: 4,
    openedProtectedLength: 4096,
    openedLogicalBlockSize: 512,
  }

  pure val Binding3: Binding = {
    slot: 3,
    role: Parity,
    position: 2,
    instance: 13,
    generation: 1,
    epoch: 4,
    store: 3,
    openedStore: 3,
    openedEpoch: 4,
    openedProtectedLength: 4096,
    openedLogicalBlockSize: 512,
  }

  pure val Bindings21: List[Binding] = List(Binding1, Binding2, Binding3)

  pure val Snapshot11: Snapshot = {
    array: 7,
    epoch: 4,
    profile: { dataSlots: 1, paritySlots: 1 },
    geometry: GeometryA,
    assignments: List(Assignment1, {
      ...Assignment2,
      slot: 2,
      role: Parity,
      position: 1,
      instance: 12,
    }),
  }

  pure val Bindings11: List[Binding] = List(Binding1, {
    ...Binding2,
    slot: 2,
    role: Parity,
    position: 1,
    instance: 12,
  })

  pure val Snapshot12: Snapshot = {
    array: 8,
    epoch: 5,
    profile: { dataSlots: 1, paritySlots: 2 },
    geometry: GeometryA,
    assignments: List(
      {...Assignment1, generation: 2},
      {...Assignment2, role: Parity, generation: 2},
      {...Assignment3, generation: 2}
    ),
  }

  pure val Bindings12: List[Binding] = List(
    {...Binding1, generation: 2, epoch: 5, openedEpoch: 5},
    {...Binding2, role: Parity, generation: 2, epoch: 5, openedEpoch: 5},
    {...Binding3, generation: 2, epoch: 5, openedEpoch: 5}
  )

  pure val ReorderedSnapshot21: Snapshot = {
    ...Snapshot21,
    assignments: List(Assignment3, Assignment1, Assignment2),
  }

  pure val ReorderedBindings21: List[Binding] = List(Binding2, Binding3, Binding1)

  run validOneDataOneParityTest = valid(Snapshot11, Bindings11, distinctComparison)

  run validTwoDataOneParityTest = valid(Snapshot21, Bindings21, distinctComparison)

  run structurallyValidMultiParityTest = valid(Snapshot12, Bindings12, distinctComparison)

  run matchingOpenedStoreGeometryTest = openedStoresMatch(Snapshot21, Bindings21)

  run mismatchedOpenedStoreGeometryRejectsTest = not(valid(
    Snapshot21,
    List({...Binding1, openedProtectedLength: 8192}, Binding2, Binding3),
    distinctComparison,
  ))

  run laterPairSameRejectsTest =
    not(valid(Snapshot21, Bindings21, sameLastPair))

  run laterPairAmbiguousRejectsTest =
    not(valid(Snapshot21, Bindings21, ambiguousLastPair))

  run reorderedInputsPreserveValidityAndLookupTest = all {
    valid(Snapshot21, Bindings21, distinctComparison),
    valid(ReorderedSnapshot21, ReorderedBindings21, distinctComparison),
    assignmentForSlot(Snapshot21, 2) == assignmentForSlot(ReorderedSnapshot21, 2),
    assignmentForPosition(Snapshot21, 1) == assignmentForPosition(ReorderedSnapshot21, 1),
    bindingForSlot(Bindings21, 2) == bindingForSlot(ReorderedBindings21, 2),
  }
}
