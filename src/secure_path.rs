use crate::path_chain::reject_symlink_chain;
use crate::{TelemetryError, error::Result};
use std::fs;
use std::path::Path;

const HEX: &[u8; 16] = b"0123456789abcdef";

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

pub fn prepare_database(path: &Path) -> Result<()> {
    reject_symlink_chain(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| TelemetryError::InvalidPath("database parent is absent".into()))?;
    prepare_directory(parent)?;
    reject_symlink_chain(path)?;
    if path.exists() {
        owner_only_file(path)?;
    } else {
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        drop(options.open(path)?);
    }
    Ok(())
}

pub fn read_token(path: &Path) -> Result<String> {
    reject_symlink_chain(path)?;
    owner_only_file(path)?;
    let value = fs::read_to_string(path)?.trim().to_owned();
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(TelemetryError::InvalidInput(
            "token must be exactly 256-bit lowercase hexadecimal".into(),
        ));
    }
    Ok(value)
}

pub fn generate_token(path: &Path) -> Result<()> {
    reject_symlink_chain(path)?;
    if path.exists() {
        return Err(TelemetryError::InvalidPath(
            "token path must be absolute and absent".into(),
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| TelemetryError::InvalidPath("token parent is absent".into()))?;
    prepare_directory(parent)?;
    reject_symlink_chain(path)?;
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        TelemetryError::Io(std::io::Error::other(format!(
            "random source failed: {error}"
        )))
    })?;
    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    std::io::Write::write_all(&mut options.open(path)?, encoded.as_bytes())?;
    Ok(())
}

fn prepare_directory(path: &Path) -> Result<()> {
    reject_symlink_chain(path)?;
    let created = !path.exists();
    if created {
        fs::create_dir_all(path)?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    reject_symlink_chain(path)?;
    owner_only_directory(path)
}

#[cfg(unix)]
fn owner_only_directory(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.uid() != nix::unistd::Uid::current().as_raw() {
        return Err(TelemetryError::InvalidPath(
            "directory is not owned by the runtime user".into(),
        ));
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o700 {
        return Err(TelemetryError::InvalidPath(format!(
            "{} must have mode 0700",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn owner_only_file(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.uid() != nix::unistd::Uid::current().as_raw() {
        return Err(TelemetryError::InvalidPath(
            "file is not owned by the runtime user".into(),
        ));
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o600 {
        return Err(TelemetryError::InvalidPath(format!(
            "{} must have mode 0600",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn owner_only_directory(_: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn owner_only_file(_: &Path) -> Result<()> {
    Ok(())
}
