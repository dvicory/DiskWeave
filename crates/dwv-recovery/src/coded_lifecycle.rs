use crate::coded_authority::CodedAdmissionAuthorityId;
use crate::{
    ChecksumProfile, ChecksumSetGeneration, CodedAuthorityError, CodedCaptureCleanAuthorization,
    CodedCaptureCoordinator, CodedCaptureError, CodedCaptureId, CodedCaptureSnapshot,
    CodedClaimRelease, CodedGeometryOwner, CodedRangeAuthority, FenceCertificate,
    RecoveryCleanPermit, RecoveryGeneration, RecoverySnapshot,
};
use dwv_core::TopologySnapshot;
use dwv_lifecycle_authority::{
    IncludedLifecycleAuthorization, LifecycleAuthorityVerifier, ReleaseAuthorization,
};
use dwv_store::OperationReleasePermit;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Service-owned release authority accepted by the canonical coded consumer.
///
/// Implementations choose the exact capability type accepted by one
/// `CodedLifecycleAuthority` specialization. A different wrapper type cannot
/// be substituted even when it contains semantically similar values.
pub trait CodedReleaseAuthority {
    #[doc(hidden)]
    fn lifecycle_release(&self) -> &ReleaseAuthorization;
}

/// Service-owned Included lifecycle authority accepted by coded CLEAN.
pub trait CodedIncludedAuthority {
    #[doc(hidden)]
    fn lifecycle_included(&self) -> &IncludedLifecycleAuthorization;
}

/// Coded consumers bound to one service-selected lifecycle verifier and one
/// admission/geometry authority graph.
///
/// This type validates service-owned opaque capabilities. It does not compose
/// lifecycle facts or expose a positive lifecycle issuer.
pub struct CodedLifecycleAuthority<R, I> {
    range: CodedRangeAuthority,
    captures: CodedCaptureCoordinator,
    capability_types: PhantomData<fn(&R, &I)>,
}

impl<R, I> CodedLifecycleAuthority<R, I> {
    pub fn from_snapshots_with_verifier(
        snapshots: impl IntoIterator<Item = CodedCaptureSnapshot>,
        topology: &TopologySnapshot,
        recovery_generation: RecoveryGeneration,
        checksum_profile: ChecksumProfile,
        checksum_set_generation: ChecksumSetGeneration,
        dirty_region_bytes: u64,
        verifier: LifecycleAuthorityVerifier,
    ) -> Result<(Self, CodedGeometryOwner), CodedCaptureError> {
        let geometry =
            CodedGeometryOwner::new(topology.clone(), dirty_region_bytes, checksum_profile)?;
        let admission_authority = CodedAdmissionAuthorityId::new();
        let range = CodedRangeAuthority::new_with_authorities(
            verifier.clone(),
            admission_authority,
            geometry.authority(),
        );
        let captures = CodedCaptureCoordinator::from_snapshots_for_owner(
            snapshots,
            topology,
            recovery_generation,
            checksum_profile,
            checksum_set_generation,
            dirty_region_bytes,
            verifier,
            admission_authority,
        )?;
        Ok((
            Self {
                range,
                captures,
                capability_types: PhantomData,
            },
            geometry,
        ))
    }

    pub const fn range(&self) -> &CodedRangeAuthority {
        &self.range
    }

    pub const fn range_mut(&mut self) -> &mut CodedRangeAuthority {
        &mut self.range
    }

    pub fn prepare_release(
        &self,
        authorization: &R,
        permit: OperationReleasePermit,
    ) -> Result<(CodedRangeAuthority, CodedClaimRelease), CodedAuthorityError>
    where
        R: CodedReleaseAuthority,
    {
        let mut range = self.range.clone();
        let release = range.release(authorization.lifecycle_release(), permit)?;
        Ok((range, release))
    }

    pub fn authorize_clean(
        &self,
        recovery: &RecoverySnapshot,
        capture: CodedCaptureId,
        permit: RecoveryCleanPermit,
        included: impl IntoIterator<Item = (I, FenceCertificate)>,
    ) -> Result<CodedCaptureCleanAuthorization, CodedCaptureError>
    where
        I: CodedIncludedAuthority,
    {
        self.captures.authorize_clean(
            recovery,
            capture,
            permit,
            included.into_iter().map(|(authorization, certificate)| {
                (authorization.lifecycle_included().clone(), certificate)
            }),
        )
    }
}

impl<R, I> Deref for CodedLifecycleAuthority<R, I> {
    type Target = CodedCaptureCoordinator;

    fn deref(&self) -> &Self::Target {
        &self.captures
    }
}

impl<R, I> DerefMut for CodedLifecycleAuthority<R, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.captures
    }
}
