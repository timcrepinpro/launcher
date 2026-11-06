//! Epic : un fichier JSON `.item` par jeu dans `ProgramData\Epic\EpicGamesLauncher\Data\Manifests`.

use std::fs;
use std::path::{Path, PathBuf};

use launcher_core::{Covers, InstalledGame, LaunchAction, StoreId};
use serde::Deserialize;

use crate::StoreError;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Item {
    display_name: String,
    app_name: String,
    catalog_namespace: String,
    catalog_item_id: String,
    install_location: Option<String>,
    install_size: Option<u64>,
    #[serde(default)]
    app_categories: Vec<String>,
    #[serde(default, rename = "bIsIncompleteInstall")]
    incomplete: bool,
    main_game_app_name: Option<String>,
}

/// Le PROMPT propose `apps/<AppName>` ; la forme complète `namespace:itemId:appName`
/// est celle qu'écrit le launcher Epic lui-même (à vérifier sur une vraie machine).
pub fn launch(namespace: &str, item_id: &str, app_name: &str) -> LaunchAction {
    LaunchAction::Uri {
        uri: format!("com.epicgames.launcher://apps/{namespace}%3A{item_id}%3A{app_name}?action=launch&silent=true"),
    }
}

pub fn scan(manifests: &Path) -> Result<Vec<InstalledGame>, StoreError> {
    let entries = fs::read_dir(manifests).map_err(|source| StoreError::Io {
        path: manifests.to_path_buf(),
        source,
    })?;
    let mut games = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(item) = serde_json::from_str::<Item>(&text) else {
            continue;
        };
        let is_dlc = item
            .main_game_app_name
            .as_deref()
            .is_some_and(|m| m != item.app_name);
        let not_game =
            !item.app_categories.is_empty() && !item.app_categories.iter().any(|c| c == "games");
        if item.incomplete || is_dlc || not_game {
            continue;
        }
        games.push(InstalledGame {
            store: StoreId::Epic,
            title: item.display_name,
            install_path: item.install_location,
            size_bytes: item.install_size,
            playtime_minutes: None,
            last_played: None,
            covers: Covers::default(),
            launch: launch(
                &item.catalog_namespace,
                &item.catalog_item_id,
                &item.app_name,
            ),
            external_id: item.app_name,
        });
    }
    Ok(games)
}

pub fn detect() -> Option<PathBuf> {
    let dir = launcher_platform::program_data().join("Epic/EpicGamesLauncher/Data/Manifests");
    dir.is_dir().then_some(dir)
}
