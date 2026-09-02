use super::{OperationReadiness, OperationSlotToken};

pub(super) struct OperationReadinessLedger {
    entries: Vec<Option<(OperationSlotToken, OperationReadiness)>>,
}

impl OperationReadinessLedger {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            entries: vec![None; capacity],
        }
    }

    pub(super) fn set(&mut self, operation: OperationSlotToken, readiness: OperationReadiness) {
        if let Ok(index) = usize::try_from(operation.index)
            && let Some(entry) = self.entries.get_mut(index)
        {
            *entry = Some((operation, readiness));
        }
    }

    #[cfg(test)]
    pub(super) fn get(&self, operation: OperationSlotToken) -> Option<OperationReadiness> {
        usize::try_from(operation.index)
            .ok()
            .and_then(|index| self.entries.get(index).copied())
            .and_then(|entry| {
                let (token, readiness) = entry?;
                (token == operation).then_some(readiness)
            })
    }

    pub(super) fn clear(&mut self, operation: OperationSlotToken) {
        if let Ok(index) = usize::try_from(operation.index)
            && let Some(entry) = self.entries.get_mut(index)
            && entry.is_some_and(|(token, _)| token == operation)
        {
            *entry = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generation_cannot_clear_current_readiness() {
        let mut ledger = OperationReadinessLedger::new(1);
        let current = OperationSlotToken::new(0, 2);
        let stale = OperationSlotToken::new(0, 1);

        ledger.set(current, OperationReadiness::Runnable);
        ledger.clear(stale);
        assert_eq!(ledger.get(current), Some(OperationReadiness::Runnable));

        ledger.clear(current);
        assert_eq!(ledger.get(current), None);
    }
}
