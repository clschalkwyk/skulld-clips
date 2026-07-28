use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::domain::{AppError, AppErrorCode, Overlay, ProjectV1};

const LOG_FILE_BYTES: u64 = 1024 * 1024;
const LOG_FILE_COUNT: usize = 5;
const MAX_LOG_ENTRY_CHARS: usize = 512;
const MAX_DIAGNOSTIC_PROJECT_BYTES: u64 = 4 * 1024 * 1024;

pub struct Diagnostics {
    log_dir: PathBuf,
    lock: Mutex<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticBundle {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogRecord<'a> {
    timestamp: String,
    level: &'a str,
    event: &'a str,
    code: Option<AppErrorCode>,
    safe_detail: Option<String>,
}

impl Diagnostics {
    pub fn new(log_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&log_dir);
        Self {
            log_dir,
            lock: Mutex::new(()),
        }
    }

    pub fn record(&self, level: &'static str, event: &'static str, error: Option<&AppError>) {
        let safe_detail = error.and_then(|error| {
            error
                .safe_detail
                .as_deref()
                .map(|detail| sanitize_text(detail, home_directory().as_deref()))
        });
        let record = LogRecord {
            timestamp: Utc::now().to_rfc3339(),
            level,
            event,
            code: error.map(|error| error.code),
            safe_detail,
        };
        let _ = self.write_record(&record, LOG_FILE_BYTES);
    }

    pub fn create_bundle(
        &self,
        destination: &Path,
        project_path: Option<&Path>,
        runtime: &Value,
    ) -> Result<DiagnosticBundle, AppError> {
        if destination.exists() {
            return Err(AppError::output_exists());
        }
        let parent = destination.parent().ok_or_else(|| {
            AppError::destination_denied("The diagnostic destination has no parent folder.")
        })?;
        let temporary = parent.join(format!(".skcf-diagnostic-{}.partial.zip", Uuid::new_v4()));
        let result = self.write_bundle(&temporary, project_path, runtime);
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        fs::hard_link(&temporary, destination).map_err(|_| {
            let _ = fs::remove_file(&temporary);
            if destination.exists() {
                AppError::output_exists()
            } else {
                AppError::destination_denied(
                    "The diagnostic bundle could not be published in the selected folder.",
                )
            }
        })?;
        if fs::remove_file(&temporary).is_err() {
            let _ = fs::remove_file(destination);
            return Err(AppError::destination_denied(
                "The diagnostic bundle could not be finalized cleanly.",
            ));
        }
        let size_bytes = fs::metadata(destination)
            .map_err(|_| AppError::io("The diagnostic bundle could not be read.", "Try again."))?
            .len();
        Ok(DiagnosticBundle {
            path: path_to_string(destination)?,
            size_bytes,
        })
    }

    fn write_record(
        &self,
        record: &LogRecord<'_>,
        maximum_bytes: u64,
    ) -> Result<(), std::io::Error> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| std::io::Error::other("lock"))?;
        fs::create_dir_all(&self.log_dir)?;
        let current = self.log_dir.join("skcf.log");
        if fs::metadata(&current).is_ok_and(|metadata| metadata.len() >= maximum_bytes) {
            rotate_logs(&self.log_dir)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(current)?;
        serde_json::to_writer(&mut file, record)?;
        file.write_all(b"\n")
    }

    fn write_bundle(
        &self,
        temporary: &Path,
        project_path: Option<&Path>,
        runtime: &Value,
    ) -> Result<(), AppError> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(temporary)
            .map_err(|_| {
                AppError::destination_denied(
                    "The diagnostic bundle could not be created in this folder.",
                )
            })?;
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        write_json_entry(&mut archive, "runtime.json", runtime, options)?;
        write_json_entry(
            &mut archive,
            "bundle-manifest.json",
            &json!({
                "formatVersion": 1,
                "createdAt": Utc::now().to_rfc3339(),
                "includes": ["runtime metadata", "sanitized local logs", "redacted project metadata when available"],
                "excludes": ["source media", "output media", "overlay artwork", "caption text", "private paths", "source fingerprints", "asset hashes"]
            }),
            options,
        )?;
        if let Some(project_path) = project_path {
            let redacted = redacted_project_metadata(project_path);
            write_json_entry(&mut archive, "project-redacted.json", &redacted, options)?;
        }
        for index in 0..LOG_FILE_COUNT {
            let name = if index == 0 {
                "skcf.log".to_owned()
            } else {
                format!("skcf.{index}.log")
            };
            let path = self.log_dir.join(&name);
            if !path.is_file() {
                continue;
            }
            let mut bytes = Vec::new();
            File::open(&path)
                .and_then(|file| file.take(LOG_FILE_BYTES + 1).read_to_end(&mut bytes))
                .map_err(|_| {
                    AppError::io(
                        "A sanitized log could not be read.",
                        "Retry diagnostic bundle creation.",
                    )
                })?;
            bytes.truncate(LOG_FILE_BYTES as usize);
            archive
                .start_file(format!("logs/{name}"), options)
                .and_then(|_| archive.write_all(&bytes).map_err(Into::into))
                .map_err(|_| AppError::internal("A diagnostic log could not be archived."))?;
        }
        let file = archive
            .finish()
            .map_err(|_| AppError::internal("The diagnostic archive could not be finalized."))?;
        file.sync_all().map_err(|_| {
            AppError::destination_denied("The diagnostic archive could not be synced to disk.")
        })
    }
}

fn write_json_entry(
    archive: &mut ZipWriter<File>,
    name: &str,
    value: &Value,
    options: SimpleFileOptions,
) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| AppError::internal("Diagnostic metadata could not be serialized."))?;
    archive
        .start_file(name, options)
        .and_then(|_| archive.write_all(&bytes).map_err(Into::into))
        .map_err(|_| AppError::internal("Diagnostic metadata could not be archived."))
}

fn redacted_project_metadata(project_path: &Path) -> Value {
    let project = File::open(project_path)
        .and_then(|file| {
            let mut bytes = Vec::new();
            file.take(MAX_DIAGNOSTIC_PROJECT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        })
        .ok()
        .filter(|bytes| bytes.len() as u64 <= MAX_DIAGNOSTIC_PROJECT_BYTES)
        .and_then(|bytes| serde_json::from_slice::<ProjectV1>(&bytes).ok());
    let Some(project) = project else {
        return json!({
            "status": "unavailable",
            "reason": "Project metadata could not be parsed safely."
        });
    };
    let overlays: Vec<Value> = project
        .overlays
        .iter()
        .map(|overlay| {
            let base = overlay.base();
            let kind = match overlay {
                Overlay::Image { .. } => "image",
                Overlay::Caption { .. } => "caption",
                Overlay::Sting { .. } => "sting",
            };
            json!({
                "type": kind,
                "position": base.position,
                "opacity": base.opacity,
                "startMs": base.start_ms,
                "endMs": base.end_ms,
                "zIndex": base.z_index,
                "asset": {
                    "width": overlay.asset().0.width,
                    "height": overlay.asset().0.height,
                    "mimeType": overlay.asset().0.mime_type
                }
            })
        })
        .collect();
    json!({
        "status": "available",
        "schemaVersion": project.schema_version,
        "source": {
            "durationMs": project.source.probe.duration_ms,
            "containerName": project.source.probe.container_name,
            "video": project.source.probe.video,
            "hasAudio": project.source.probe.has_audio,
            "audio": project.source.probe.audio,
            "warnings": project.source.probe.warnings
        },
        "timeline": project.timeline,
        "canvas": project.canvas,
        "crop": project.crop,
        "overlays": overlays,
        "exportDefaults": project.export_defaults
    })
}

fn rotate_logs(log_dir: &Path) -> Result<(), std::io::Error> {
    let oldest = log_dir.join(format!("skcf.{}.log", LOG_FILE_COUNT - 1));
    if oldest.is_file() {
        fs::remove_file(oldest)?;
    }
    for index in (1..LOG_FILE_COUNT - 1).rev() {
        let source = log_dir.join(format!("skcf.{index}.log"));
        if source.is_file() {
            fs::rename(source, log_dir.join(format!("skcf.{}.log", index + 1)))?;
        }
    }
    let current = log_dir.join("skcf.log");
    if current.is_file() {
        fs::rename(current, log_dir.join("skcf.1.log"))?;
    }
    Ok(())
}

pub fn sanitize_text(value: &str, home: Option<&Path>) -> String {
    let home = home.and_then(Path::to_str).filter(|home| !home.is_empty());
    let sanitized = value
        .replace(['\r', '\n'], " ")
        .split_whitespace()
        .map(|token| {
            if home.is_some_and(|home| token.contains(home))
                || token.starts_with('/')
                || token.contains("/Users/")
                || token.contains("/home/")
                || token.contains(":\\")
            {
                "[redacted-path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    sanitized.chars().take(MAX_LOG_ENTRY_CHARS).collect()
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

fn path_to_string(path: &Path) -> Result<String, AppError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        AppError::destination_denied("Diagnostic paths must be valid Unicode on this platform.")
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read};

    use serde_json::json;
    use uuid::Uuid;
    use zip::ZipArchive;

    use crate::domain::AppError;

    use super::{redacted_project_metadata, sanitize_text, Diagnostics, LogRecord};

    #[test]
    fn sanitizes_private_paths_and_bounds_log_details() {
        let value = format!(
            "failed at /Users/player/Videos/private.mp4 and C:\\Users\\player\\clip.mp4 {}",
            "x".repeat(700)
        );
        let sanitized = sanitize_text(&value, Some(std::path::Path::new("/Users/player")));
        assert!(!sanitized.contains("player"));
        assert!(!sanitized.contains("private.mp4"));
        assert!(sanitized.chars().count() <= 512);
    }

    #[test]
    fn rotates_bounded_log_files() {
        let root = std::env::temp_dir().join(format!("skcf-logs-{}", Uuid::new_v4()));
        let diagnostics = Diagnostics::new(root.clone());
        for _ in 0..12 {
            diagnostics
                .write_record(
                    &LogRecord {
                        timestamp: "2026-07-27T00:00:00Z".to_owned(),
                        level: "error",
                        event: "export_failed",
                        code: None,
                        safe_detail: Some("bounded".repeat(20)),
                    },
                    128,
                )
                .unwrap();
        }
        let count = fs::read_dir(&root).unwrap().count();
        assert!(count <= 5);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_contains_only_declared_redacted_entries() {
        let root = std::env::temp_dir().join(format!("skcf-bundle-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let diagnostics = Diagnostics::new(root.join("logs"));
        diagnostics.record(
            "error",
            "export_failed",
            Some(&AppError::ffmpeg_failed(
                "Review /Users/player/private.mp4 and retry.",
            )),
        );
        let destination = root.join("diagnostic.zip");
        diagnostics
            .create_bundle(&destination, None, &json!({"appVersion": "0.1.0"}))
            .unwrap();

        let mut archive = ZipArchive::new(fs::File::open(&destination).unwrap()).unwrap();
        let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
        assert_eq!(
            names,
            vec!["runtime.json", "bundle-manifest.json", "logs/skcf.log"]
        );
        let mut log = String::new();
        archive
            .by_name("logs/skcf.log")
            .unwrap()
            .read_to_string(&mut log)
            .unwrap();
        assert!(!log.contains("/Users/player"));
        assert!(!log.contains("private.mp4"));
        drop(archive);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn redacted_project_metadata_excludes_paths_hashes_names_and_caption_text() {
        let root = std::env::temp_dir().join(format!("skcf-redaction-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("project.skcf.json");
        fs::write(
            &path,
            include_bytes!("../../../examples/example-project.skcf.json"),
        )
        .unwrap();

        let serialized = redacted_project_metadata(&path).to_string();

        assert!(serialized.contains("\"status\":\"available\""));
        assert!(!serialized.contains("/Users/"));
        assert!(!serialized.contains("firstChunkSha256"));
        assert!(!serialized.contains("relativePath"));
        assert!(!serialized.contains("CLUTCH"));
        assert!(!serialized.contains("Skull"));
        fs::remove_dir_all(root).unwrap();
    }
}
