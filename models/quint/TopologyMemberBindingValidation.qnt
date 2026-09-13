/**
 * Pure topology member-binding validity over owner-accepted inputs.
 *
 * Profile and geometry construction, deserialization validity, array comparison,
 * assignment-to-store authorization, identity assessment, service policy,
 * transitions, recovery, durability, and physical I/O remain outside this model.
 * Pairwise comparison is an owner-supplied function, not an encoded table.
 */
// dwv:req req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments
module TopologyMemberBindingValidation {
  type MemberRole = Data | Parity
  type IdentityComparison = Distinct | Same | Ambiguous

  type Profile = {
    dataSlots: int,
    paritySlots: int,
  }

  type Geometry = {
    protectedLength: int,
    parityLength: int,
    logicalBlockSize: int,
  }

  type Assignment = {
    slot: int,
    role: MemberRole,
    position: int,
    instance: int,
    generation: int,
  }

  type Snapshot = {
    array: int,
    epoch: int,
    profile: Profile,
    geometry: Geometry,
    assignments: List[Assignment],
  }

  type Binding = {
    slot: int,
    role: MemberRole,
    position: int,
    instance: int,
    generation: int,
    epoch: int,
    store: int,
    openedStore: int,
    openedEpoch: int,
    openedProtectedLength: int,
    openedLogicalBlockSize: int,
  }

  pure def profileTotal(profile: Profile): int =
    profile.dataSlots + profile.paritySlots

  pure def profileAccepts(profile: Profile, role: MemberRole, position: int): bool =
    match role {
      | Data => position >= 0 and position < profile.dataSlots
      | Parity => position >= profile.dataSlots and position < profileTotal(profile)
    }


  pure def assignmentForSlot(snapshot: Snapshot, slot: int): List[Assignment] =
    snapshot.assignments.select(assignment => assignment.slot == slot)

  pure def assignmentForPosition(snapshot: Snapshot, position: int): List[Assignment] =
    snapshot.assignments.select(assignment => assignment.position == position)

  pure def bindingForSlot(bindings: List[Binding], slot: int): List[Binding] =
    bindings.select(binding => binding.slot == slot)

  pure def snapshotCoverage(snapshot: Snapshot): bool = all {
    snapshot.assignments.length() == profileTotal(snapshot.profile),
    snapshot.assignments.foldl(true, (accepted, assignment) =>
      accepted and profileAccepts(snapshot.profile, assignment.role, assignment.position)
    ),
  }

  pure def snapshotUniqueness(snapshot: Snapshot): bool = all {
    snapshot.assignments.foldl(Set(), (seen, assignment) =>
      seen.union(Set(assignment.slot))
    ).size() == snapshot.assignments.length(),
    snapshot.assignments.foldl(Set(), (seen, assignment) =>
      seen.union(Set(assignment.position))
    ).size() == snapshot.assignments.length(),
    snapshot.assignments.foldl(Set(), (seen, assignment) =>
      seen.union(Set(assignment.instance))
    ).size() == snapshot.assignments.length(),
  }

  pure def bindingBijection(snapshot: Snapshot, bindings: List[Binding]): bool = all {
    bindings.length() == snapshot.assignments.length(),
    bindings.foldl(true, (bijective, binding) =>
      bijective and assignmentForSlot(snapshot, binding.slot).length() == 1
    ),
    snapshot.assignments.foldl(true, (bijective, assignment) =>
      bijective and bindingForSlot(bindings, assignment.slot).length() == 1
    ),
  }

  pure def capturedFieldsMatch(snapshot: Snapshot, bindings: List[Binding]): bool =
    bindings.foldl(true, (matches, binding) =>
      matches and snapshot.assignments.foldl(false, (found, assignment) =>
        found or all {
          assignment.slot == binding.slot,
          assignment.role == binding.role,
          assignment.position == binding.position,
          assignment.instance == binding.instance,
          assignment.generation == binding.generation,
          binding.epoch == snapshot.epoch,
        }
      )
    )

  pure def openedStoresMatch(snapshot: Snapshot, bindings: List[Binding]): bool = all {
    bindings.foldl(Set(), (seen, binding) =>
      seen.union(Set(binding.store))
    ).size() == bindings.length(),
    bindings.foldl(true, (matches, binding) =>
      matches and all {
        binding.store == binding.openedStore,
        binding.openedEpoch == binding.epoch,
        binding.openedEpoch == snapshot.epoch,
        binding.openedProtectedLength == snapshot.geometry.protectedLength,
        binding.openedLogicalBlockSize == snapshot.geometry.logicalBlockSize,
      }
    ),
  }

  pure def ownerComparisonsDistinct(
    bindings: List[Binding],
    comparison: (Set[int]) => IdentityComparison,
  ): bool =
    bindings.foldl(true, (outerDistinct, left) =>
      outerDistinct and bindings.foldl(true, (innerDistinct, right) =>
        innerDistinct and (
          left.store == right.store or comparison(Set(left.store, right.store)) == Distinct
        )
      )
    )

  pure def snapshotValid(snapshot: Snapshot): bool =
    snapshotCoverage(snapshot) and snapshotUniqueness(snapshot)

  pure def bindingsValid(
    snapshot: Snapshot,
    bindings: List[Binding],
    comparison: (Set[int]) => IdentityComparison,
  ): bool = all {
    bindingBijection(snapshot, bindings),
    capturedFieldsMatch(snapshot, bindings),
    openedStoresMatch(snapshot, bindings),
    ownerComparisonsDistinct(bindings, comparison),
  }

  pure def valid(
    snapshot: Snapshot,
    bindings: List[Binding],
    comparison: (Set[int]) => IdentityComparison,
  ): bool =
    snapshotValid(snapshot) and bindingsValid(snapshot, bindings, comparison)
}
