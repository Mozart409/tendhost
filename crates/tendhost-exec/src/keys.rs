//! SSH key management and resolution

use std::env;
use std::path::PathBuf;

use tracing::{debug, warn};

/// SSH key resolution strategy
#[derive(Debug, Clone)]
pub enum KeySource {
    /// Explicit path to key file
    Path(PathBuf),
    /// Use SSH agent
    Agent,
    /// Base64-encoded key from environment
    Env(String),
}

impl KeySource {
    /// Resolve key source to a path or agent
    ///
    /// For `Env`, decodes base64 and writes to temp file
    ///
    /// # Errors
    /// Returns `KeyError` if key resolution fails (env not set, invalid base64, etc.)
    pub fn resolve(&self) -> Result<ResolvedKey, KeyError> {
        match self {
            KeySource::Path(path) => {
                validate_key_permissions(path)?;
                Ok(ResolvedKey::Path(path.clone()))
            }
            KeySource::Agent => Ok(ResolvedKey::Agent),
            KeySource::Env(var_name) => {
                let base64_key =
                    env::var(var_name).map_err(|_| KeyError::EnvNotSet(var_name.clone()))?;
                let key_data = base64_decode(&base64_key).map_err(|_| KeyError::InvalidBase64)?;

                // Write to temp file
                let temp_path = write_temp_key(&key_data)?;
                Ok(ResolvedKey::Temp(temp_path))
            }
        }
    }
}

/// Resolved key location
#[derive(Debug)]
pub enum ResolvedKey {
    /// Path to key file
    Path(PathBuf),
    /// Use SSH agent
    Agent,
    /// Temporary file (will be deleted on drop)
    Temp(PathBuf),
}

impl ResolvedKey {
    /// Get path for SSH library
    #[must_use]
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            ResolvedKey::Path(p) | ResolvedKey::Temp(p) => Some(p),
            ResolvedKey::Agent => None,
        }
    }

    /// Whether to use SSH agent
    #[must_use]
    pub fn use_agent(&self) -> bool {
        matches!(self, ResolvedKey::Agent)
    }
}

/// Key resolution errors
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("environment variable {0} not set")]
    EnvNotSet(String),

    #[error("invalid base64 encoding")]
    InvalidBase64,

    #[error("key file permissions too open: {0} (should be 600)")]
    BadPermissions(String),

    #[error("key file not found: {0}")]
    NotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

fn base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(input.trim())
}

fn validate_key_permissions(path: &PathBuf) -> Result<(), KeyError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path).map_err(KeyError::Io)?;

    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if permissions are 600 (owner read/write only)
    // mode & 0o77 checks group and other permissions
    if mode & 0o77 != 0 {
        return Err(KeyError::BadPermissions(path.display().to_string()));
    }

    Ok(())
}

fn write_temp_key(key_data: &[u8]) -> Result<PathBuf, KeyError> {
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let temp_path = std::env::temp_dir().join(format!("tendhost_ssh_key_{}", std::process::id()));

    let mut file = File::create(&temp_path)?;
    file.write_all(key_data)?;

    // Set 600 permissions
    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(&temp_path, permissions)?;

    debug!(path = %temp_path.display(), "wrote temporary SSH key");

    Ok(temp_path)
}

impl Drop for ResolvedKey {
    fn drop(&mut self) {
        if let ResolvedKey::Temp(path) = self {
            let path_clone = path.clone();
            if let Err(e) = std::fs::remove_file(&path_clone) {
                warn!(path = %path_clone.display(), error = %e, "failed to remove temp key");
            }
        }
    }
}
