---- MODULE RecoveryProtocol ----
EXTENDS Naturals

CONSTANT MaxDepth

(* --algorithm RecoveryProtocol
variables
  intent = "none",
  home = "unmodified",
  recovery = "clean",
  obligation = "none",
  invalidated = FALSE,
  mutated = {},
  fenced = {},
  checkpointed = {},
  steps = 0;

define
  WithinBudget == steps < MaxDepth
  Regions == {"data", "parity"}
  Stores == {"data", "parity"}
end define;

begin
  Loop:
  while TRUE do
    either
      when WithinBudget /\ (obligation = "none" \/ obligation = "terminal");
      intent := "pending";
      home := "unmodified";
      recovery := "dirty";
      obligation := "inflight";
      invalidated := FALSE;
      mutated := {};
      fenced := {};
      checkpointed := {};
      steps := steps + 1;
    or
      when WithinBudget /\ intent = "pending";
      intent := "durable";
      invalidated := TRUE;
      steps := steps + 1;
    or
      when WithinBudget /\ intent = "durable" /\ mutated # Regions;
      with region \in Regions \ mutated do
        mutated := mutated \cup {region};
        home := "volatile";
      end with;
      steps := steps + 1;
    or
      when WithinBudget /\ home = "volatile" /\ mutated = Regions;
      home := "durable";
      steps := steps + 1;
    or
      when WithinBudget /\ home = "durable" /\ fenced # Stores;
      with store \in Stores \ fenced do
        fenced := fenced \cup {store};
      end with;
      steps := steps + 1;
    or
      when WithinBudget /\ recovery = "dirty" /\ home = "durable"
        /\ intent = "durable" /\ mutated = Regions /\ fenced = Stores;
      recovery := "clean";
      checkpointed := Regions;
      obligation := "terminal";
      steps := steps + 1;
    or
      when WithinBudget /\ home \in {"volatile", "durable"} /\ obligation = "inflight";
      home := "unknown";
      recovery := "indeterminate";
      steps := steps + 1;
    or
      when WithinBudget /\ home = "unknown";
      home := "durable";
      recovery := "dirty";
      obligation := "handoff";
      steps := steps + 1;
    or
      when WithinBudget /\ obligation \in {"inflight", "handoff"}
        /\ intent = "pending" /\ home = "unmodified";
      intent := "none";
      recovery := "clean";
      obligation := "none";
      invalidated := FALSE;
      mutated := {};
      fenced := {};
      checkpointed := {};
      steps := steps + 1;
    or
      when WithinBudget /\ obligation \in {"inflight", "handoff"}
        /\ ~(intent = "pending" /\ home = "unmodified");
      recovery := "dirty";
      obligation := "handoff";
      home := IF home = "volatile" THEN "unknown" ELSE home;
      steps := steps + 1;
    end either;
  end while;
end algorithm; *)
\* BEGIN TRANSLATION (chksum(pcal) = "a55078da" /\ chksum(tla) = "ca0a87a3")
VARIABLES intent, home, recovery, obligation, invalidated, mutated, fenced, 
          checkpointed, steps

(* define statement *)
WithinBudget == steps < MaxDepth
Regions == {"data", "parity"}
Stores == {"data", "parity"}


vars == << intent, home, recovery, obligation, invalidated, mutated, fenced, 
           checkpointed, steps >>

Init == (* Global variables *)
        /\ intent = "none"
        /\ home = "unmodified"
        /\ recovery = "clean"
        /\ obligation = "none"
        /\ invalidated = FALSE
        /\ mutated = {}
        /\ fenced = {}
        /\ checkpointed = {}
        /\ steps = 0

Next == \/ /\ WithinBudget /\ (obligation = "none" \/ obligation = "terminal")
           /\ intent' = "pending"
           /\ home' = "unmodified"
           /\ recovery' = "dirty"
           /\ obligation' = "inflight"
           /\ invalidated' = FALSE
           /\ mutated' = {}
           /\ fenced' = {}
           /\ checkpointed' = {}
           /\ steps' = steps + 1
        \/ /\ WithinBudget /\ intent = "pending"
           /\ intent' = "durable"
           /\ invalidated' = TRUE
           /\ steps' = steps + 1
           /\ UNCHANGED <<home, recovery, obligation, mutated, fenced, checkpointed>>
        \/ /\ WithinBudget /\ intent = "durable" /\ mutated # Regions
           /\ \E region \in Regions \ mutated:
                /\ mutated' = (mutated \cup {region})
                /\ home' = "volatile"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, recovery, obligation, invalidated, fenced, checkpointed>>
        \/ /\ WithinBudget /\ home = "volatile" /\ mutated = Regions
           /\ home' = "durable"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, recovery, obligation, invalidated, mutated, fenced, checkpointed>>
        \/ /\ WithinBudget /\ home = "durable" /\ fenced # Stores
           /\ \E store \in Stores \ fenced:
                fenced' = (fenced \cup {store})
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, home, recovery, obligation, invalidated, mutated, checkpointed>>
        \/ /\    WithinBudget /\ recovery = "dirty" /\ home = "durable"
              /\ intent = "durable" /\ mutated = Regions /\ fenced = Stores
           /\ recovery' = "clean"
           /\ checkpointed' = Regions
           /\ obligation' = "terminal"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, home, invalidated, mutated, fenced>>
        \/ /\ WithinBudget /\ home \in {"volatile", "durable"} /\ obligation = "inflight"
           /\ home' = "unknown"
           /\ recovery' = "indeterminate"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, obligation, invalidated, mutated, fenced, checkpointed>>
        \/ /\ WithinBudget /\ home = "unknown"
           /\ home' = "durable"
           /\ recovery' = "dirty"
           /\ obligation' = "handoff"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, invalidated, mutated, fenced, checkpointed>>
        \/ /\    WithinBudget /\ obligation \in {"inflight", "handoff"}
              /\ intent = "pending" /\ home = "unmodified"
           /\ intent' = "none"
           /\ recovery' = "clean"
           /\ obligation' = "none"
           /\ invalidated' = FALSE
           /\ mutated' = {}
           /\ fenced' = {}
           /\ checkpointed' = {}
           /\ steps' = steps + 1
           /\ home' = home
        \/ /\    WithinBudget /\ obligation \in {"inflight", "handoff"}
              /\ ~(intent = "pending" /\ home = "unmodified")
           /\ recovery' = "dirty"
           /\ obligation' = "handoff"
           /\ home' = (IF home = "volatile" THEN "unknown" ELSE home)
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, invalidated, mutated, fenced, checkpointed>>

Spec == Init /\ [][Next]_vars

\* END TRANSLATION 

IntentStates == {"none", "pending", "durable"}
HomeStates == {"unmodified", "volatile", "durable", "unknown"}
RecoveryStates == {"clean", "dirty", "indeterminate"}
ObligationStates == {"none", "inflight", "handoff", "terminal"}

TypeInvariant ==
  /\ intent \in IntentStates
  /\ home \in HomeStates
  /\ recovery \in RecoveryStates
  /\ obligation \in ObligationStates
  /\ invalidated \in BOOLEAN
  /\ mutated \subseteq Regions
  /\ fenced \subseteq Stores
  /\ checkpointed \subseteq Regions
  /\ steps \in Nat
  /\ steps <= MaxDepth
NoFalseClean ==
  recovery = "clean"
    => /\ obligation \in {"none", "terminal"}
       /\ (intent = "none" \/ (intent = "durable" /\ invalidated /\ home = "durable"))
       /\ (obligation = "terminal" => checkpointed = Regions)

MutationRequiresIntent ==
  mutated # {} => intent = "durable"

UncertaintyIsVisible ==
  (home = "unknown" \/ recovery = "indeterminate")
    => /\ recovery # "clean"
       /\ obligation # "terminal"

DurableWorkIsOwned ==
  (intent = "durable" \/ home # "unmodified") => obligation # "none"

TerminalRequiresEvidence ==
  obligation = "terminal"
    => /\ recovery = "clean"
       /\ intent = "durable"
       /\ invalidated
       /\ home = "durable"
       /\ mutated = Regions
       /\ fenced = Stores
       /\ checkpointed = Regions

FenceAndCheckpointCoverage ==
  /\ fenced \subseteq Stores
  /\ checkpointed \subseteq mutated
  /\ checkpointed # {} => fenced = Stores

====
