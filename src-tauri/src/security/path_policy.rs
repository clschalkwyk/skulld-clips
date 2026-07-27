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
}
