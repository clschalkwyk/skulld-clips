use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

const STALE_PARTIAL_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    pub partial_files_removed: u64,
    pub cache_entries_removed: u64,
    pub failures: u64,
}

pub fn cleanup_project_storage(projects_root: &Path) -> CleanupReport {
    cleanup_project_storage_at(projects_root, SystemTime::now(), STALE_PARTIAL_AGE)
}

fn cleanup_project_storage_at(
    projects_root: &Path,
    now: SystemTime,
    stale_partial_age: Duration,
) -> CleanupReport {
    let mut report = CleanupReport::default();
    let Ok(project_entries) = fs::read_dir(projects_root) else {
        return report;
    };
    for entry in project_entries.flatten() {
        let project_dir = entry.path();
        if !entry.file_type().is_ok_and(|kind| kind.is_dir())
            || !entry
                .file_name()
                .to_str()
                .is_some_and(|name| uuid::Uuid::parse_str(name).is_ok())
        {
            continue;
        }
        cleanup_partials(
            &project_dir.join("renders/.partial"),
            now,
            stale_partial_age,
            &mut report,
        );
        cleanup_cache(&project_dir.join("cache"), &mut report);
    }
    report
}

fn cleanup_partials(
    directory: &Path,
    now: SystemTime,
    stale_age: Duration,
    report: &mut CleanupReport,
) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            report.failures += 1;
            continue;
        };
        let safe_name = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with('.') && name.ends_with(".partial.mp4"));
        let stale = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age >= stale_age);
        if metadata.file_type().is_file() && safe_name && stale {
            if fs::remove_file(path).is_ok() {
                report.partial_files_removed += 1;
            } else {
                report.failures += 1;
            }
        }
    }
}

fn cleanup_cache(directory: &Path, report: &mut CleanupReport) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            report.failures += 1;
            continue;
        };
        let result = if metadata.file_type().is_symlink() || metadata.file_type().is_file() {
            fs::remove_file(path)
        } else if metadata.file_type().is_dir() {
            fs::remove_dir_all(path)
        } else {
            Ok(())
        };
        if result.is_ok() {
            report.cache_entries_removed += 1;
        } else {
            report.failures += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use uuid::Uuid;

    use super::{cleanup_project_storage_at, CleanupReport};

    #[test]
    fn removes_only_known_app_owned_partial_and_cache_entries() {
        let root = std::env::temp_dir().join(format!("skcf-cleanup-{}", Uuid::new_v4()));
        let project = root.join(Uuid::new_v4().to_string());
        let partials = project.join("renders/.partial");
        let cache = project.join("cache");
        fs::create_dir_all(&partials).unwrap();
        fs::create_dir_all(&cache).unwrap();
        fs::write(partials.join(".clip.job.partial.mp4"), b"partial").unwrap();
        fs::write(partials.join("keep.mp4"), b"final").unwrap();
        fs::write(cache.join("thumbnail.png"), b"cache").unwrap();
        fs::write(root.join("outside.txt"), b"outside").unwrap();

        let report =
            cleanup_project_storage_at(&root, SystemTime::now(), std::time::Duration::ZERO);

        assert_eq!(
            report,
            CleanupReport {
                partial_files_removed: 1,
                cache_entries_removed: 1,
                failures: 0,
            }
        );
        assert!(partials.join("keep.mp4").is_file());
        assert!(root.join("outside.txt").is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
