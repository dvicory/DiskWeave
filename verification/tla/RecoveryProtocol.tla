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
  steps = 0;

define
  WithinBudget == steps < MaxDepth
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
      steps := steps + 1;
    or
      when WithinBudget /\ intent = "pending";
      intent := "durable";
      invalidated := TRUE;
      steps := steps + 1;
    or
      when WithinBudget /\ intent = "durable" /\ home = "unmodified";
      home := "volatile";
      steps := steps + 1;
    or
      when WithinBudget /\ home = "volatile";
      home := "durable";
      steps := steps + 1;
    or
      when WithinBudget /\ recovery = "dirty" /\ home = "durable" /\ intent = "durable";
      recovery := "clean";
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
\* BEGIN TRANSLATION (chksum(pcal) = "609a86fc" /\ chksum(tla) = "461d2b64")
VARIABLES intent, home, recovery, obligation, invalidated, steps

(* define statement *)
WithinBudget == steps < MaxDepth


vars == << intent, home, recovery, obligation, invalidated, steps >>

Init == (* Global variables *)
        /\ intent = "none"
        /\ home = "unmodified"
        /\ recovery = "clean"
        /\ obligation = "none"
        /\ invalidated = FALSE
        /\ steps = 0

Next == \/ /\ WithinBudget /\ (obligation = "none" \/ obligation = "terminal")
           /\ intent' = "pending"
           /\ home' = "unmodified"
           /\ recovery' = "dirty"
           /\ obligation' = "inflight"
           /\ invalidated' = FALSE
           /\ steps' = steps + 1
        \/ /\ WithinBudget /\ intent = "pending"
           /\ intent' = "durable"
           /\ invalidated' = TRUE
           /\ steps' = steps + 1
           /\ UNCHANGED <<home, recovery, obligation>>
        \/ /\ WithinBudget /\ intent = "durable" /\ home = "unmodified"
           /\ home' = "volatile"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, recovery, obligation, invalidated>>
        \/ /\ WithinBudget /\ home = "volatile"
           /\ home' = "durable"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, recovery, obligation, invalidated>>
        \/ /\ WithinBudget /\ recovery = "dirty" /\ home = "durable" /\ intent = "durable"
           /\ recovery' = "clean"
           /\ obligation' = "terminal"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, home, invalidated>>
        \/ /\ WithinBudget /\ home \in {"volatile", "durable"} /\ obligation = "inflight"
           /\ home' = "unknown"
           /\ recovery' = "indeterminate"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, obligation, invalidated>>
        \/ /\ WithinBudget /\ home = "unknown"
           /\ home' = "durable"
           /\ recovery' = "dirty"
           /\ obligation' = "handoff"
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, invalidated>>
        \/ /\    WithinBudget /\ obligation \in {"inflight", "handoff"}
              /\ intent = "pending" /\ home = "unmodified"
           /\ intent' = "none"
           /\ recovery' = "clean"
           /\ obligation' = "none"
           /\ invalidated' = FALSE
           /\ steps' = steps + 1
           /\ home' = home
        \/ /\    WithinBudget /\ obligation \in {"inflight", "handoff"}
              /\ ~(intent = "pending" /\ home = "unmodified")
           /\ recovery' = "dirty"
           /\ obligation' = "handoff"
           /\ home' = (IF home = "volatile" THEN "unknown" ELSE home)
           /\ steps' = steps + 1
           /\ UNCHANGED <<intent, invalidated>>

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
  /\ steps \in Nat
  /\ steps <= MaxDepth
NoFalseClean ==
  recovery = "clean"
    => /\ obligation \in {"none", "terminal"}
       /\ (intent = "none" \/ (intent = "durable" /\ invalidated /\ home = "durable"))

MutationRequiresIntent ==
  home # "unmodified" => intent = "durable"

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

====
