use crate::*;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStrength {
    Declared,
    Probed,
    Certified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilitySupport {
    Supported { strength: EvidenceStrength },
    Unsupported,
    Unknown,
}

impl CapabilitySupport {
    pub const fn declared() -> Self {
        Self::Supported {
            strength: EvidenceStrength::Declared,
        }
    }

    pub const fn probed() -> Self {
        Self::Supported {
            strength: EvidenceStrength::Probed,
        }
    }

    pub const fn certified() -> Self {
        Self::Supported {
            strength: EvidenceStrength::Certified,
        }
    }

    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Supported { .. })
    }

    pub const fn is_certified(self) -> bool {
        matches!(
            self,
            Self::Supported {
                strength: EvidenceStrength::Certified
            }
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Evidence<T> {
    Known(T),
    Unknown,
}

impl<T> Evidence<T> {
    fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TornWriteModel {
    Atomic { granularity: u32 },
    MayTear,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VolatileCacheModel {
    None,
    VolatileUntilFlush,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetyProfile {
    SimulationCertified,
    PortableDemo,
    ProductionReadOnly,
    ProductionWriteSafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileAuthorization {
    pub profile: SafetyProfile,
    pub capability_evidence_id: CapabilityEvidenceId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileError {
    pub profile: SafetyProfile,
    pub field: CapabilityField,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "profile {:?} lacks required {:?} evidence",
            self.profile, self.field
        )
    }
}

impl std::error::Error for ProfileError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreCapabilities {
    pub logical_length: Evidence<u64>,
    pub logical_block_size: Evidence<u32>,
    pub physical_block_size: Evidence<u32>,
    pub minimum_io_size: Evidence<u32>,
    pub optimal_io_size: Evidence<Option<u32>>,
    pub maximum_transfer: Evidence<u64>,
    pub required_alignment: Evidence<u32>,
    pub read: CapabilitySupport,
    pub write: CapabilitySupport,
    pub durable_flush: CapabilitySupport,
    pub fua: CapabilitySupport,
    pub stable_ordering: CapabilitySupport,
    pub atomic_write_granularity: Evidence<Option<u32>>,
    pub torn_write_model: TornWriteModel,
    pub volatile_cache: VolatileCacheModel,
    pub write_zeroes: CapabilitySupport,
    pub discard: CapabilitySupport,
    pub sparse_allocation: CapabilitySupport,
    pub cancellation: CapabilitySupport,
    pub stable_identity_sources: IdentitySourceSet,
    pub simulation_certification: CapabilitySupport,
    pub evidence_id: CapabilityEvidenceId,
}

impl StoreCapabilities {
    pub fn unknown() -> Self {
        Self {
            logical_length: Evidence::Unknown,
            logical_block_size: Evidence::Unknown,
            physical_block_size: Evidence::Unknown,
            minimum_io_size: Evidence::Unknown,
            optimal_io_size: Evidence::Unknown,
            maximum_transfer: Evidence::Unknown,
            required_alignment: Evidence::Unknown,
            read: CapabilitySupport::Unknown,
            write: CapabilitySupport::Unknown,
            durable_flush: CapabilitySupport::Unknown,
            fua: CapabilitySupport::Unknown,
            stable_ordering: CapabilitySupport::Unknown,
            atomic_write_granularity: Evidence::Unknown,
            torn_write_model: TornWriteModel::Unknown,
            volatile_cache: VolatileCacheModel::Unknown,
            write_zeroes: CapabilitySupport::Unknown,
            discard: CapabilitySupport::Unknown,
            sparse_allocation: CapabilitySupport::Unknown,
            cancellation: CapabilitySupport::Unknown,
            stable_identity_sources: IdentitySourceSet::none(),
            simulation_certification: CapabilitySupport::Unknown,
            evidence_id: CapabilityEvidenceId(0),
        }
    }

    pub fn portable_demo(
        logical_length: u64,
        logical_block_size: u32,
        maximum_transfer: u64,
        evidence_id: CapabilityEvidenceId,
    ) -> Self {
        Self {
            logical_length: Evidence::Known(logical_length),
            logical_block_size: Evidence::Known(logical_block_size),
            physical_block_size: Evidence::Unknown,
            minimum_io_size: Evidence::Known(logical_block_size),
            optimal_io_size: Evidence::Unknown,
            maximum_transfer: Evidence::Known(maximum_transfer),
            required_alignment: Evidence::Known(logical_block_size),
            read: CapabilitySupport::declared(),
            write: CapabilitySupport::declared(),
            durable_flush: CapabilitySupport::declared(),
            fua: CapabilitySupport::Unsupported,
            stable_ordering: CapabilitySupport::declared(),
            atomic_write_granularity: Evidence::Unknown,
            torn_write_model: TornWriteModel::Unknown,
            volatile_cache: VolatileCacheModel::Unknown,
            write_zeroes: CapabilitySupport::Unsupported,
            discard: CapabilitySupport::Unsupported,
            sparse_allocation: CapabilitySupport::Unknown,
            cancellation: CapabilitySupport::Unknown,
            stable_identity_sources: IdentitySourceSet::none(),
            simulation_certification: CapabilitySupport::Unsupported,
            evidence_id,
        }
    }

    pub(crate) fn validate_range(
        self: &StoreCapabilities,
        range: ByteRange,
        end: u64,
    ) -> Result<(), StoreError> {
        let length = match &self.logical_length {
            Evidence::Known(length) => *length,
            Evidence::Unknown => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::LogicalLength,
                ));
            }
        };
        if end > length {
            return Err(StoreError::RangeOutsideStore { end, length });
        }

        let maximum = match &self.maximum_transfer {
            Evidence::Known(maximum) => *maximum,
            Evidence::Unknown => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::MaximumTransfer,
                ));
            }
        };
        if range.length > maximum {
            return Err(StoreError::TransferTooLarge {
                length: range.length,
                maximum,
            });
        }

        let alignment = match &self.required_alignment {
            Evidence::Known(alignment) if *alignment != 0 => *alignment,
            Evidence::Known(_) => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::RequiredAlignment,
                ));
            }
            Evidence::Unknown => {
                return Err(StoreError::CapabilityUnknown(
                    CapabilityField::RequiredAlignment,
                ));
            }
        };
        if !range.offset.is_multiple_of(u64::from(alignment))
            || !range.length.is_multiple_of(u64::from(alignment))
        {
            return Err(StoreError::AlignmentViolation {
                offset: range.offset,
                length: range.length,
                alignment,
            });
        }
        Ok(())
    }

    pub fn authorize(&self, profile: SafetyProfile) -> Result<ProfileAuthorization, ProfileError> {
        let require_known = |condition: bool, field| {
            if condition {
                Ok(())
            } else {
                Err(ProfileError { profile, field })
            }
        };

        match profile {
            SafetyProfile::SimulationCertified => {
                require_support_for_profile(
                    self.simulation_certification,
                    profile,
                    CapabilityField::SimulationCertification,
                )?;
            }
            SafetyProfile::PortableDemo => {
                require_known(
                    self.logical_length.is_known(),
                    CapabilityField::LogicalLength,
                )?;
                require_known(
                    self.logical_block_size.is_known(),
                    CapabilityField::LogicalBlockSize,
                )?;
                require_known(
                    self.maximum_transfer.is_known(),
                    CapabilityField::MaximumTransfer,
                )?;
                require_known(
                    self.required_alignment.is_known(),
                    CapabilityField::RequiredAlignment,
                )?;
                require_support_for_profile(self.read, profile, CapabilityField::Read)?;
                require_support_for_profile(self.write, profile, CapabilityField::Write)?;
            }
            SafetyProfile::ProductionReadOnly => {
                self.authorize(SafetyProfile::PortableDemo)?;
                require_known(
                    self.physical_block_size.is_known(),
                    CapabilityField::PhysicalBlockSize,
                )?;
                require_known(
                    self.minimum_io_size.is_known(),
                    CapabilityField::MinimumIoSize,
                )?;
                require_known(
                    self.stable_identity_sources.has_stable_source(),
                    CapabilityField::StableIdentity,
                )?;
            }
            SafetyProfile::ProductionWriteSafe => {
                self.authorize(SafetyProfile::ProductionReadOnly)?;
                require_support_for_profile(self.write, profile, CapabilityField::Write)?;
                require_certified(self.durable_flush, profile, CapabilityField::DurableFlush)?;
                require_certified(
                    self.stable_ordering,
                    profile,
                    CapabilityField::StableOrdering,
                )?;
                require_known(
                    !matches!(self.torn_write_model, TornWriteModel::Unknown),
                    CapabilityField::TornWriteModel,
                )?;
                require_known(
                    !matches!(self.volatile_cache, VolatileCacheModel::Unknown),
                    CapabilityField::VolatileCache,
                )?;
            }
        }

        Ok(ProfileAuthorization {
            profile,
            capability_evidence_id: self.evidence_id,
        })
    }
}

pub(crate) fn require_support(
    support: CapabilitySupport,
    field: CapabilityField,
) -> Result<(), StoreError> {
    match support {
        CapabilitySupport::Supported { .. } => Ok(()),
        CapabilitySupport::Unsupported => Err(StoreError::CapabilityUnavailable { field, support }),
        CapabilitySupport::Unknown => Err(StoreError::CapabilityUnknown(field)),
    }
}

fn require_support_for_profile(
    support: CapabilitySupport,
    profile: SafetyProfile,
    field: CapabilityField,
) -> Result<(), ProfileError> {
    if support.is_supported() {
        Ok(())
    } else {
        Err(ProfileError { profile, field })
    }
}

fn require_certified(
    support: CapabilitySupport,
    profile: SafetyProfile,
    field: CapabilityField,
) -> Result<(), ProfileError> {
    if support.is_certified() {
        Ok(())
    } else {
        Err(ProfileError { profile, field })
    }
}
