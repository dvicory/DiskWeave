use super::{OperationSlotToken, ReleaseScope};

pub(super) struct ReleaseScopeLedger {
    entries: Vec<Option<(OperationSlotToken, ReleaseScope)>>,
}

impl ReleaseScopeLedger {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            entries: vec![None; capacity],
        }
    }

    pub(super) fn observe(&self, operation: OperationSlotToken) -> ReleaseScope {
        let Ok(index) = usize::try_from(operation.index) else {
            return ReleaseScope::Unknown;
        };
        self.entries
            .get(index)
            .and_then(|scope| *scope)
            .filter(|(token, _)| *token == operation)
            .map_or(ReleaseScope::Unknown, |(_, scope)| scope)
    }

    pub(super) fn set(&mut self, operation: OperationSlotToken, scope: ReleaseScope) {
        let Ok(index) = usize::try_from(operation.index) else {
            return;
        };
        if let Some(slot) = self.entries.get_mut(index) {
            *slot = Some((operation, scope));
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
    fn stale_generation_cannot_observe_or_clear_current_scope() {
        let mut ledger = ReleaseScopeLedger::new(1);
        let current = OperationSlotToken::new(0, 2);
        let stale = OperationSlotToken::new(0, 1);

        ledger.set(current, ReleaseScope::InScope);
        assert_eq!(ledger.observe(current), ReleaseScope::InScope);
        assert_eq!(ledger.observe(stale), ReleaseScope::Unknown);

        ledger.clear(stale);
        assert_eq!(ledger.observe(current), ReleaseScope::InScope);

        ledger.clear(current);
        assert_eq!(ledger.observe(current), ReleaseScope::Unknown);
    }
}
