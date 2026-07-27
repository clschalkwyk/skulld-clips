use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::domain::AppError;

#[derive(Default)]
pub struct PathPolicy {
    approved_paths: Mutex<HashSet<PathBuf>>,
    approved_outputs: Mutex<HashSet<PathBuf>>,
    approved_diagnostics: Mutex<HashSet<PathBuf>>,
}

impl PathPolicy {
    pub fn authorize_existing_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let canonical = canonical_file(path)?;
        let mut approved = self
            .approved_paths
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        approved.insert(canonical.clone());
        Ok(canonical)
    }

    pub fn authorize_existing_directory(&self, path: &Path) -> Result<PathBuf, AppError> {
        let canonical = canonical_directory(path)?;
        let mut approved = self
            .approved_paths
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        approved.insert(canonical.clone());
        Ok(canonical)
    }

    pub fn require_existing_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let canonical = canonical_file(path)?;
        let approved = self
            .approved_paths
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        if approved.contains(&canonical) {
            Ok(canonical)
        } else {
            Err(AppError::invalid_argument(
                "Choose or drop the file before using it.",
            ))
        }
    }

    pub fn require_existing_directory(&self, path: &Path) -> Result<PathBuf, AppError> {
        let canonical = canonical_directory(path)?;
        let approved = self
            .approved_paths
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        if approved.contains(&canonical) {
            Ok(canonical)
        } else {
            Err(AppError::invalid_argument(
                "Choose the folder before using it.",
            ))
        }
    }

    pub fn authorize_output_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let normalized = normalize_output_file(path)?;
        let mut approved = self
            .approved_outputs
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        approved.insert(normalized.clone());
        Ok(normalized)
    }

    pub fn require_output_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let normalized = normalize_output_file(path)?;
        let approved = self
            .approved_outputs
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        if approved.contains(&normalized) {
            Ok(normalized)
        } else {
            Err(AppError::destination_denied(
                "Choose the export destination before validating or starting.",
            ))
        }
    }

    pub fn authorize_diagnostic_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let normalized = normalize_diagnostic_file(path)?;
        let mut approved = self
            .approved_diagnostics
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        approved.insert(normalized.clone());
        Ok(normalized)
    }

    pub fn require_diagnostic_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let normalized = normalize_diagnostic_file(path)?;
        let approved = self
            .approved_diagnostics
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?;
        if approved.contains(&normalized) {
            Ok(normalized)
        } else {
            Err(AppError::destination_denied(
                "Choose the diagnostic destination before creating the bundle.",
            ))
        }
    }

    pub fn require_reveal_file(&self, path: &Path) -> Result<PathBuf, AppError> {
        let canonical = canonical_file(path)?;
        if self
            .approved_paths
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?
            .contains(&canonical)
        {
            return Ok(canonical);
        }
        let normalized = canonical
            .parent()
            .and_then(|parent| fs::canonicalize(parent).ok())
            .and_then(|parent| canonical.file_name().map(|name| parent.join(name)))
            .ok_or_else(|| {
                AppError::destination_denied("The reveal path could not be normalized.")
            })?;
        let output_approved = self
            .approved_outputs
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?
            .contains(&normalized);
        let diagnostic_approved = self
            .approved_diagnostics
            .lock()
            .map_err(|_| AppError::internal("Path authorization state is unavailable."))?
            .contains(&normalized);
        if output_approved || diagnostic_approved {
            Ok(canonical)
        } else {
            Err(AppError::destination_denied(
                "Only a selected project, export, or diagnostic file can be revealed.",
            ))
        }
    }
}

fn canonical_file(path: &Path) -> Result<PathBuf, AppError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::source_missing()
        } else {
            AppError::io(
                "The selected file could not be opened.",
                "Choose a readable local file and try again.",
            )
        }
    })?;
    if !canonical.is_file() {
        return Err(AppError::invalid_argument(
            "The selected path must be a regular file.",
        ));
    }
    Ok(canonical)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, AppError> {
    let canonical = fs::canonicalize(path).map_err(|_| {
        AppError::io(
            "The selected folder could not be opened.",
            "Choose a readable local folder and try again.",
        )
    })?;
    if !canonical.is_dir() {
        return Err(AppError::invalid_argument(
            "The selected path must be a folder.",
        ));
    }
    Ok(canonical)
}

fn normalize_output_file(path: &Path) -> Result<PathBuf, AppError> {
    normalize_new_file(path, "mp4", "Export filenames must use the .mp4 extension.")
}

fn normalize_diagnostic_file(path: &Path) -> Result<PathBuf, AppError> {
    normalize_new_file(
        path,
        "zip",
        "Diagnostic filenames must use the .zip extension.",
    )
}

fn normalize_new_file(
    path: &Path,
    expected_extension: &str,
    extension_error: &'static str,
) -> Result<PathBuf, AppError> {
    let filename = path.file_name().ok_or_else(|| {
        AppError::destination_denied("Choose a filename inside a writable local folder.")
    })?;
    let extension_matches = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected_extension));
    if !extension_matches {
        return Err(AppError::invalid_argument(extension_error));
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppError::destination_denied("The destination folder is unavailable."))?;
    let canonical_parent = canonical_directory(parent)?;
    let normalized = canonical_parent.join(filename);
    if let Ok(metadata) = fs::symlink_metadata(&normalized) {
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(AppError::destination_denied(
                "The export destination must be a regular file, not a link or folder.",
            ));
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::PathPolicy;

    #[test]
    fn rejects_a_file_until_it_is_explicitly_authorized() {
        let directory =
            std::env::temp_dir().join(format!("skcf-path-policy-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("clip.mp4");
        fs::write(&path, b"fixture").unwrap();

        let policy = PathPolicy::default();
        assert!(policy.require_existing_file(&path).is_err());
        let authorized = policy.authorize_existing_file(&path).unwrap();
        assert_eq!(policy.require_existing_file(&path).unwrap(), authorized);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn export_destinations_require_dialog_authorization_and_an_mp4_filename() {
        let directory =
            std::env::temp_dir().join(format!("skcf-output-policy-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("boss fight.mp4");
        let invalid = directory.join("boss fight.mov");

        let policy = PathPolicy::default();
        assert!(policy.require_output_file(&path).is_err());
        fs::write(directory.join("unapproved.mp4"), b"unapproved").unwrap();
        assert!(policy
            .require_reveal_file(&directory.join("unapproved.mp4"))
            .is_err());
        assert!(policy.authorize_output_file(&invalid).is_err());
        let authorized = policy.authorize_output_file(&path).unwrap();
        assert_eq!(policy.require_output_file(&path).unwrap(), authorized);
        fs::write(&path, b"verified").unwrap();
        assert_eq!(policy.require_reveal_file(&path).unwrap(), authorized);
        let diagnostics = directory.join("diagnostic.zip");
        assert!(policy.require_diagnostic_file(&diagnostics).is_err());
        let authorized_diagnostics = policy.authorize_diagnostic_file(&diagnostics).unwrap();
        assert_eq!(
            policy.require_diagnostic_file(&diagnostics).unwrap(),
            authorized_diagnostics
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
