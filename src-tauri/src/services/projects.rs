use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    domain::{
        centered_crop, AppError, Canvas, ExportSettings, ProjectSource, ProjectV1,
        SourceFingerprint, Timeline, PROJECT_FILENAME, PROJECT_SCHEMA_VERSION,
    },
    services::probe,
};

const FINGERPRINT_CHUNK_BYTES: usize = 1024 * 1024;
const MAX_PROJECT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    Ok,
    Missing,
    Changed,
}

pub struct CreatedProject {
    pub project_path: PathBuf,
    pub project: ProjectV1,
}

pub struct LoadedProject {
    pub project_path: PathBuf,
    pub project: ProjectV1,
    pub source_status: SourceStatus,
}

pub struct SavedProject {
    pub saved_at: String,
    pub project_sha256: String,
}

pub struct RelinkedProject {
    pub project: ProjectV1,
    pub fingerprint_matched: bool,
}

pub fn create_project(
    source_path: &Path,
    projects_root: &Path,
    project_name: Option<&str>,
    resource_dir: Option<&Path>,
) -> Result<CreatedProject, AppError> {
    let source_path = fs::canonicalize(source_path).map_err(|_| AppError::source_missing())?;
    let source_filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AppError::invalid_argument("The source filename must be valid Unicode text.")
        })?;
    let name = project_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .or_else(|| source_path.file_stem().and_then(|name| name.to_str()))
        .unwrap_or("Untitled clip")
        .to_owned();

    let probe = probe::probe_media(&source_path, resource_dir)?;
    let fingerprint = fingerprint(&source_path)?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let project = ProjectV1 {
        schema_version: PROJECT_SCHEMA_VERSION,
        id: id.clone(),
        name,
        created_at: now.clone(),
        updated_at: now,
        source: ProjectSource {
            path: path_to_string(&source_path)?,
            filename: source_filename.to_owned(),
            fingerprint,
            probe: probe.clone(),
        },
        timeline: Timeline {
            in_ms: 0,
            out_ms: probe.duration_ms,
        },
        canvas: Canvas {
            width: 1080,
            height: 1920,
            background: "crop-fill".to_owned(),
        },
        crop: centered_crop(&probe),
        overlays: Vec::new(),
        export_defaults: ExportSettings::default(),
    };
    project.validate()?;

    fs::create_dir_all(projects_root).map_err(|_| project_folder_error())?;
    let project_dir = projects_root.join(&id);
    for relative in [
        "assets/images",
        "assets/captions",
        "cache",
        "renders/.partial",
        "diagnostics",
    ] {
        fs::create_dir_all(project_dir.join(relative)).map_err(|_| project_folder_error())?;
    }
    let project_path = project_dir.join(PROJECT_FILENAME);
    atomic_write_project(&project_path, &project)?;

    Ok(CreatedProject {
        project_path,
        project,
    })
}

pub fn load_project(project_path: &Path) -> Result<LoadedProject, AppError> {
    let canonical = fs::canonicalize(project_path).map_err(|_| project_open_error())?;
    let project = read_project(&canonical)?;
    validate_project_location(&canonical, &project)?;
    let source_status = source_status(&project);
    Ok(LoadedProject {
        project_path: canonical,
        project,
        source_status,
    })
}

pub fn save_project(project_path: &Path, mut project: ProjectV1) -> Result<SavedProject, AppError> {
    let canonical = fs::canonicalize(project_path).map_err(|_| project_open_error())?;
    let persisted = read_project(&canonical)?;
    validate_project_location(&canonical, &persisted)?;
    if project.id != persisted.id
        || project.created_at != persisted.created_at
        || project.source != persisted.source
        || project.schema_version != persisted.schema_version
    {
        return Err(AppError::project_schema(
            "Project identity, schema, and source metadata are native-owned.",
        ));
    }

    project.updated_at = Utc::now().to_rfc3339();
    project.validate()?;
    let bytes = serialize_project(&project)?;
    let project_sha256 = sha256_hex(&bytes);
    atomic_write_bytes(&canonical, &bytes)?;
    Ok(SavedProject {
        saved_at: project.updated_at,
        project_sha256,
    })
}

pub fn relink_source(
    project_path: &Path,
    replacement_path: &Path,
    accept_fingerprint_mismatch: bool,
    resource_dir: Option<&Path>,
) -> Result<RelinkedProject, AppError> {
    let canonical_project = fs::canonicalize(project_path).map_err(|_| project_open_error())?;
    let replacement = fs::canonicalize(replacement_path).map_err(|_| AppError::source_missing())?;
    let mut project = read_project(&canonical_project)?;
    validate_project_location(&canonical_project, &project)?;

    let replacement_fingerprint = fingerprint(&replacement)?;
    let fingerprint_matched =
        fingerprints_match(&project.source.fingerprint, &replacement_fingerprint);
    if !fingerprint_matched && !accept_fingerprint_mismatch {
        return Err(AppError::source_changed());
    }

    let replacement_probe = probe::probe_media(&replacement, resource_dir)?;
    let replacement_filename = replacement
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AppError::invalid_argument("The replacement filename must be valid Unicode text.")
        })?;
    project.source = ProjectSource {
        path: path_to_string(&replacement)?,
        filename: replacement_filename.to_owned(),
        fingerprint: replacement_fingerprint,
        probe: replacement_probe,
    };
    project.updated_at = Utc::now().to_rfc3339();
    project.validate()?;
    atomic_write_project(&canonical_project, &project)?;

    Ok(RelinkedProject {
        project,
        fingerprint_matched,
    })
}

pub fn source_status(project: &ProjectV1) -> SourceStatus {
    let path = Path::new(&project.source.path);
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => return SourceStatus::Missing,
    };
    if metadata.len() == project.source.fingerprint.size_bytes
        && modified_at_ms(&metadata) == Some(project.source.fingerprint.modified_at_ms)
    {
        return SourceStatus::Ok;
    }
    match fingerprint(path) {
        Ok(actual) if fingerprints_match(&project.source.fingerprint, &actual) => SourceStatus::Ok,
        _ => SourceStatus::Changed,
    }
}

pub fn fingerprint(path: &Path) -> Result<SourceFingerprint, AppError> {
    let mut file = File::open(path).map_err(|_| AppError::source_missing())?;
    let metadata = file.metadata().map_err(|_| {
        AppError::io(
            "The source file could not be inspected.",
            "Check that the source is readable and try again.",
        )
    })?;
    if !metadata.is_file() {
        return Err(AppError::invalid_argument(
            "The source path must be a regular file.",
        ));
    }

    let first_length = usize::try_from(metadata.len().min(FINGERPRINT_CHUNK_BYTES as u64))
        .map_err(|_| AppError::internal("The source size could not be represented."))?;
    let mut first = vec![0_u8; first_length];
    file.read_exact(&mut first).map_err(|_| {
        AppError::io(
            "The source file could not be inspected.",
            "Check that the source is readable and try again.",
        )
    })?;
    let last_chunk_sha256 = if metadata.len() > FINGERPRINT_CHUNK_BYTES as u64 {
        file.seek(SeekFrom::End(-(FINGERPRINT_CHUNK_BYTES as i64)))
            .map_err(|_| AppError::internal("The source fingerprint seek failed."))?;
        let mut last = vec![0_u8; FINGERPRINT_CHUNK_BYTES];
        file.read_exact(&mut last)
            .map_err(|_| AppError::internal("The source fingerprint read failed."))?;
        Some(sha256_hex(&last))
    } else {
        None
    };

    Ok(SourceFingerprint {
        size_bytes: metadata.len(),
        modified_at_ms: modified_at_ms(&metadata).unwrap_or_default(),
        first_chunk_sha256: sha256_hex(&first),
        last_chunk_sha256,
    })
}

pub fn fingerprints_match(expected: &SourceFingerprint, actual: &SourceFingerprint) -> bool {
    expected.size_bytes == actual.size_bytes
        && expected.first_chunk_sha256 == actual.first_chunk_sha256
        && expected.last_chunk_sha256 == actual.last_chunk_sha256
}

fn read_project(path: &Path) -> Result<ProjectV1, AppError> {
    let file = File::open(path).map_err(|_| project_open_error())?;
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| project_open_error())?;
    if bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(AppError::project_schema(
            "Project files may not exceed 4 MiB.",
        ));
    }
    let project: ProjectV1 = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::project_schema("Project JSON is malformed or incomplete."))?;
    project.validate()?;
    Ok(project)
}

fn validate_project_location(path: &Path, project: &ProjectV1) -> Result<(), AppError> {
    if path.file_name().and_then(|name| name.to_str()) != Some(PROJECT_FILENAME)
        || path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some(project.id.as_str())
    {
        return Err(AppError::project_schema(
            "The project file must remain inside its ID-named project folder.",
        ));
    }
    Ok(())
}

fn atomic_write_project(path: &Path, project: &ProjectV1) -> Result<(), AppError> {
    let bytes = serialize_project(project)?;
    atomic_write_bytes(path, &bytes)
}

fn serialize_project(project: &ProjectV1) -> Result<Vec<u8>, AppError> {
    let mut bytes = serde_json::to_vec_pretty(project)
        .map_err(|_| AppError::internal("Project serialization failed."))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::project_schema("The project path does not have a parent folder.")
    })?;
    let temporary = parent.join(format!(".project-{}.tmp", Uuid::new_v4()));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        replace_with_backup(path, &temporary)?;
        sync_directory(parent);
        Ok::<(), std::io::Error>(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result.map_err(|_| {
        AppError::io(
            "The project could not be saved safely.",
            "Check available disk space and folder permissions, then retry.",
        )
    })
}

#[cfg(not(windows))]
fn replace_with_backup(path: &Path, temporary: &Path) -> std::io::Result<()> {
    if path.exists() {
        let backup = backup_path(path);
        if fs::symlink_metadata(&backup).is_ok() {
            fs::remove_file(&backup)?;
        }
        fs::copy(path, backup)?;
    }
    fs::rename(temporary, path)
}

#[cfg(windows)]
fn replace_with_backup(path: &Path, temporary: &Path) -> std::io::Result<()> {
    use std::{iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

    if !path.exists() {
        return fs::rename(temporary, path);
    }
    let backup_path = backup_path(path);
    if fs::symlink_metadata(&backup_path).is_ok() {
        fs::remove_file(&backup_path)?;
    }
    let destination: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let replacement: Vec<u16> = temporary
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let backup: Vec<u16> = backup_path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let result = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            replacement.as_ptr(),
            backup.as_ptr(),
            REPLACEFILE_WRITE_THROUGH,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_file_name(format!("{PROJECT_FILENAME}.bak"))
}

#[cfg(unix)]
fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) {}

fn modified_at_ms(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn path_to_string(path: &Path) -> Result<String, AppError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        AppError::invalid_argument("Selected paths must be valid Unicode on this platform.")
    })
}

fn project_open_error() -> AppError {
    AppError::io(
        "The project file could not be opened.",
        "Choose an existing readable Skull’d Clip Forge project.",
    )
}

fn project_folder_error() -> AppError {
    AppError::io(
        "The project folder could not be created.",
        "Check that the application can write to its local project folder.",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::domain::{
        centered_crop, Canvas, ExportSettings, MediaProbe, ProjectSource, ProjectV1, Timeline,
        VideoProbe,
    };

    use super::{
        atomic_write_project, backup_path, fingerprint, fingerprints_match, load_project,
        save_project, source_status, SourceStatus,
    };

    #[test]
    fn fingerprint_uses_first_and_last_chunks_without_buffering_the_file() {
        let directory =
            std::env::temp_dir().join(format!("skcf-fingerprint-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("source.mp4");
        fs::write(&path, vec![42_u8; 1024 * 1024 + 5]).unwrap();

        let first = fingerprint(&path).unwrap();
        let second = fingerprint(&path).unwrap();
        assert!(fingerprints_match(&first, &second));
        assert!(first.last_chunk_sha256.is_some());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn atomically_saves_backs_up_and_restores_a_project() {
        let directory = std::env::temp_dir().join(format!("skcf-project-save-{}", Uuid::new_v4()));
        let id = Uuid::new_v4().to_string();
        let project_directory = directory.join(&id);
        fs::create_dir_all(&project_directory).unwrap();
        let source_path = directory.join("source.mp4");
        fs::write(&source_path, b"fixture source bytes").unwrap();
        let source_fingerprint = fingerprint(&source_path).unwrap();
        let probe = sample_probe(source_fingerprint.size_bytes);
        let now = Utc::now().to_rfc3339();
        let mut project = ProjectV1 {
            schema_version: 1,
            id,
            name: "First name".to_owned(),
            created_at: now.clone(),
            updated_at: now,
            source: ProjectSource {
                path: source_path.to_string_lossy().into_owned(),
                filename: "source.mp4".to_owned(),
                fingerprint: source_fingerprint,
                probe: probe.clone(),
            },
            timeline: Timeline {
                in_ms: 0,
                out_ms: probe.duration_ms,
            },
            canvas: Canvas {
                width: 1080,
                height: 1920,
                background: "crop-fill".to_owned(),
            },
            crop: centered_crop(&probe),
            overlays: Vec::new(),
            export_defaults: ExportSettings::default(),
        };
        let project_path = project_directory.join("project.skcf.json");
        atomic_write_project(&project_path, &project).unwrap();

        project.name = "Saved name".to_owned();
        save_project(&project_path, project).unwrap();
        assert!(backup_path(&project_path).is_file());
        assert_eq!(
            load_project(&project_path).unwrap().project.name,
            "Saved name"
        );

        fs::write(&source_path, b"changed source bytes").unwrap();
        assert_eq!(
            source_status(&load_project(&project_path).unwrap().project),
            SourceStatus::Changed
        );
        fs::remove_file(&source_path).unwrap();
        assert_eq!(
            source_status(&load_project(&project_path).unwrap().project),
            SourceStatus::Missing
        );

        fs::remove_dir_all(directory).unwrap();
    }

    fn sample_probe(file_size_bytes: u64) -> MediaProbe {
        MediaProbe {
            duration_ms: 2_000,
            container_name: "mov,mp4".to_owned(),
            file_size_bytes,
            video: VideoProbe {
                stream_index: 0,
                codec: "h264".to_owned(),
                raw_width: 1280,
                raw_height: 720,
                display_width: 1280,
                display_height: 720,
                rotation_degrees: 0,
                avg_frame_rate: Some(30.0),
                real_frame_rate: Some(30.0),
                pixel_format: Some("yuv420p".to_owned()),
                sample_aspect_ratio: Some("1:1".to_owned()),
            },
            has_audio: false,
            audio: None,
            warnings: vec!["Source has no audio stream; silent export is supported.".to_owned()],
        }
    }
}
