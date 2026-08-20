/**
 * Verification-only profile: exactly three operation identities and two coded
 * units. A={u0} and B={u1} coexist while bridge={u0,u1} waits for both.
 */
module CodedRangeCleanConflictExhaustive {
  import CodedRangeClean(
    Operations = Set("opA", "opBridge", "opB"),
    Captures = Set("capture0"),
    CodedUnits = Set("u0", "u1")
  ).* from "../../models/quint/CodedRangeClean"

  val ClaimU0: CodedClaim = {
    units: Set("u0"),
    complete: true,
    validated: true,
  }

  val ClaimU1: CodedClaim = {
    units: Set("u1"),
    complete: true,
    validated: true,
  }

  val ClaimU0U1: CodedClaim = {
    units: Set("u0", "u1"),
    complete: true,
    validated: true,
  }

  val ProfileInvariants: bool =
    TypeInvariant
    and NoHeldCodedOverlap
    and CompleteAdmissionPrecedesEffect
    and CaptureMembershipExhaustive
    and CaptureMembershipExclusive
    and LaterEffectRequiresDurableCut

  action step: bool = any {
    admitComplete("opA", ClaimU0),
    admitComplete("opBridge", ClaimU0U1),
    admitComplete("opB", ClaimU1),
    permitMutationEffect("opA"),
    permitMutationEffect("opBridge"),
    permitMutationEffect("opB"),
    removeCodedClaim("opA", ReleaseAllowed),
    removeCodedClaim("opBridge", ReleaseAllowed),
    removeCodedClaim("opB", ReleaseAllowed),
  }

  val nonTransitiveBridgeReachable: bool =
    state.operations.get("opA").phase == Released
    and state.operations.get("opB").phase == Released
    and state.operations.get("opBridge").phase == Held

  run bridgeRefusedWhileBothConflictsHeld =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(admitComplete("opB", ClaimU1))
      .expect(
        state.operations.get("opA").phase == Held
        and state.operations.get("opB").phase == Held
      )
      .then(admitComplete("opBridge", ClaimU0U1).fail())

  run bridgeRefusedUntilUnit0ConflictReleases =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(admitComplete("opB", ClaimU1))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(admitComplete("opBridge", ClaimU0U1).fail())

  run nonTransitiveBridgeWaitsForBothConflicts =
    init
      .then(admitComplete("opA", ClaimU0))
      .then(admitComplete("opB", ClaimU1))
      .then(removeCodedClaim("opA", ReleaseAllowed))
      .then(removeCodedClaim("opB", ReleaseAllowed))
      .then(admitComplete("opBridge", ClaimU0U1))
      .expect(nonTransitiveBridgeReachable)
}
