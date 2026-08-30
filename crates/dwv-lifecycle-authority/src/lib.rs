//! Process-bound lifecycle signing primitives.
//!
//! An owner and verifier share one unforgeable runtime domain. These types
//! carry domain and operation identity only. They are not the healthy
//! service's canonical release or Included capabilities: `dwv-service`
//! performs the owner-fact composition and wraps the resulting primitive in
//! service-owned opaque types before coded consumers can accept it.
//!
//! The former process-global positive issuers are absent:
//!
//! ```compile_fail
//! use dwv_lifecycle_authority::owner;
//! use dwv_store::OperationSlotToken;
//!
//! let _ = owner::release(OperationSlotToken::new(0, 1));
//! ```
//!
//! ```compile_fail
//! use dwv_lifecycle_authority::owner;
//! use dwv_store::OperationSlotToken;
//!
//! let _ = owner::included_live(OperationSlotToken::new(0, 1));
//! ```
//!
//! ```compile_fail
//! use dwv_lifecycle_authority::owner;
//! use dwv_store::OperationSlotToken;
//!
//! let _ = owner::included_released(OperationSlotToken::new(0, 1));
//! ```

use dwv_store::{
    DrainState, OperationSlotTable, OperationSlotToken, ReconciliationOutcome, SlotState,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LifecycleAuthorityOwner {
    domain: Arc<()>,
}

#[derive(Clone, Debug)]
pub struct LifecycleAuthorityVerifier {
    domain: Arc<()>,
}

impl LifecycleAuthorityOwner {
    /// Create one isolated owner/verifier domain.
    pub fn new() -> (Self, LifecycleAuthorityVerifier) {
        let domain = Arc::new(());
        (
            Self {
                domain: Arc::clone(&domain),
            },
            LifecycleAuthorityVerifier { domain },
        )
    }

    /// Sign the service's seven release observations for one exact operation.
    pub fn authorize_release(
        &self,
        operations: &OperationSlotTable,
        operation: OperationSlotToken,
        operation_effect_resolved: bool,
        transaction_release_satisfied: bool,
        recovery_authoritative: bool,
        basis_conformant: bool,
    ) -> Option<ReleaseAuthorization> {
        let snapshot = operations.snapshot(operation).ok()?;
        (snapshot.token == operation
            && operation_effect_resolved
            && snapshot.children.iter().all(|child| child.terminal)
            && snapshot.reconciliation == Some(ReconciliationOutcome::Durable)
            && snapshot.state == SlotState::Reclaimable
            && snapshot.drain_state != DrainState::Required
            && transaction_release_satisfied
            && recovery_authoritative
            && basis_conformant)
            .then(|| ReleaseAuthorization {
                domain: Arc::clone(&self.domain),
                operation,
            })
    }

    /// Sign the service's exact live finalization observation.
    pub fn authorize_included_live(
        &self,
        operations: &OperationSlotTable,
        operation: OperationSlotToken,
        finalization_complete: bool,
    ) -> Option<IncludedLifecycleAuthorization> {
        let snapshot = operations.snapshot(operation).ok()?;
        (snapshot.token == operation
            && finalization_complete
            && !snapshot.children.is_empty()
            && snapshot.children.iter().all(|child| child.terminal))
        .then(|| IncludedLifecycleAuthorization {
            domain: Arc::clone(&self.domain),
            operation,
            released: false,
        })
    }
    /// Sign live Included disposition after service release evaluation.
    pub fn authorize_included_live_from_release(
        &self,
        release: &ReleaseAuthorization,
    ) -> Option<IncludedLifecycleAuthorization> {
        Arc::ptr_eq(&self.domain, &release.domain).then(|| IncludedLifecycleAuthorization {
            domain: Arc::clone(&self.domain),
            operation: release.operation,
            released: false,
        })
    }

    /// Sign released Included disposition after service receipt validation.
    pub fn authorize_included_released(
        &self,
        release: &ReleaseAuthorization,
    ) -> Option<IncludedLifecycleAuthorization> {
        Arc::ptr_eq(&self.domain, &release.domain).then(|| IncludedLifecycleAuthorization {
            domain: Arc::clone(&self.domain),
            operation: release.operation,
            released: true,
        })
    }
}

impl LifecycleAuthorityVerifier {
    pub fn accepts_release(&self, authorization: &ReleaseAuthorization) -> bool {
        Arc::ptr_eq(&self.domain, &authorization.domain)
    }

    pub fn accepts_included(&self, authorization: &IncludedLifecycleAuthorization) -> bool {
        Arc::ptr_eq(&self.domain, &authorization.domain)
    }
}

/// Exact-generation lower primitive wrapped by the healthy service.
#[derive(Clone, Debug)]
pub struct ReleaseAuthorization {
    domain: Arc<()>,
    operation: OperationSlotToken,
}

impl ReleaseAuthorization {
    pub const fn operation(&self) -> OperationSlotToken {
        self.operation
    }
}

impl PartialEq for ReleaseAuthorization {
    fn eq(&self, other: &Self) -> bool {
        self.operation == other.operation && Arc::ptr_eq(&self.domain, &other.domain)
    }
}

impl Eq for ReleaseAuthorization {}

/// Exact-generation lower Included primitive wrapped by the healthy service.
#[derive(Clone, Debug)]
pub struct IncludedLifecycleAuthorization {
    domain: Arc<()>,
    operation: OperationSlotToken,
    released: bool,
}

impl IncludedLifecycleAuthorization {
    pub const fn operation(&self) -> OperationSlotToken {
        self.operation
    }

    pub const fn is_released(&self) -> bool {
        self.released
    }
}

impl PartialEq for IncludedLifecycleAuthorization {
    fn eq(&self, other: &Self) -> bool {
        self.operation == other.operation
            && self.released == other.released
            && Arc::ptr_eq(&self.domain, &other.domain)
    }
}

impl Eq for IncludedLifecycleAuthorization {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_rejects_capabilities_from_another_owner_domain() {
        let operation = OperationSlotToken::new(3, 7);
        let (owner, verifier) = LifecycleAuthorityOwner::new();
        let (foreign_owner, _) = LifecycleAuthorityOwner::new();
        let release = ReleaseAuthorization {
            domain: Arc::clone(&owner.domain),
            operation,
        };
        let foreign_release = ReleaseAuthorization {
            domain: Arc::clone(&foreign_owner.domain),
            operation,
        };

        assert!(verifier.accepts_release(&release));
        assert!(!verifier.accepts_release(&foreign_release));
        assert!(
            owner
                .authorize_included_released(&release)
                .is_some_and(|authorization| verifier.accepts_included(&authorization))
        );
        assert!(
            owner
                .authorize_included_released(&foreign_release)
                .is_none()
        );
    }
}
