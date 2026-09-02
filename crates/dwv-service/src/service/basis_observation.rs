use super::{BasisConformance, OperationSlotToken};

pub(super) struct BasisObservationLedger {
    entries: Vec<Option<(OperationSlotToken, BasisConformance)>>,
}

impl BasisObservationLedger {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            entries: vec![None; capacity],
        }
    }

    pub(super) fn observe(&self, operation: OperationSlotToken) -> BasisConformance {
        let Ok(index) = usize::try_from(operation.index) else {
            return BasisConformance::Unresolved;
        };
        self.entries
            .get(index)
            .and_then(|observation| *observation)
            .filter(|(token, _)| *token == operation)
            .map_or(BasisConformance::Unresolved, |(_, observation)| observation)
    }

    pub(super) fn record(&mut self, operation: OperationSlotToken, observation: BasisConformance) {
        if !observation.is_conformant() {
            return;
        }
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.entries.get_mut(index) {
            *slot = Some((operation, observation));
        }
    }

    pub(super) fn clear(&mut self, operation: OperationSlotToken) {
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.entries.get_mut(index)
            && slot.is_some_and(|(token, _)| token == operation)
        {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basis_observation_is_conformant_and_generation_bound() {
        let mut ledger = BasisObservationLedger::new(1);
        let current = OperationSlotToken::new(0, 2);
        let stale = OperationSlotToken::new(0, 1);

        ledger.record(current, BasisConformance::Unresolved);
        assert_eq!(ledger.observe(current), BasisConformance::Unresolved);

        ledger.record(current, BasisConformance::Consumed);
        assert_eq!(ledger.observe(current), BasisConformance::Consumed);
        ledger.record(current, BasisConformance::Unresolved);
        assert_eq!(ledger.observe(current), BasisConformance::Consumed);
        assert_eq!(ledger.observe(stale), BasisConformance::Unresolved);

        ledger.clear(stale);
        assert_eq!(ledger.observe(current), BasisConformance::Consumed);

        ledger.clear(current);
        assert_eq!(ledger.observe(current), BasisConformance::Unresolved);
    }
}
