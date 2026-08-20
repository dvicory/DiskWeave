//! Experimental, bounded parity-device envelope profiles.
//!
//! Data members remain ordinary payload bytes. This crate only describes the
//! optional metadata area already owned by a parity device.

use std::fmt;

pub const MAGIC: [u8; 8] = *b"DWVENV01";
pub const FORMAT_VERSION: u16 = 1;
pub const COPY_BYTES: usize = 4096;
pub const MAX_BODY_BYTES: usize = COPY_BYTES - HEADER_BYTES;
pub const MAX_BITMAP_BYTES: usize = 4096;
pub const DIRTY_REGION_BYTES: u64 = 1 << 20;
pub const FEATURE_COARSE_BITMAP: u32 = 1;
pub const SUPPORTED_REQUIRED_FEATURES: u32 = FEATURE_COARSE_BITMAP;
pub const SUPPORTED_COMPATIBLE_FEATURES: u32 = FEATURE_COARSE_BITMAP;
const PREFIX_BYTES: usize = 40;
const BODY_CHECKSUM_BYTES: usize = 32;
const HEADER_CHECKSUM_BYTES: usize = 32;
pub const HEADER_BYTES: usize = PREFIX_BYTES + BODY_CHECKSUM_BYTES + HEADER_CHECKSUM_BYTES;
pub const FLAG_BITMAP: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ByteRange {
    pub offset: u64,
    pub length: u64,
}

impl ByteRange {
    pub const fn new(offset: u64, length: u64) -> Option<Self> {
        if offset.checked_add(length).is_some() {
            Some(Self { offset, length })
        } else {
            None
        }
    }

    pub const fn end(self) -> u64 {
        self.offset + self.length
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Profile {
    BareParity,
    RedundantEnvelope,
    EnvelopeBitmap,
}

impl Profile {
    pub const ALL: [Self; 3] = [
        Self::BareParity,
        Self::RedundantEnvelope,
        Self::EnvelopeBitmap,
    ];

    const fn id(self) -> u8 {
        match self {
            Self::BareParity => 0,
            Self::RedundantEnvelope => 1,
            Self::EnvelopeBitmap => 2,
        }
    }

    fn from_id(id: u8) -> Result<Self, FormatError> {
        match id {
            0 => Ok(Self::BareParity),
            1 => Ok(Self::RedundantEnvelope),
            2 => Ok(Self::EnvelopeBitmap),
            _ => Err(FormatError::UnsupportedProfile(id)),
        }
    }

    pub const fn has_envelope(self) -> bool {
        !matches!(self, Self::BareParity)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProfileConfig {
    pub profile: Profile,
    pub parity_extent_length: u64,
    pub protected_data_length: u64,
    pub dirty_region_bytes: u64,
}

impl ProfileConfig {
    pub const fn new(
        profile: Profile,
        parity_extent_length: u64,
        protected_data_length: u64,
    ) -> Self {
        Self {
            profile,
            parity_extent_length,
            protected_data_length,
            dirty_region_bytes: DIRTY_REGION_BYTES,
        }
    }

    pub fn layout(self) -> Result<ProfileLayout, FormatError> {
        if self.profile == Profile::EnvelopeBitmap && self.dirty_region_bytes == 0 {
            return Err(FormatError::InvalidGeometry("dirty region size is zero"));
        }
        let copy_bytes = if self.profile.has_envelope() {
            COPY_BYTES as u64
        } else {
            0
        };
        let bitmap_bytes = if self.profile == Profile::EnvelopeBitmap {
            bitmap_bytes(self.protected_data_length, self.dirty_region_bytes)?
        } else {
            0
        };
        let metadata_bytes = copy_bytes
            .checked_mul(2)
            .and_then(|value| value.checked_add(bitmap_bytes))
            .ok_or(FormatError::ArithmeticOverflow)?;
        let payload_length = self
            .parity_extent_length
            .checked_sub(metadata_bytes)
            .ok_or(FormatError::InsufficientParityCapacity {
                profile: self.profile,
                parity_extent_length: self.parity_extent_length,
                required_metadata_bytes: metadata_bytes,
                protected_data_length: self.protected_data_length,
            })?;
        if self.protected_data_length > payload_length {
            return Err(FormatError::InsufficientParityCapacity {
                profile: self.profile,
                parity_extent_length: self.parity_extent_length,
                required_metadata_bytes: metadata_bytes,
                protected_data_length: self.protected_data_length,
            });
        }
        let payload_offset = if self.profile.has_envelope() {
            copy_bytes + bitmap_bytes
        } else {
            0
        };
        let payload = range(payload_offset, payload_length)?;
        let copy_a = self
            .profile
            .has_envelope()
            .then(|| range(0, copy_bytes))
            .transpose()?;
        let copy_b = self
            .profile
            .has_envelope()
            .then(|| range(self.parity_extent_length - copy_bytes, copy_bytes))
            .transpose()?;
        let bitmap = (self.profile == Profile::EnvelopeBitmap)
            .then(|| range(copy_bytes, bitmap_bytes))
            .transpose()?;
        Ok(ProfileLayout {
            profile: self.profile,
            metadata_bytes,
            bitmap_bytes,
            payload,
            copy_a,
            copy_b,
            bitmap,
            dirty_region_bytes: self.dirty_region_bytes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileLayout {
    pub profile: Profile,
    pub metadata_bytes: u64,
    pub bitmap_bytes: u64,
    pub payload: ByteRange,
    pub copy_a: Option<ByteRange>,
    pub copy_b: Option<ByteRange>,
    pub bitmap: Option<ByteRange>,
    pub dirty_region_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileEvaluation {
    pub profile: Profile,
    pub layout: Result<ProfileLayout, FormatError>,
}

pub fn compare_profiles(
    parity_extent_length: u64,
    protected_data_length: u64,
) -> [ProfileEvaluation; 3] {
    Profile::ALL.map(|profile| {
        let config = ProfileConfig::new(profile, parity_extent_length, protected_data_length);
        ProfileEvaluation {
            profile,
            layout: config.layout(),
        }
    })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SessionState {
    Prepared,
    Active,
    Dirty,
    Unknown,
    Clean,
    Closed,
}

impl SessionState {
    fn id(self) -> u8 {
        match self {
            Self::Prepared => 0,
            Self::Active => 1,
            Self::Dirty => 2,
            Self::Unknown => 3,
            Self::Clean => 4,
            Self::Closed => 5,
        }
    }

    fn from_id(id: u8) -> Result<Self, FormatError> {
        match id {
            0 => Ok(Self::Prepared),
            1 => Ok(Self::Active),
            2 => Ok(Self::Dirty),
            3 => Ok(Self::Unknown),
            4 => Ok(Self::Clean),
            5 => Ok(Self::Closed),
            _ => Err(FormatError::InvalidField("session state")),
        }
    }

    fn evidence(self) -> EvidenceState {
        match self {
            Self::Clean | Self::Closed => EvidenceState::Clean,
            Self::Prepared | Self::Active | Self::Dirty => EvidenceState::Dirty,
            Self::Unknown => EvidenceState::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MigrationState {
    Stable,
    Preparing,
    Complete,
    Aborted,
}

impl MigrationState {
    fn id(self) -> u8 {
        match self {
            Self::Stable => 0,
            Self::Preparing => 1,
            Self::Complete => 2,
            Self::Aborted => 3,
        }
    }

    fn from_id(id: u8) -> Result<Self, FormatError> {
        match id {
            0 => Ok(Self::Stable),
            1 => Ok(Self::Preparing),
            2 => Ok(Self::Complete),
            3 => Ok(Self::Aborted),
            _ => Err(FormatError::InvalidField("migration state")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvelopeRecord {
    pub profile: Profile,
    pub compatible_features: u32,
    pub required_features: u32,
    pub array_id: [u8; 16],
    pub parity_device_id: [u8; 16],
    pub parity_role: u8,
    pub codec_profile: u16,
    pub payload: ByteRange,
    pub logical_length: u64,
    pub logical_block_size: u32,
    pub stripe_width: u16,
    pub topology_generation: u64,
    pub session_generation: u64,
    pub last_global_recovery_clean_generation: u64,
    pub last_full_verified_checkpoint: Option<u64>,
    pub session_state: SessionState,
    pub migration_state: MigrationState,
    pub copy_generation: u64,
    pub copy_slot: u8,
    pub bitmap_region_bytes: u64,
    pub bitmap: Vec<u8>,
}

impl EnvelopeRecord {
    pub fn validate(&self) -> Result<(), FormatError> {
        if self.profile == Profile::BareParity {
            return Err(FormatError::InvalidField(
                "bare profile has no envelope copy",
            ));
        }
        if self.profile == Profile::EnvelopeBitmap
            && self.required_features & FEATURE_COARSE_BITMAP == 0
        {
            return Err(FormatError::InvalidField(
                "bitmap profile missing required feature",
            ));
        }
        if self.profile == Profile::RedundantEnvelope
            && self.required_features & FEATURE_COARSE_BITMAP != 0
        {
            return Err(FormatError::InvalidField(
                "non-bitmap profile requires bitmap feature",
            ));
        }
        if self.copy_slot > 1 {
            return Err(FormatError::InvalidField("copy slot"));
        }
        if self.payload.end() < self.payload.offset {
            return Err(FormatError::ArithmeticOverflow);
        }
        if self.logical_block_size == 0 || self.stripe_width == 0 {
            return Err(FormatError::InvalidGeometry(
                "block size and stripe width must be non-zero",
            ));
        }
        if self.profile == Profile::RedundantEnvelope && !self.bitmap.is_empty() {
            return Err(FormatError::InvalidField(
                "Profile B cannot contain a bitmap",
            ));
        }
        if self.profile == Profile::EnvelopeBitmap {
            if self.bitmap_region_bytes == 0 {
                return Err(FormatError::InvalidGeometry("bitmap region size is zero"));
            }
            let expected = bitmap_bytes(self.logical_length, self.bitmap_region_bytes)? as usize;
            if self.bitmap.len() != expected || self.bitmap.len() > MAX_BITMAP_BYTES {
                return Err(FormatError::BitmapLength {
                    expected,
                    actual: self.bitmap.len(),
                });
            }
        } else if self.bitmap_region_bytes != 0 {
            return Err(FormatError::InvalidField(
                "bitmap region on non-bitmap profile",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeUsability {
    Usable,
    ReadOnlyCompatible(u32),
    UnsupportedRequiredFeatures(u32),
}

impl DecodeUsability {
    pub const fn usable(self) -> bool {
        matches!(self, Self::Usable | Self::ReadOnlyCompatible(_))
    }

    pub const fn writable(self) -> bool {
        matches!(self, Self::Usable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedCopy {
    pub record: EnvelopeRecord,
    pub usability: DecodeUsability,
}

impl DecodedCopy {
    pub fn inspectable(&self) -> bool {
        true
    }
}

pub fn encode_copy(record: &EnvelopeRecord) -> Result<Vec<u8>, FormatError> {
    record.validate()?;
    let body = encode_body(record)?;
    if body.len() > MAX_BODY_BYTES {
        return Err(FormatError::BodyTooLarge(body.len()));
    }
    let mut output = vec![0; COPY_BYTES];
    output[..8].copy_from_slice(&MAGIC);
    output[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    output[10..12].copy_from_slice(&(HEADER_BYTES as u16).to_le_bytes());
    output[12..16].copy_from_slice(&(body.len() as u32).to_le_bytes());
    output[16] = record.profile.id();
    output[17] = if record.profile == Profile::EnvelopeBitmap {
        FLAG_BITMAP
    } else {
        0
    };
    output[18] = record.copy_slot;
    output[20..28].copy_from_slice(&record.copy_generation.to_le_bytes());
    output[28..32].copy_from_slice(&record.required_features.to_le_bytes());
    output[32..36].copy_from_slice(&record.compatible_features.to_le_bytes());
    output[40..72].copy_from_slice(blake3::hash(&body).as_bytes());
    output[104..104 + body.len()].copy_from_slice(&body);
    let header_digest = blake3::hash(&output[..72]);
    output[72..104].copy_from_slice(header_digest.as_bytes());
    Ok(output)
}

pub fn decode_copy(bytes: &[u8]) -> Result<DecodedCopy, FormatError> {
    if bytes.len() != COPY_BYTES {
        return Err(FormatError::InvalidLength {
            expected: COPY_BYTES,
            actual: bytes.len(),
        });
    }
    if bytes[..8] != MAGIC {
        return Err(FormatError::BadMagic);
    }
    if u16::from_le_bytes(bytes[8..10].try_into().unwrap()) != FORMAT_VERSION {
        return Err(FormatError::UnsupportedVersion(u16::from_le_bytes(
            bytes[8..10].try_into().unwrap(),
        )));
    }
    if u16::from_le_bytes(bytes[10..12].try_into().unwrap()) as usize != HEADER_BYTES {
        return Err(FormatError::InvalidField("header length"));
    }
    let body_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    if body_len > MAX_BODY_BYTES || HEADER_BYTES.checked_add(body_len).is_none() {
        return Err(FormatError::BodyTooLarge(body_len));
    }
    let expected_header = blake3::hash(&bytes[..72]);
    if bytes[72..104] != expected_header.as_bytes()[..] {
        return Err(FormatError::HeaderChecksum);
    }
    let body_end = HEADER_BYTES + body_len;
    if bytes[body_end..].iter().any(|byte| *byte != 0) {
        return Err(FormatError::NonCanonicalPadding);
    }
    let body = &bytes[HEADER_BYTES..body_end];
    let expected_body = blake3::hash(body);
    if bytes[40..72] != expected_body.as_bytes()[..] {
        return Err(FormatError::BodyChecksum);
    }
    let profile = Profile::from_id(bytes[16])?;
    let required_features = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
    let compatible_features = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
    let copy_generation = u64::from_le_bytes(bytes[20..28].try_into().unwrap());
    let copy_slot = bytes[18];
    if copy_slot > 1 {
        return Err(FormatError::InvalidField("copy slot"));
    }
    let record = decode_body(
        body,
        profile,
        required_features,
        compatible_features,
        copy_generation,
        copy_slot,
    )?;
    let unknown_required = required_features & !SUPPORTED_REQUIRED_FEATURES;
    let unknown_compatible = compatible_features & !SUPPORTED_COMPATIBLE_FEATURES;
    let usability = if unknown_required != 0 {
        DecodeUsability::UnsupportedRequiredFeatures(unknown_required)
    } else if unknown_compatible != 0 {
        DecodeUsability::ReadOnlyCompatible(unknown_compatible)
    } else {
        DecodeUsability::Usable
    };
    Ok(DecodedCopy { record, usability })
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EvidenceState {
    Clean,
    Dirty,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EvidenceStrength {
    None,
    SingleCopy,
    MatchingCopies,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CopyReason {
    NoCopies,
    MissingCopy,
    InvalidCopy,
    UnsupportedFeature,
    StaleTopology,
    ConflictingCopies,
    MatchingCopies,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopyAssessment {
    pub state: EvidenceState,
    pub strength: EvidenceStrength,
    pub reason: CopyReason,
    pub selected: Option<DecodedCopy>,
    pub clean_recovery_authorized: bool,
}

pub fn assess_copies(
    copy_a: Option<&[u8]>,
    copy_b: Option<&[u8]>,
    expected_topology_generation: Option<u64>,
) -> CopyAssessment {
    let decoded_a = copy_a.map(decode_copy);
    let decoded_b = copy_b.map(decode_copy);
    match (decoded_a, decoded_b) {
        (Some(Ok(a)), Some(Ok(b))) => {
            if !a.usability.writable() || !b.usability.writable() {
                return unknown(CopyReason::UnsupportedFeature, None);
            }
            let stale = expected_topology_generation.is_some_and(|expected| {
                a.record.topology_generation != expected || b.record.topology_generation != expected
            });
            if stale {
                return unknown(CopyReason::StaleTopology, Some(a));
            }
            if a.record.copy_slot == b.record.copy_slot || !same_semantics(&a.record, &b.record) {
                return unknown(CopyReason::ConflictingCopies, None);
            }
            CopyAssessment {
                state: a.record.session_state.evidence(),
                strength: EvidenceStrength::MatchingCopies,
                reason: CopyReason::MatchingCopies,
                selected: Some(a),
                clean_recovery_authorized: false,
            }
        }
        (Some(Ok(copy)), None) | (None, Some(Ok(copy))) => {
            let reason = if copy.usability.usable() {
                CopyReason::MissingCopy
            } else {
                CopyReason::UnsupportedFeature
            };
            unknown(reason, Some(copy))
        }
        (Some(Ok(copy)), Some(Err(_))) | (Some(Err(_)), Some(Ok(copy))) => {
            unknown(CopyReason::InvalidCopy, Some(copy))
        }
        (Some(Err(_)), None) | (None, Some(Err(_))) | (Some(Err(_)), Some(Err(_))) => {
            unknown(CopyReason::InvalidCopy, None)
        }
        (None, None) => unknown(CopyReason::NoCopies, None),
    }
}

pub fn inspect_bare(parity_extent_length: u64) -> Result<ProfileLayout, FormatError> {
    ProfileConfig::new(Profile::BareParity, parity_extent_length, 0).layout()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationDecision {
    Selected {
        profile: Profile,
        generation: u64,
    },
    RetainedPrior {
        profile: Profile,
        reason: CopyReason,
    },
}

pub fn select_migrated_profile(
    prior: Profile,
    target: Profile,
    copy_a: Option<&[u8]>,
    copy_b: Option<&[u8]>,
) -> MigrationDecision {
    if target == Profile::BareParity {
        return MigrationDecision::Selected {
            profile: target,
            generation: 0,
        };
    }
    let assessment = assess_copies(copy_a, copy_b, None);
    let Some(selected) = assessment.selected else {
        return MigrationDecision::RetainedPrior {
            profile: prior,
            reason: assessment.reason,
        };
    };
    if assessment.state == EvidenceState::Unknown
        || selected.record.profile != target
        || selected.record.migration_state != MigrationState::Complete
    {
        return MigrationDecision::RetainedPrior {
            profile: prior,
            reason: assessment.reason,
        };
    }
    MigrationDecision::Selected {
        profile: target,
        generation: selected.record.copy_generation,
    }
}

fn unknown(reason: CopyReason, selected: Option<DecodedCopy>) -> CopyAssessment {
    CopyAssessment {
        state: EvidenceState::Unknown,
        strength: selected
            .as_ref()
            .map_or(EvidenceStrength::None, |_| EvidenceStrength::SingleCopy),
        reason,
        selected,
        clean_recovery_authorized: false,
    }
}

fn same_semantics(a: &EnvelopeRecord, b: &EnvelopeRecord) -> bool {
    a.profile == b.profile
        && a.compatible_features == b.compatible_features
        && a.required_features == b.required_features
        && a.array_id == b.array_id
        && a.parity_device_id == b.parity_device_id
        && a.parity_role == b.parity_role
        && a.codec_profile == b.codec_profile
        && a.payload == b.payload
        && a.logical_length == b.logical_length
        && a.logical_block_size == b.logical_block_size
        && a.stripe_width == b.stripe_width
        && a.topology_generation == b.topology_generation
        && a.session_generation == b.session_generation
        && a.last_global_recovery_clean_generation == b.last_global_recovery_clean_generation
        && a.last_full_verified_checkpoint == b.last_full_verified_checkpoint
        && a.session_state == b.session_state
        && a.migration_state == b.migration_state
        && a.copy_generation == b.copy_generation
        && a.bitmap_region_bytes == b.bitmap_region_bytes
        && a.bitmap == b.bitmap
}

fn range(offset: u64, length: u64) -> Result<ByteRange, FormatError> {
    ByteRange::new(offset, length).ok_or(FormatError::ArithmeticOverflow)
}

fn bitmap_bytes(logical_length: u64, region_bytes: u64) -> Result<u64, FormatError> {
    if region_bytes == 0 {
        return Err(FormatError::InvalidGeometry("bitmap region size is zero"));
    }
    let regions = logical_length
        .checked_add(region_bytes - 1)
        .ok_or(FormatError::ArithmeticOverflow)?
        / region_bytes;
    let bytes = regions
        .checked_add(7)
        .ok_or(FormatError::ArithmeticOverflow)?
        / 8;
    if bytes > MAX_BITMAP_BYTES as u64 {
        return Err(FormatError::BitmapTooLarge(bytes));
    }
    Ok(bytes)
}

fn encode_body(record: &EnvelopeRecord) -> Result<Vec<u8>, FormatError> {
    let mut body = Vec::with_capacity(160 + record.bitmap.len());
    body.extend_from_slice(&record.array_id);
    body.extend_from_slice(&record.parity_device_id);
    body.push(record.parity_role);
    body.extend_from_slice(&record.codec_profile.to_le_bytes());
    body.extend_from_slice(&record.payload.offset.to_le_bytes());
    body.extend_from_slice(&record.payload.length.to_le_bytes());
    body.extend_from_slice(&record.logical_length.to_le_bytes());
    body.extend_from_slice(&record.logical_block_size.to_le_bytes());
    body.extend_from_slice(&record.stripe_width.to_le_bytes());
    body.extend_from_slice(&record.topology_generation.to_le_bytes());
    body.extend_from_slice(&record.session_generation.to_le_bytes());
    body.extend_from_slice(&record.last_global_recovery_clean_generation.to_le_bytes());
    body.push(record.last_full_verified_checkpoint.is_some() as u8);
    body.extend_from_slice(
        &record
            .last_full_verified_checkpoint
            .unwrap_or(0)
            .to_le_bytes(),
    );
    body.push(record.session_state.id());
    body.push(record.migration_state.id());
    body.extend_from_slice(&record.bitmap_region_bytes.to_le_bytes());
    body.extend_from_slice(&(record.bitmap.len() as u32).to_le_bytes());
    body.extend_from_slice(&record.bitmap);
    Ok(body)
}

fn decode_body(
    body: &[u8],
    profile: Profile,
    required_features: u32,
    compatible_features: u32,
    copy_generation: u64,
    copy_slot: u8,
) -> Result<EnvelopeRecord, FormatError> {
    let mut cursor = Cursor::new(body);
    let mut array_id = [0; 16];
    cursor.take(&mut array_id)?;
    let mut parity_device_id = [0; 16];
    cursor.take(&mut parity_device_id)?;
    let parity_role = cursor.u8()?;
    let codec_profile = cursor.u16()?;
    let payload =
        ByteRange::new(cursor.u64()?, cursor.u64()?).ok_or(FormatError::ArithmeticOverflow)?;
    let logical_length = cursor.u64()?;
    let logical_block_size = cursor.u32()?;
    let stripe_width = cursor.u16()?;
    let topology_generation = cursor.u64()?;
    let session_generation = cursor.u64()?;
    let last_global_recovery_clean_generation = cursor.u64()?;
    let verified = cursor.u8()?;
    if verified > 1 {
        return Err(FormatError::InvalidField("verified checkpoint flag"));
    }
    let verified_checkpoint = cursor.u64()?;
    let session_state = SessionState::from_id(cursor.u8()?)?;
    let migration_state = MigrationState::from_id(cursor.u8()?)?;
    let bitmap_region_bytes = cursor.u64()?;
    let bitmap_len = cursor.u32()? as usize;
    if bitmap_len > MAX_BITMAP_BYTES || bitmap_len > cursor.remaining() {
        return Err(FormatError::BitmapLength {
            expected: bitmap_len,
            actual: cursor.remaining(),
        });
    }
    let bitmap = cursor.bytes(bitmap_len)?.to_vec();
    if cursor.remaining() != 0 {
        return Err(FormatError::NonCanonicalBody);
    }
    let record = EnvelopeRecord {
        profile,
        compatible_features,
        required_features,
        array_id,
        parity_device_id,
        parity_role,
        codec_profile,
        payload,
        logical_length,
        logical_block_size,
        stripe_width,
        topology_generation,
        session_generation,
        last_global_recovery_clean_generation,
        last_full_verified_checkpoint: (verified == 1).then_some(verified_checkpoint),
        session_state,
        migration_state,
        copy_generation,
        copy_slot,
        bitmap_region_bytes,
        bitmap,
    };
    record.validate()?;
    if profile == Profile::EnvelopeBitmap && required_features & FEATURE_COARSE_BITMAP == 0 {
        return Err(FormatError::InvalidField(
            "bitmap profile missing required feature",
        ));
    }
    if profile == Profile::RedundantEnvelope && required_features & FEATURE_COARSE_BITMAP != 0 {
        return Err(FormatError::InvalidField(
            "non-bitmap profile requires bitmap feature",
        ));
    }
    Ok(record)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, target: &mut [u8]) -> Result<(), FormatError> {
        target.copy_from_slice(self.bytes(target.len())?);
        Ok(())
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], FormatError> {
        let end = self.checked_len(length)?;
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn checked_len(&self, length: usize) -> Result<usize, FormatError> {
        self.offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(FormatError::Truncated {
                offset: self.offset,
                needed: length,
            })
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn u8(&mut self) -> Result<u8, FormatError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, FormatError> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, FormatError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, FormatError> {
        Ok(u64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatError {
    ArithmeticOverflow,
    BadMagic,
    BodyChecksum,
    BodyTooLarge(usize),
    BitmapLength {
        expected: usize,
        actual: usize,
    },
    BitmapTooLarge(u64),
    HeaderChecksum,
    InsufficientParityCapacity {
        profile: Profile,
        parity_extent_length: u64,
        required_metadata_bytes: u64,
        protected_data_length: u64,
    },
    InvalidField(&'static str),
    InvalidGeometry(&'static str),
    InvalidLength {
        expected: usize,
        actual: usize,
    },
    NonCanonicalBody,
    NonCanonicalPadding,
    Truncated {
        offset: usize,
        needed: usize,
    },
    UnsupportedProfile(u8),
    UnsupportedVersion(u16),
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => write!(formatter, "format arithmetic overflow"),
            Self::BadMagic => write!(formatter, "invalid envelope magic"),
            Self::BodyChecksum => write!(formatter, "envelope body checksum mismatch"),
            Self::BodyTooLarge(length) => {
                write!(formatter, "envelope body is too large: {length} bytes")
            }
            Self::BitmapLength { expected, actual } => write!(
                formatter,
                "invalid bitmap length: expected {expected}, got {actual}"
            ),
            Self::BitmapTooLarge(length) => {
                write!(formatter, "bitmap exceeds bound: {length} bytes")
            }
            Self::HeaderChecksum => write!(formatter, "envelope header checksum mismatch"),
            Self::InsufficientParityCapacity {
                profile,
                parity_extent_length,
                required_metadata_bytes,
                protected_data_length,
            } => write!(
                formatter,
                "{profile:?} needs {required_metadata_bytes} metadata bytes in a {parity_extent_length}-byte extent for {protected_data_length} protected bytes"
            ),
            Self::InvalidField(field) => write!(formatter, "invalid envelope field: {field}"),
            Self::InvalidGeometry(message) => {
                write!(formatter, "invalid envelope geometry: {message}")
            }
            Self::InvalidLength { expected, actual } => write!(
                formatter,
                "invalid envelope length: expected {expected}, got {actual}"
            ),
            Self::NonCanonicalBody => write!(formatter, "non-canonical envelope body"),
            Self::NonCanonicalPadding => write!(formatter, "non-zero envelope padding"),
            Self::Truncated { offset, needed } => write!(
                formatter,
                "truncated envelope at {offset}, need {needed} bytes"
            ),
            Self::UnsupportedProfile(profile) => {
                write!(formatter, "unsupported envelope profile: {profile}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported envelope version: {version}")
            }
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(profile: Profile, slot: u8) -> EnvelopeRecord {
        let bitmap = if profile == Profile::EnvelopeBitmap {
            vec![0; 1]
        } else {
            Vec::new()
        };
        EnvelopeRecord {
            profile,
            compatible_features: 0,
            required_features: (profile == Profile::EnvelopeBitmap) as u32,
            array_id: [1; 16],
            parity_device_id: [2; 16],
            parity_role: 0,
            codec_profile: 1,
            payload: ByteRange::new(4096, 8192).unwrap(),
            logical_length: if profile == Profile::EnvelopeBitmap {
                1
            } else {
                0
            },
            logical_block_size: 4096,
            stripe_width: 2,
            topology_generation: 7,
            session_generation: 8,
            last_global_recovery_clean_generation: 6,
            last_full_verified_checkpoint: None,
            session_state: SessionState::Dirty,
            migration_state: MigrationState::Stable,
            copy_generation: 9,
            copy_slot: slot,
            bitmap_region_bytes: if profile == Profile::EnvelopeBitmap {
                DIRTY_REGION_BYTES
            } else {
                0
            },
            bitmap,
        }
    }

    #[test]
    fn profile_capacity_is_exact() {
        let layout = ProfileConfig::new(Profile::RedundantEnvelope, 10_240, 2_048)
            .layout()
            .unwrap();
        assert_eq!(layout.metadata_bytes, (COPY_BYTES * 2) as u64);
        assert_eq!(layout.payload, ByteRange::new(4096, 2048).unwrap());
        assert!(
            ProfileConfig::new(Profile::RedundantEnvelope, 10_240, 2_049)
                .layout()
                .is_err()
        );
    }

    #[test]
    fn profile_a_has_no_metadata() {
        let layout = inspect_bare(100).unwrap();
        assert_eq!(layout.metadata_bytes, 0);
        assert_eq!(layout.payload, ByteRange::new(0, 100).unwrap());
        assert!(layout.copy_a.is_none());
    }

    #[test]
    fn round_trip_is_bounded_and_independently_decoded() {
        let bytes = encode_copy(&record(Profile::RedundantEnvelope, 0)).unwrap();
        assert_eq!(bytes.len(), COPY_BYTES);
        let decoded = decode_copy(&bytes).unwrap();
        assert_eq!(decoded.record, record(Profile::RedundantEnvelope, 0));
        assert!(decoded.usability.writable());
    }

    #[test]
    fn hand_authored_mutations_are_rejected() {
        let mut bytes = encode_copy(&record(Profile::RedundantEnvelope, 0)).unwrap();
        bytes[HEADER_BYTES] ^= 1;
        assert_eq!(decode_copy(&bytes), Err(FormatError::BodyChecksum));
        let mut truncated = bytes.clone();
        truncated.truncate(COPY_BYTES - 1);
        assert!(matches!(
            decode_copy(&truncated),
            Err(FormatError::InvalidLength { .. })
        ));
    }
    #[test]
    fn seeded_envelope_mutations_fail_closed() {
        const MUTATION_OFFSETS: [usize; 7] = [0, 8, 10, 16, 18, HEADER_BYTES, COPY_BYTES - 1];
        for (line, seed_text) in
            include_str!("../../../verification/corpus/envelope-mutation-seeds.txt")
                .lines()
                .enumerate()
        {
            let seed = u64::from_str_radix(seed_text.trim(), 16)
                .unwrap_or_else(|_| panic!("invalid envelope seed on line {}", line + 1));
            let mut bytes = encode_copy(&record(Profile::RedundantEnvelope, 0)).unwrap();
            let offset = MUTATION_OFFSETS[(seed as usize) % MUTATION_OFFSETS.len()];
            bytes[offset] ^= 1 + (seed as u8);
            assert!(
                decode_copy(&bytes).is_err(),
                "mutation at offset {offset} was accepted"
            );
        }
    }

    #[test]
    fn oversized_input_is_rejected_before_decode() {
        let mut bytes = encode_copy(&record(Profile::RedundantEnvelope, 0)).unwrap();
        bytes.push(0);
        assert_eq!(
            decode_copy(&bytes),
            Err(FormatError::InvalidLength {
                expected: COPY_BYTES,
                actual: COPY_BYTES + 1,
            })
        );
    }

    #[test]
    fn unknown_required_features_remain_inspectable_but_unusable() {
        let mut value = record(Profile::RedundantEnvelope, 0);
        value.required_features = 1 << 31;
        let bytes = encode_copy(&value).unwrap();
        let decoded = decode_copy(&bytes).unwrap();
        assert!(decoded.inspectable());
        assert!(!decoded.usability.usable());
    }

    #[test]
    fn matching_copies_are_not_a_gate_h_clean_authorization() {
        let a = encode_copy(&record(Profile::RedundantEnvelope, 0)).unwrap();
        let b = encode_copy(&record(Profile::RedundantEnvelope, 1)).unwrap();
        let assessment = assess_copies(Some(&a), Some(&b), Some(7));
        assert_eq!(assessment.state, EvidenceState::Dirty);
        assert_eq!(assessment.strength, EvidenceStrength::MatchingCopies);
        assert!(!assessment.clean_recovery_authorized);
    }

    #[test]
    fn conflicts_are_conservative() {
        let mut first = record(Profile::RedundantEnvelope, 0);
        first.session_generation = 2;
        let a = encode_copy(&first).unwrap();
        let b = encode_copy(&record(Profile::RedundantEnvelope, 1)).unwrap();
        let assessment = assess_copies(Some(&a), Some(&b), Some(7));
        assert_eq!(assessment.state, EvidenceState::Unknown);
        assert_eq!(assessment.reason, CopyReason::ConflictingCopies);
    }

    #[test]
    fn migration_keeps_prior_until_both_new_copies_complete() {
        let mut value = record(Profile::RedundantEnvelope, 0);
        value.migration_state = MigrationState::Complete;
        let a = encode_copy(&value).unwrap();
        value.copy_slot = 1;
        let b = encode_copy(&value).unwrap();
        assert_eq!(
            select_migrated_profile(
                Profile::BareParity,
                Profile::RedundantEnvelope,
                Some(&a),
                Some(&b)
            ),
            MigrationDecision::Selected {
                profile: Profile::RedundantEnvelope,
                generation: 9
            }
        );
        assert_eq!(
            select_migrated_profile(
                Profile::BareParity,
                Profile::RedundantEnvelope,
                Some(&a),
                None
            ),
            MigrationDecision::RetainedPrior {
                profile: Profile::BareParity,
                reason: CopyReason::MissingCopy
            }
        );
    }
}
