use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const POLICY_SCHEMA: &str = "dwv.array-policy.v1";

#[derive(Clone, Debug, serde::Serialize)]
pub struct ArrayPolicy {
    pub schema: String,
    pub array_id: [u8; 16],
    pub topology_epoch: u64,
    pub protected_length: u64,
    pub logical_block_size: u32,
    pub recovery_path: PathBuf,
    pub members: Vec<MemberPolicy>,
    pub frontend: FrontendPolicy,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MemberPolicy {
    pub path: PathBuf,
    pub role: String,
    pub expected_identity: [u8; 16],
    pub slot_id: [u8; 16],
    pub coding_position: u16,
    pub assignment_instance: [u8; 16],
    pub assignment_generation: u64,
    pub store_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum FrontendPolicy {
    None,
    LinuxUblk,
}

#[derive(Debug, Deserialize)]
struct PolicyDocument {
    schema: String,
    array_id: String,
    topology_epoch: u64,
    protected_length: u64,
    logical_block_size: u32,
    recovery: PolicyRecovery,
    members: Vec<PolicyMember>,
    frontend: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PolicyRecovery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct PolicyMember {
    path: String,
    role: String,
    expected_identity: String,
    slot_id: String,
    coding_position: u16,
    assignment_instance: String,
    assignment_generation: u64,
    store_id: u64,
}

#[derive(Debug)]
pub enum PolicyError {
    Io(String),
    Parse(String),
    Invalid(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "array policy I/O failed: {message}"),
            Self::Parse(message) => write!(formatter, "array policy is invalid JSON: {message}"),
            Self::Invalid(message) => write!(formatter, "array policy is invalid: {message}"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// dwv:req req.operator-recovery.declarative-array-policy-locates-but-does-not-authorize
pub fn load(path: &Path) -> Result<ArrayPolicy, PolicyError> {
    let root = path
        .parent()
        .ok_or_else(|| PolicyError::Invalid("array policy has no parent directory".into()))?
        .to_path_buf();
    let bytes = std::fs::read(path).map_err(|error| PolicyError::Io(error.to_string()))?;
    if bytes.len() > 1024 * 1024 {
        return Err(PolicyError::Invalid(
            "array policy exceeds the one MiB bound".into(),
        ));
    }
    let document: PolicyDocument =
        serde_json::from_slice(&bytes).map_err(|error| PolicyError::Parse(error.to_string()))?;
    if document.schema != POLICY_SCHEMA {
        return Err(PolicyError::Invalid(format!(
            "unsupported schema {}; expected {POLICY_SCHEMA}",
            document.schema
        )));
    }
    let members = document
        .members
        .iter()
        .map(|member| {
            let role = member.role.to_ascii_lowercase();
            if role != "data" && role != "parity" {
                return Err(PolicyError::Invalid(format!(
                    "member {} has unsupported role {}",
                    member.path, member.role
                )));
            }
            Ok(MemberPolicy {
                path: resolve_member_path(&root, &member.path)?,
                role,
                expected_identity: decode_identity(&member.expected_identity)?,
                slot_id: decode_identity(&member.slot_id)?,
                coding_position: member.coding_position,
                assignment_instance: decode_identity(&member.assignment_instance)?,
                assignment_generation: member.assignment_generation,
                store_id: member.store_id,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if members.is_empty() {
        return Err(PolicyError::Invalid(
            "array policy must name at least one member".into(),
        ));
    }
    let frontend = match document.frontend.as_deref() {
        None => FrontendPolicy::None,
        Some("linux-ublk") => FrontendPolicy::LinuxUblk,
        Some(frontend) => {
            return Err(PolicyError::Invalid(format!(
                "unsupported frontend {frontend}"
            )));
        }
    };
    Ok(ArrayPolicy {
        schema: POLICY_SCHEMA.into(),
        array_id: decode_identity(&document.array_id)?,
        topology_epoch: document.topology_epoch,
        protected_length: document.protected_length,
        logical_block_size: document.logical_block_size,
        recovery_path: resolve_member_path(&root, &document.recovery.path)?,
        members,
        frontend,
    })
}

fn resolve_member_path(root: &Path, path: &str) -> Result<PathBuf, PolicyError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        let root =
            std::fs::canonicalize(root).map_err(|error| PolicyError::Io(error.to_string()))?;
        Ok(root.join(path))
    }
}

fn decode_identity(value: &str) -> Result<[u8; 16], PolicyError> {
    if value.len() != 32 {
        return Err(PolicyError::Invalid(format!(
            "identity must be 32 lowercase hex characters: {value}"
        )));
    }
    let mut identity = [0_u8; 16];
    for (index, byte) in identity.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).map_err(|_| {
            PolicyError::Invalid(format!("identity contains non-hex input: {value}"))
        })?;
    }
    Ok(identity)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
