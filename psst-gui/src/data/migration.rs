use std::{fs, path::Path};

use directories::ProjectDirs;
use platform_dirs::AppDirs;

use crate::data::config::Config;

const APP_NAME: &str = "Psst";

pub fn perform_migration() {
    let old_app_dirs = AppDirs::new(Some(APP_NAME), false);
    let new_project_dirs = ProjectDirs::from("", "", APP_NAME);

    if let (Some(old_dirs), Some(new_dirs)) = (old_app_dirs, new_project_dirs) {
        migrate_path(
            &old_dirs.config_dir,
            new_dirs.config_dir(),
            &["config.json"],
        );
        migrate_path(
            &old_dirs.cache_dir,
            new_dirs.cache_dir(),
            &[
                "tracks",
                "track",
                "episodes",
                "episode",
                "audio",
                "keys",
                "key",
                "lyrics",
                "images",
                "artist-info",
                "related-artists",
                "album",
                "show",
                "user-info",
            ],
        );
    }

    if let Some(active_cache_dir) = Config::cache_dir() {
        rename_cache_subdirectories(&active_cache_dir);
    }
}

fn migrate_path(old_dir: &Path, new_dir: &Path, items_to_move: &[&str]) {
    if old_dir == new_dir || !old_dir.exists() {
        return;
    }

    // New path is a subdirectory of the old path (e.g. .../Psst/config vs .../Psst).
    // We must move specific items into the new subdirectory to avoid recursion.
    if new_dir.starts_with(old_dir) {
        log::info!(
            "migrating content from {:?} into subdirectory {:?}",
            old_dir,
            new_dir
        );
        if let Err(err) = fs::create_dir_all(new_dir) {
            log::error!("failed to create directory {:?}: {}", new_dir, err);
            return;
        }

        for &item in items_to_move {
            let old_item = old_dir.join(item);
            let new_item = new_dir.join(item);
            move_if_exists(&old_item, &new_item);
        }
    } else if !new_dir.exists() {
        log::info!("migrating directory from {:?} to {:?}", old_dir, new_dir);
        if let Some(parent) = new_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(err) = fs::rename(old_dir, new_dir) {
            log::error!("failed to migrate directory: {}", err);
        }
    }
}

fn rename_cache_subdirectories(cache_dir: &Path) {
    if !cache_dir.exists() {
        return;
    }

    let renames = [
        ("track", "tracks"),
        ("episode", "episodes"),
        ("show", "shows"),
        ("album", "albums"),
        ("artist", "artists"),
        ("key", "keys"),
    ];

    for (old_name, new_name) in renames {
        let old_path = cache_dir.join(old_name);
        let new_path = cache_dir.join(new_name);
        move_if_exists(&old_path, &new_path);
    }
}

fn move_if_exists(from: &Path, to: &Path) {
    if from.exists() && !to.exists() {
        log::info!("moving {:?} to {:?}", from, to);
        if let Err(err) = fs::rename(from, to) {
            log::error!("failed to move {:?}: {}", from, err);
        }
    }
}
