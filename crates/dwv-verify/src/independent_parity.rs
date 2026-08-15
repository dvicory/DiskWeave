use dwv_codec::{Geometry, ParityCodec, XorReference};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const COMMAND: &str = "dwv-independent-parity";
const DESCRIPTOR_VERSION: &str = "dwv.independent-parity.v1";
const RESULT_SCHEMA: &str = "dwv.independent-parity-verification.v1";
const FORMAT_CLAIM: &str = "experimental";
const EVIDENCE_TIER: &str = "file-backed-model";
const INVALID_EXIT_CODE: u8 = 2;
const OPERATIONAL_EXIT_CODE: u8 = 1;
const MAX_DESCRIPTOR_BYTES: u64 = 1024 * 1024;
const MAX_PAYLOADS: usize = 64;
const MAX_PATH_BYTES: usize = 4096;
const MAX_TOTAL_PATH_BYTES: usize = 256 * 1024;
const MAX_PROTECTED_LENGTH: u64 = 1 << 40;
const MAX_REGION_SIZE: u64 = 8 * 1024 * 1024;
const MAX_REGION_COUNT: usize = 16_384;
const MAX_BUFFER_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;

const NON_CLAIMS: &[&str] = &[
    "does not establish production identity, generation, custody, or current protection",
    "does not establish historical protection, checksum validity, or hardware durability",
    "does not identify a bad shard or authorize repair, recovery, publication, or mutation",
    "does not assert clean state or graduate a persistent format",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ByteRange {
    offset: u64,
    length: u64,
}

impl ByteRange {
    fn end(self) -> Option<u64> {
        self.offset.checked_add(self.length)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputMode {
    Human,
    Json,
}

#[derive(Debug)]
struct Descriptor {
    selected: ByteRange,
    protected_length: u64,
    region_size: u64,
    region_count: usize,
    data_paths: Vec<PathBuf>,
    parity_path: PathBuf,
    output_mode: OutputMode,
}

#[derive(Debug)]
struct DescriptorError(&'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessStatus {
    CompletedObservation,
    InvalidUnsupportedInput,
    OperationalFailure,
}

impl ProcessStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::CompletedObservation => "completed-observation",
            Self::InvalidUnsupportedInput => "invalid-unsupported-input",
            Self::OperationalFailure => "unable-to-produce-bounded-report",
        }
    }

    fn exit_code(self) -> u8 {
        match self {
            Self::CompletedObservation => 0,
            Self::InvalidUnsupportedInput => INVALID_EXIT_CODE,
            Self::OperationalFailure => OPERATIONAL_EXIT_CODE,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Disposition {
    Matched,
    Mismatched,
    Incomplete,
    Unknown,
}

impl Disposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
            Self::Incomplete => "incomplete",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RegionFinding {
    range: ByteRange,
    disposition: Disposition,
    reason: &'static str,
}

#[derive(Clone, Debug)]
struct SemanticResult {
    descriptor_version: Option<&'static str>,
    status: ProcessStatus,
    reason: &'static str,
    regions: Vec<RegionFinding>,
}

impl SemanticResult {
    fn invalid(reason: &'static str) -> Self {
        Self {
            descriptor_version: None,
            status: ProcessStatus::InvalidUnsupportedInput,
            reason,
            regions: Vec::new(),
        }
    }

    fn operational(reason: &'static str) -> Self {
        Self {
            descriptor_version: None,
            status: ProcessStatus::OperationalFailure,
            reason,
            regions: Vec::new(),
        }
    }

    fn completed(regions: Vec<RegionFinding>) -> Self {
        let reason = if regions
            .iter()
            .any(|region| region.disposition == Disposition::Unknown)
        {
            "one or more required payload reads were unavailable"
        } else if regions
            .iter()
            .any(|region| region.disposition == Disposition::Incomplete)
        {
            "one or more required payload reads were incomplete"
        } else if regions
            .iter()
            .any(|region| region.disposition == Disposition::Mismatched)
        {
            "one or more bounded XOR equations mismatched"
        } else {
            "all selected bounded XOR equations matched"
        };
        Self {
            descriptor_version: Some(DESCRIPTOR_VERSION),
            status: ProcessStatus::CompletedObservation,
            reason,
            regions,
        }
    }

    fn as_json(&self) -> Value {
        let regions = self
            .regions
            .iter()
            .map(|region| {
                json!({
                    "offset": region.range.offset,
                    "length": region.range.length,
                    "disposition": region.disposition.as_str(),
                    "reason": region.reason,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "schema": RESULT_SCHEMA,
            "command": COMMAND,
            "outcome": self.status.as_str(),
            "descriptor_version": self.descriptor_version,
            "format_claim": FORMAT_CLAIM,
            "experimental": true,
            "evidence_tier": EVIDENCE_TIER,
            "regions": regions,
            "non_claims": NON_CLAIMS,
            "reason": self.reason,
            "process_status": self.status.as_str(),
            "exit_code": self.status.exit_code(),
        })
    }
}

struct DuplicateKeySeed;

struct DuplicateKeyVisitor;

impl<'de> Visitor<'de> for DuplicateKeyVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_string<E>(self, _: String) -> Result<(), E> {
        Ok(())
    }

    fn visit_bytes<E>(self, _: &[u8]) -> Result<(), E> {
        Ok(())
    }

    fn visit_byte_buf<E>(self, _: Vec<u8>) -> Result<(), E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateKeyVisitor)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence
            .next_element_seed(DuplicateKeySeed)?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(de::Error::custom("duplicate JSON object member"));
            }
            map.next_value_seed(DuplicateKeySeed)?;
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for DuplicateKeySeed {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateKeyVisitor)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRange {
    offset: Option<u64>,
    length: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDescriptor {
    version: Option<String>,
    profile: Option<String>,
    protected_length: Option<u64>,
    selected_range: Option<Option<RawRange>>,
    whole_protected_range: Option<Option<bool>>,
    region_size: Option<u64>,
    max_regions: Option<u64>,
    data_payloads: Option<Vec<String>>,
    parity_payload: Option<String>,
    features: Option<Option<Vec<String>>>,
    output: Option<Option<String>>,
}

fn reject_duplicate_members(bytes: &[u8]) -> Result<(), DescriptorError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    if let Err(error) = deserializer.deserialize_any(DuplicateKeyVisitor) {
        let reason = if error.classify() == serde_json::error::Category::Data {
            "descriptor contains duplicate members"
        } else {
            "descriptor is malformed JSON"
        };
        return Err(DescriptorError(reason));
    }
    deserializer
        .end()
        .map_err(|_| DescriptorError("descriptor contains trailing or malformed JSON"))?;
    Ok(())
}

fn parse_range(range: &RawRange) -> Result<ByteRange, DescriptorError> {
    let offset = range
        .offset
        .ok_or(DescriptorError("selected range offset is missing"))?;
    let length = range
        .length
        .ok_or(DescriptorError("selected range length is missing"))?;
    offset
        .checked_add(length)
        .ok_or(DescriptorError("selected range overflows"))?;
    if length == 0 {
        return Err(DescriptorError("selected range must be non-empty"));
    }
    Ok(ByteRange { offset, length })
}

fn parse_path(path: &str) -> Result<PathBuf, DescriptorError> {
    if path.contains('\0') {
        return Err(DescriptorError("payload identity contains a NUL byte"));
    }
    if path.is_empty() || path.as_bytes().len() > MAX_PATH_BYTES {
        return Err(DescriptorError("payload identity is empty or over its bound"));
    }
    Ok(PathBuf::from(path))
}

fn parse_descriptor(bytes: &[u8]) -> Result<Descriptor, DescriptorError> {
    if bytes.is_empty() {
        return Err(DescriptorError("descriptor is empty"));
    }
    if bytes.len() as u64 > MAX_DESCRIPTOR_BYTES {
        return Err(DescriptorError("descriptor exceeds its byte bound"));
    }
    reject_duplicate_members(bytes)?;
    let raw: RawDescriptor = serde_json::from_slice(bytes)
        .map_err(|_| DescriptorError("descriptor fields are malformed or unsupported"))?;
    let version = raw
        .version
        .ok_or(DescriptorError("descriptor version is missing"))?;
    if version != DESCRIPTOR_VERSION {
        return Err(DescriptorError("descriptor version is unsupported"));
    }
    let profile = raw
        .profile
        .ok_or(DescriptorError("descriptor profile is missing"))?;
    if profile != "single-xor" {
        return Err(DescriptorError("descriptor profile is unsupported"));
    }
    if let Some(features) = raw.features {
        match features {
            Some(features) if !features.is_empty() => {
                return Err(DescriptorError("descriptor feature is unsupported"));
            }
            Some(_) => {}
            None => return Err(DescriptorError("descriptor features have the wrong type")),
        }
    }
    let protected_length = raw
        .protected_length
        .ok_or(DescriptorError("protected length is missing"))?;
    if protected_length == 0 || protected_length > MAX_PROTECTED_LENGTH {
        return Err(DescriptorError("protected length is outside its bound"));
    }
    let selected = match (raw.selected_range, raw.whole_protected_range) {
        (Some(Some(range)), None) => parse_range(&range)?,
        (None, Some(Some(true))) => ByteRange {
            offset: 0,
            length: protected_length,
        },
        (None, Some(Some(false))) => {
            return Err(DescriptorError("whole protected range must be true"));
        }
        (None, None) => {
            return Err(DescriptorError("descriptor is missing a selected range"));
        }
        (Some(None), None) => {
            return Err(DescriptorError("selected range has the wrong type"));
        }
        (None, Some(None)) => {
            return Err(DescriptorError(
                "whole protected range has the wrong type",
            ));
        }
        (Some(_), Some(_)) => {
            return Err(DescriptorError("descriptor must select exactly one range"));
        }
    };
    let selected_end = selected
        .end()
        .ok_or(DescriptorError("selected range overflows"))?;
    if selected_end > protected_length {
        return Err(DescriptorError("selected range exceeds protected geometry"));
    }
    let region_size = raw
        .region_size
        .ok_or(DescriptorError("region size is missing"))?;
    if region_size == 0 || region_size > MAX_REGION_SIZE {
        return Err(DescriptorError("region size is outside its bound"));
    }
    let max_regions = usize::try_from(
        raw.max_regions
            .ok_or(DescriptorError("maximum region count is missing"))?,
    )
    .map_err(|_| DescriptorError("maximum region count does not fit"))?;
    if max_regions == 0 || max_regions > MAX_REGION_COUNT {
        return Err(DescriptorError("maximum region count is outside its bound"));
    }
    let region_count_u64 = selected
        .length
        .checked_add(region_size - 1)
        .ok_or(DescriptorError("region count overflows"))?
        / region_size;
    let region_count = usize::try_from(region_count_u64)
        .map_err(|_| DescriptorError("region count does not fit"))?;
    if region_count == 0 || region_count > max_regions || region_count > MAX_REGION_COUNT {
        return Err(DescriptorError("selected range exceeds its region bound"));
    }
    let data_payloads = raw
        .data_payloads
        .ok_or(DescriptorError("data payloads are missing"))?;
    if data_payloads.is_empty() || data_payloads.len() > MAX_PAYLOADS {
        return Err(DescriptorError("data payload count is outside its bound"));
    }
    let parity_payload = raw
        .parity_payload
        .ok_or(DescriptorError("parity payload is missing"))?;
    let parity_path = parse_path(&parity_payload)?;
    let mut data_paths = Vec::with_capacity(data_payloads.len());
    let mut total_path_bytes = parity_path.as_os_str().len();
    for payload in data_payloads {
        let path = parse_path(&payload)?;
        total_path_bytes = total_path_bytes
            .checked_add(path.as_os_str().len())
            .ok_or(DescriptorError("payload identities exceed their bound"))?;
        if data_paths.iter().any(|existing| existing == &path) || path == parity_path {
            return Err(DescriptorError("payload identities are duplicate or aliased"));
        }
        data_paths.push(path);
    }
    if total_path_bytes > MAX_TOTAL_PATH_BYTES {
        return Err(DescriptorError("payload identities exceed their bound"));
    }
    let total_buffer_count = data_paths
        .len()
        .checked_add(2)
        .ok_or(DescriptorError("region buffers exceed their bound"))?;
    let total_buffers = u64::try_from(total_buffer_count)
        .ok()
        .and_then(|count| count.checked_mul(region_size))
        .ok_or(DescriptorError("region buffers exceed their bound"))?;
    if total_buffers >= MAX_BUFFER_BYTES {
        return Err(DescriptorError("region buffers exceed their bound"));
    }
    let output_mode = match raw.output {
        None => OutputMode::Human,
        Some(Some(output)) => match output.as_str() {
            "human" => OutputMode::Human,
            "json" => OutputMode::Json,
            _ => return Err(DescriptorError("output mode is unsupported")),
        },
        Some(None) => return Err(DescriptorError("output mode has the wrong type")),
    };
    if region_count > MAX_RESULT_BYTES / 128 {
        return Err(DescriptorError("bounded report exceeds its output bound"));
    }
    Ok(Descriptor {
        selected,
        protected_length,
        region_size,
        region_count,
        data_paths,
        parity_path,
        output_mode,
    })
}

enum DescriptorLoadError {
    Invalid(&'static str),
    Operational(&'static str),
}

fn read_descriptor(path: &Path) -> Result<Vec<u8>, DescriptorLoadError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(DescriptorLoadError::Invalid("descriptor file is missing"));
        }
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
            return Err(DescriptorLoadError::Invalid("descriptor path is invalid"));
        }
        Err(_) => return Err(DescriptorLoadError::Operational("descriptor could not be read")),
    };
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(DescriptorLoadError::Invalid("descriptor file is missing"));
        }
        Err(_) => {
            return Err(DescriptorLoadError::Operational(
                "descriptor metadata is unavailable",
            ));
        }
    };
    if !metadata.is_file() {
        return Err(DescriptorLoadError::Invalid(
            "descriptor is not a regular file",
        ));
    }
    if metadata.len() == 0 {
        return Err(DescriptorLoadError::Invalid("descriptor is empty"));
    }
    if metadata.len() > MAX_DESCRIPTOR_BYTES {
        return Err(DescriptorLoadError::Invalid(
            "descriptor exceeds its byte bound",
        ));
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len()).map_err(|_| {
            DescriptorLoadError::Invalid("descriptor size does not fit its bound")
        })?,
    );
    let mut limited = file.take(MAX_DESCRIPTOR_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .map_err(|_| DescriptorLoadError::Operational("descriptor could not be read"))?;
    if bytes.is_empty() {
        return Err(DescriptorLoadError::Invalid("descriptor is empty"));
    }
    if bytes.len() as u64 > MAX_DESCRIPTOR_BYTES {
        return Err(DescriptorLoadError::Invalid(
            "descriptor exceeds its byte bound",
        ));
    }
    Ok(bytes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileIdentity {
    canonical_path: PathBuf,
    first: u64,
    second: u64,
}

#[cfg(unix)]
fn file_identity(path: &Path, metadata: &Metadata) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    Some(FileIdentity {
        canonical_path: path.to_path_buf(),
        first: metadata.dev(),
        second: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn file_identity(_: &Path, _: &Metadata) -> Option<FileIdentity> {
    None
}

fn identities_alias(left: &FileIdentity, right: &FileIdentity) -> bool {
    left.canonical_path == right.canonical_path
        || (left.first == right.first && left.second == right.second)
}

#[derive(Debug)]
struct OpenedPayload {
    file: File,
    declared_length: u64,
}

fn open_payloads(
    descriptor: &Descriptor,
) -> Result<(Vec<OpenedPayload>, OpenedPayload), DescriptorError> {
    let mut paths = descriptor.data_paths.clone();
    paths.push(descriptor.parity_path.clone());
    let canonical_paths = paths
        .iter()
        .map(|path| {
            std::fs::canonicalize(path)
                .map_err(|_| DescriptorError("payload identity is unavailable"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (index, path) in canonical_paths.iter().enumerate() {
        if canonical_paths
            .iter()
            .skip(index + 1)
            .any(|other| other == path)
        {
            return Err(DescriptorError("payload identities are duplicate or aliased"));
        }
    }
    let mut opened = Vec::with_capacity(canonical_paths.len());
    let mut identities = Vec::with_capacity(canonical_paths.len());
    for path in canonical_paths {
        let file = File::open(&path)
            .map_err(|_| DescriptorError("payload identity is unavailable"))?;
        let metadata = file
            .metadata()
            .map_err(|_| DescriptorError("payload metadata is unavailable"))?;
        if !metadata.is_file() || metadata.len() > descriptor.protected_length {
            return Err(DescriptorError("payload geometry is outside its bound"));
        }
        let path_metadata = std::fs::metadata(&path)
            .map_err(|_| DescriptorError("payload identity is unavailable"))?;
        if !path_metadata.is_file() {
            return Err(DescriptorError("payload identity is unavailable"));
        }
        let identity = file_identity(&path, &metadata)
            .ok_or(DescriptorError("payload identity is unavailable"))?;
        let path_identity = file_identity(&path, &path_metadata)
            .ok_or(DescriptorError("payload identity is unavailable"))?;
        if identity != path_identity {
            return Err(DescriptorError("payload identity changed during admission"));
        }
        if identities
            .iter()
            .any(|other| identities_alias(other, &identity))
        {
            return Err(DescriptorError("payload identities are duplicate or aliased"));
        }
        identities.push(identity);
        opened.push(OpenedPayload {
            declared_length: metadata.len(),
            file,
        });
    }
    let parity = opened
        .pop()
        .ok_or(DescriptorError("payload count is invalid"))?;
    Ok((opened, parity))
}

enum ReadOutcome {
    Bytes(Vec<u8>),
    Incomplete,
    Unknown,
}

fn read_data_region(payload: &mut OpenedPayload, range: ByteRange) -> ReadOutcome {
    let length = match usize::try_from(range.length) {
        Ok(length) => length,
        Err(_) => return ReadOutcome::Unknown,
    };
    let mut bytes = vec![0; length];
    if range.offset >= payload.declared_length {
        return ReadOutcome::Bytes(bytes);
    }
    let end = match range.end() {
        Some(end) => end,
        None => return ReadOutcome::Unknown,
    };
    let read_end = end.min(payload.declared_length);
    let read_length = match read_end.checked_sub(range.offset) {
        Some(length) => match usize::try_from(length) {
            Ok(length) => length,
            Err(_) => return ReadOutcome::Unknown,
        },
        None => return ReadOutcome::Unknown,
    };
    if payload.file.seek(SeekFrom::Start(range.offset)).is_err() {
        return ReadOutcome::Unknown;
    }
    match payload.file.read_exact(&mut bytes[..read_length]) {
        Ok(()) => ReadOutcome::Bytes(bytes),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => ReadOutcome::Incomplete,
        Err(_) => ReadOutcome::Unknown,
    }
}

fn read_parity_region(payload: &mut OpenedPayload, range: ByteRange) -> ReadOutcome {
    let length = match usize::try_from(range.length) {
        Ok(length) => length,
        Err(_) => return ReadOutcome::Unknown,
    };
    let end = match range.end() {
        Some(end) => end,
        None => return ReadOutcome::Unknown,
    };
    if end > payload.declared_length {
        return ReadOutcome::Incomplete;
    }
    let mut bytes = vec![0; length];
    if payload.file.seek(SeekFrom::Start(range.offset)).is_err() {
        return ReadOutcome::Unknown;
    }
    match payload.file.read_exact(&mut bytes) {
        Ok(()) => ReadOutcome::Bytes(bytes),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => ReadOutcome::Incomplete,
        Err(_) => ReadOutcome::Unknown,
    }
}

fn observe(descriptor: Descriptor) -> SemanticResult {
    let (mut data, mut parity) = match open_payloads(&descriptor) {
        Ok(payloads) => payloads,
        Err(error) => return SemanticResult::invalid(error.0),
    };
    let mut regions = Vec::with_capacity(descriptor.region_count);
    let mut offset = descriptor.selected.offset;
    let mut remaining = descriptor.selected.length;
    while remaining > 0 {
        let range = ByteRange {
            offset,
            length: descriptor.region_size.min(remaining),
        };
        let mut data_bytes = Vec::with_capacity(data.len());
        let mut incomplete = false;
        let mut unknown = false;
        for payload in &mut data {
            match read_data_region(payload, range) {
                ReadOutcome::Bytes(bytes) => data_bytes.push(bytes),
                ReadOutcome::Incomplete => incomplete = true,
                ReadOutcome::Unknown => unknown = true,
            }
        }
        let parity_bytes = match read_parity_region(&mut parity, range) {
            ReadOutcome::Bytes(bytes) => Some(bytes),
            ReadOutcome::Incomplete => {
                incomplete = true;
                None
            }
            ReadOutcome::Unknown => {
                unknown = true;
                None
            }
        };
        let (disposition, reason) = if unknown {
            (
                Disposition::Unknown,
                "a required payload read was unavailable",
            )
        } else if incomplete {
            (
                Disposition::Incomplete,
                "a required payload read was incomplete",
            )
        } else if let Some(parity_bytes) = parity_bytes {
            let geometry =
                match Geometry::new(vec![range.length; data_bytes.len()], range.length) {
                    Ok(geometry) => geometry,
                    Err(_) => {
                        regions.push(RegionFinding {
                            range,
                            disposition: Disposition::Unknown,
                            reason: "the bounded equation could not be evaluated",
                        });
                        remaining -= range.length;
                        if remaining > 0 {
                            offset = match offset.checked_add(range.length) {
                                Some(offset) => offset,
                                None => {
                                    return SemanticResult::invalid("selected range overflows");
                                }
                            };
                        }
                        continue;
                    }
                };
            let references = data_bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
            let expected = match XorReference.compute_parity(&geometry, &references) {
                Ok(expected) => expected,
                Err(_) => {
                    regions.push(RegionFinding {
                        range,
                        disposition: Disposition::Unknown,
                        reason: "the bounded equation could not be evaluated",
                    });
                    remaining -= range.length;
                    if remaining > 0 {
                        offset = match offset.checked_add(range.length) {
                            Some(offset) => offset,
                            None => return SemanticResult::invalid("selected range overflows"),
                        };
                    }
                    continue;
                }
            };
            if expected == parity_bytes {
                (
                    Disposition::Matched,
                    "the observed parity matched the explicit XOR equation",
                )
            } else {
                (
                    Disposition::Mismatched,
                    "the observed parity differed from the explicit XOR equation",
                )
            }
        } else {
            (
                Disposition::Unknown,
                "the bounded equation could not be evaluated",
            )
        };
        regions.push(RegionFinding {
            range,
            disposition,
            reason,
        });
        remaining -= range.length;
        if remaining > 0 {
            offset = match offset.checked_add(range.length) {
                Some(offset) => offset,
                None => return SemanticResult::invalid("selected range overflows"),
            };
        }
    }
    SemanticResult::completed(regions)
}

fn parse_args() -> Result<
    (Option<OutputMode>, PathBuf),
    (OutputMode, &'static str),
> {
    let mut arguments = env::args_os().skip(1);
    let first = match arguments.next() {
        Some(first) => first,
        None => {
            return Err((
                OutputMode::Human,
                "usage: dwv-independent-parity [--json] <descriptor>",
            ));
        }
    };
    let (requested_mode, descriptor) = if first == OsStr::new("--json") {
        let descriptor = match arguments.next() {
            Some(descriptor) => descriptor,
            None => {
                return Err((
                    OutputMode::Json,
                    "usage: dwv-independent-parity [--json] <descriptor>",
                ));
            }
        };
        (Some(OutputMode::Json), descriptor)
    } else {
        (None, first)
    };
    let error_mode = requested_mode.unwrap_or(OutputMode::Human);
    if arguments.next().is_some() {
        return Err((
            error_mode,
            "usage: dwv-independent-parity [--json] <descriptor>",
        ));
    }
    let descriptor = PathBuf::from(descriptor);
    if descriptor.as_os_str().is_empty() || descriptor.as_os_str().len() > MAX_PATH_BYTES {
        return Err((error_mode, "descriptor path is empty or over its bound"));
    }
    Ok((requested_mode, descriptor))
}

fn write_json(result: &SemanticResult) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(&result.as_json())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if bytes.len() > MAX_RESULT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bounded result exceeds its output bound",
        ));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

fn write_human(result: &SemanticResult) -> io::Result<()> {
    let mut output = format!(
        "schema: {RESULT_SCHEMA}\ncommand: {COMMAND}\noutcome: {}\ndescriptor_version: {}\nformat_claim: {FORMAT_CLAIM}\nexperimental: true\nevidence_tier: {EVIDENCE_TIER}\nprocess_status: {}\nexit_code: {}\nreason: {}\n",
        result.status.as_str(),
        result.descriptor_version.unwrap_or("unknown"),
        result.status.as_str(),
        result.status.exit_code(),
        result.reason,
    );
    for non_claim in NON_CLAIMS {
        output.push_str("non_claim: ");
        output.push_str(non_claim);
        output.push('\n');
    }
    for region in &result.regions {
        output.push_str(&format!(
            "region: offset={} length={} disposition={} reason={}\n",
            region.range.offset,
            region.range.length,
            region.disposition.as_str(),
            region.reason,
        ));
    }
    if output.len() > MAX_RESULT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bounded result exceeds its output bound",
        ));
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(output.as_bytes())?;
    stdout.flush()
}

fn render(result: &SemanticResult, mode: OutputMode) -> io::Result<()> {
    match mode {
        OutputMode::Human => write_human(result),
        OutputMode::Json => write_json(result),
    }
}
fn write_stderr(message: &str) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(message.as_bytes());
    let _ = stderr.write_all(b"\n");
    let _ = stderr.flush();
}

fn finish(result: SemanticResult, mode: OutputMode) -> ExitCode {
    match render(&result, mode) {
        Ok(()) => ExitCode::from(result.status.exit_code()),
        Err(_) => {
            write_stderr(&format!("{COMMAND}: unable to produce a bounded report"));
            ExitCode::from(OPERATIONAL_EXIT_CODE)
        }
    }
}

fn run() -> ExitCode {
    let (requested_mode, descriptor_path) = match parse_args() {
        Ok(arguments) => arguments,
        Err((mode, reason)) => return finish(SemanticResult::invalid(reason), mode),
    };
    let fallback_mode = requested_mode.unwrap_or(OutputMode::Human);
    let (result, mode) = match read_descriptor(&descriptor_path) {
        Ok(bytes) => match parse_descriptor(&bytes) {
            Ok(descriptor) => {
                let mode = requested_mode.unwrap_or(descriptor.output_mode);
                (observe(descriptor), mode)
            }
            Err(error) => (SemanticResult::invalid(error.0), fallback_mode),
        },
        Err(DescriptorLoadError::Invalid(reason)) => {
            (SemanticResult::invalid(reason), fallback_mode)
        }
        Err(DescriptorLoadError::Operational(reason)) => {
            (SemanticResult::operational(reason), fallback_mode)
        }
    };
    finish(result, mode)
}

fn main() -> ExitCode {
    run()
}
