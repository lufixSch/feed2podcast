use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use eyre::{Context, eyre};
use fs_extra::dir::get_size;
use glob::glob;
use poem::{Error, Result};
use reqwest::StatusCode;
use url::Url;

const DEMO_DIR: &str = "demo";

/// Convert URL string to posix path
fn url_to_path(url: &str) -> eyre::Result<String> {
    match Url::parse(url) {
        Ok(url) => {
            Ok(String::from(url.host_str().ok_or(eyre!("Got URL without host!"))?) + url.path())
        }
        Err(_) => Ok(String::from(url)),
    }
}

/// Generates podcast path for a given podcast episode.
/// Creates missing directories
pub fn get_podcast_path(cache_dir: &str, url: &str, uid: &str, voice: &str) -> Result<PathBuf> {
    let url_path = url_to_path(url).map_err(|e| {
        Error::from_string(
            format!("Unable to create cache path from feed URL: {e}"),
            StatusCode::BAD_REQUEST,
        )
    })?;
    let id_path = url_to_path(uid).map_err(|e| {
        Error::from_string(
            format!("Unable to create cache path from article UID: {e}"),
            StatusCode::BAD_REQUEST,
        )
    })?;
    let file_dir = Path::new(&cache_dir).join(&url_path).join(&id_path);
    let audio_path = file_dir.join(format!("{voice}.mp3"));

    if !file_dir.exists() {
        create_dir_all(file_dir).map_err(|e| {
            Error::from_string(
                format!("Unable to create cache directory: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    };

    Ok(audio_path)
}

/// Different methods to cleanup cache
#[derive(Clone)]
pub enum CleanupMethod {
    /// Delete oldest item if the cache exceeds a given storage size (in byte)
    MaxStorage(u64),

    /// Delete item if the date of creation exceeds a given date
    MaxAge(Duration),

    /// No cleanup
    None,
}

/// Cleanup cache by removing unneeded elements using the given method
/// Demo directory is always ignored
pub fn run_cache_cleanup(cache_dir: &str, method: CleanupMethod) -> eyre::Result<()> {
    const LOGGING_TARGET: &str = "Cache Cleanup";

    let cache_dir = Path::new(cache_dir);
    let demo_dir = cache_dir.join(DEMO_DIR);

    if !cache_dir.exists() {
        tracing::info!(target: LOGGING_TARGET, "Skipping (No cache dir)");
        return Ok(());
    };

    match method {
        CleanupMethod::MaxStorage(size) => {
            let cache_sz =
                get_size(cache_dir).wrap_err(eyre!("Unable to get cache directory size"))?;
            let demo_sz =
                get_size(&demo_dir).wrap_err(eyre!("Unable to get demo cache directory size"))?;

            assert!(cache_sz > demo_sz); // Demo cache should always be of smaller or equal size

            let actual_sz = cache_sz - demo_sz;

            if actual_sz <= size {
                tracing::info!(target: LOGGING_TARGET, "Skipping (Max size not reached)");
                return Ok(());
            };

            tracing::info!(target: LOGGING_TARGET, "Running (Max Storage)");

            // Calculate how much space to free
            let to_free = actual_sz - size;

            // Collect all files in the cache directory except the demo directory
            let mut files = glob(&format!("{}/**", cache_dir.display()))
                .map_err(|e| eyre!("Failed to read directory: {}", e))?
                .filter_map(|p| p.ok().filter(|p| !p.starts_with(&demo_dir) || p.is_file()))
                .collect::<Vec<PathBuf>>();

            // Sort files by modification time (oldest first)
            files.sort_by_key(|p| {
                p.metadata()
                    .and_then(|meta| meta.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH)
            });

            let mut freed = 0;
            for path in files {
                if freed >= to_free {
                    break;
                }
                if let Err(e) = std::fs::remove_file(&path) {
                    tracing::error!(target: LOGGING_TARGET, "Failed to remove file {path:?}: {e}");
                    continue;
                } else {
                    tracing::info!(target: LOGGING_TARGET, "Removed file {path:?}")
                };

                freed += path.metadata().map(|meta| meta.len()).unwrap_or(0);
            }

            if freed < to_free {
                tracing::error!(target: LOGGING_TARGET, "Failed to free enough space in the cache directory");
            };

            Ok(())
        }
        CleanupMethod::MaxAge(age) => {
            // Get current system time
            let now = SystemTime::now();

            tracing::info!(target: LOGGING_TARGET, "Running (Max Age)");

            // Collect all files in the cache directory except the demo directory
            let files = glob(&format!("{}/**", cache_dir.display()))
                .map_err(|e| eyre!("Failed to read directory: {}", e))?
                .filter_map(|entry| {
                    entry
                        .ok()
                        .filter(|p| !p.starts_with(&demo_dir) && p.is_file())
                });

            for path in files {
                let metadata = match path.metadata() {
                    Ok(meta) => meta,
                    Err(e) => {
                        tracing::error!(target: LOGGING_TARGET, "Failed to get metadata for {path:?}: {e}");
                        continue;
                    }
                };

                let file_modified = match metadata.modified() {
                    Ok(modified) => modified,
                    Err(e) => {
                        tracing::error!(target: LOGGING_TARGET, "Failed to get modified time for {path:?}: {e}");
                        continue;
                    }
                };

                if now.duration_since(file_modified).unwrap_or(Duration::MAX) > age {
                    if let Err(e) = std::fs::remove_file(&path) {
                        tracing::error!(target: LOGGING_TARGET, "Failed to remove file {path:?}: {e}");
                    } else {
                        tracing::info!(target: LOGGING_TARGET, "Removed file {path:?}")
                    }
                }
            }

            Ok(())
        }
        CleanupMethod::None => Ok(()),
    }
}

pub async fn run_cache_cleanup_task(cache_dir: String, method: CleanupMethod) {
    const LOGGING_TARGET: &str = "Cache Cleanup";

    if let Err(e) = run_cache_cleanup(&cache_dir, method) {
        tracing::error!(target: LOGGING_TARGET, "{}", e);
    }
}
