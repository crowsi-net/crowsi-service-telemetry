use crate::{TelemetryError, error::Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Rejects aliases before a caller creates, reads, or opens owner-local state.
pub fn reject_symlink_chain(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(TelemetryError::InvalidPath(
            "owner-local path must be absolute".into(),
        ));
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
            Component::CurDir | Component::ParentDir => {
                return Err(TelemetryError::InvalidPath(
                    "owner-local path must not contain relative components".into(),
                ));
            }
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(TelemetryError::InvalidPath(format!(
                    "symbolic-link path component is forbidden: {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
