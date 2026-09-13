/**
 * Widened sampled profile: five assignments/bindings, slots {10..14},
 * instances {20..24}, stores {30..34}, generation 7, epoch 9,
 * owner-accepted profile (3,2), and geometry (16384,32768,4096).
 * Sampling this finite case set does not weaken or enlarge the focused claim.
 */
module TopologyMemberBindingValidationWideAnalysis {
  import TopologyMemberBindingValidation.* from "../../models/quint/TopologyMemberBindingValidation"

  pure val WideGeometry: Geometry = {
    protectedLength: 16384,
    parityLength: 32768,
    logicalBlockSize: 4096,
  }

  pure val WideAssignments: List[Assignment] = List(
    { slot: 10, role: Data, position: 0, instance: 20, generation: 7 },
    { slot: 11, role: Data, position: 1, instance: 21, generation: 7 },
    { slot: 12, role: Data, position: 2, instance: 22, generation: 7 },
    { slot: 13, role: Parity, position: 3, instance: 23, generation: 7 },
    { slot: 14, role: Parity, position: 4, instance: 24, generation: 7 }
  )

  pure val WideSnapshot: Snapshot = {
    array: 19,
    epoch: 9,
    profile: { dataSlots: 3, paritySlots: 2 },
    geometry: WideGeometry,
    assignments: WideAssignments,
  }

  pure val WideBindings: List[Binding] = List(
    { slot: 10, role: Data, position: 0, instance: 20, generation: 7, epoch: 9,
      store: 30, openedStore: 30, openedEpoch: 9, openedProtectedLength: 16384, openedLogicalBlockSize: 4096 },
    { slot: 11, role: Data, position: 1, instance: 21, generation: 7, epoch: 9,
      store: 31, openedStore: 31, openedEpoch: 9, openedProtectedLength: 16384, openedLogicalBlockSize: 4096 },
    { slot: 12, role: Data, position: 2, instance: 22, generation: 7, epoch: 9,
      store: 32, openedStore: 32, openedEpoch: 9, openedProtectedLength: 16384, openedLogicalBlockSize: 4096 },
    { slot: 13, role: Parity, position: 3, instance: 23, generation: 7, epoch: 9,
      store: 33, openedStore: 33, openedEpoch: 9, openedProtectedLength: 16384, openedLogicalBlockSize: 4096 },
    { slot: 14, role: Parity, position: 4, instance: 24, generation: 7, epoch: 9,
      store: 34, openedStore: 34, openedEpoch: 9, openedProtectedLength: 16384, openedLogicalBlockSize: 4096 }
  )

  pure val WideReorderedSnapshot: Snapshot = {
    ...WideSnapshot,
    assignments: List(
      WideAssignments.nth(4),
      WideAssignments.nth(2),
      WideAssignments.nth(0),
      WideAssignments.nth(3),
      WideAssignments.nth(1)
    ),
  }

  pure val WideReorderedBindings: List[Binding] = List(
    WideBindings.nth(1),
    WideBindings.nth(4),
    WideBindings.nth(3),
    WideBindings.nth(0),
    WideBindings.nth(2)
  )

  pure def wideDistinct(pair: Set[int]): IdentityComparison = Distinct

  pure def wideSame(pair: Set[int]): IdentityComparison =
    if (pair == Set(30, 31)) Same else Distinct

  type WideCase = {
    snapshot: Snapshot,
    bindings: List[Binding],
    expectedValid: bool,
    samePair: bool,
  }

  pure val WideCases: Set[WideCase] = Set(
    { snapshot: WideSnapshot, bindings: WideBindings, expectedValid: true, samePair: false },
    { snapshot: WideReorderedSnapshot, bindings: WideReorderedBindings, expectedValid: true, samePair: false },
    { snapshot: WideSnapshot, bindings: WideBindings, expectedValid: false, samePair: true },
    {
      snapshot: WideSnapshot,
      bindings: WideBindings.replaceAt(0, {...WideBindings.nth(0), openedLogicalBlockSize: 512}),
      expectedValid: false,
      samePair: false,
    },
    {
      snapshot: WideSnapshot,
      bindings: WideBindings.replaceAt(4, {...WideBindings.nth(4), generation: 8}),
      expectedValid: false,
      samePair: false,
    }
  )

  var chosen: WideCase

  action init: bool = {
    nondet candidate = WideCases.oneOf()
    chosen' = candidate
  }

  action step: bool = chosen' = chosen

  val WideExpectedResult: bool =
    valid(
      chosen.snapshot,
      chosen.bindings,
      pair => if (chosen.samePair) wideSame(pair) else wideDistinct(pair),
    ) == chosen.expectedValid

  val WidePermutationInvariant: bool = all {
    valid(WideSnapshot, WideBindings, wideDistinct),
    valid(WideReorderedSnapshot, WideReorderedBindings, wideDistinct),
    assignmentForSlot(WideSnapshot, 12) == assignmentForSlot(WideReorderedSnapshot, 12),
    assignmentForPosition(WideSnapshot, 2) == assignmentForPosition(WideReorderedSnapshot, 2),
    bindingForSlot(WideBindings, 12) == bindingForSlot(WideReorderedBindings, 12),
  }

  val WideValidReachable: bool = chosen.expectedValid and WideExpectedResult
  val WideInvalidReachable: bool = not(chosen.expectedValid) and WideExpectedResult

  run widenedBoundsTest = all {
    WideAssignments.length() == 5,
    WideBindings.length() == 5,
    WideCases.size() == 5,
    valid(WideSnapshot, WideBindings, wideDistinct),
  }
}
