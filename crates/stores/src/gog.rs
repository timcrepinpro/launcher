//! GOG : une clé de registre par jeu, `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<gameID>`.

use std::path::{Path, PathBuf};

use launcher_core::{Covers, InstalledGame, LaunchAction, StoreId};
use launcher_platform::registry::{self, Hive};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GogEntry {
    pub game_id: String,
    pub name: String,
    pub path: String,
    pub exe: Option<String>,
}

/// Via GOG Galaxy s'il est là, sinon l'exe du jeu.
pub fn launch(
    game_id: &str,
    install_path: &str,
    exe: Option<&str>,
    galaxy: Option<&Path>,
) -> Option<LaunchAction> {
    if let Some(client) = galaxy {
        return Some(LaunchAction::Exe {
            path: client.to_string_lossy().into_owned(),
            args: vec![
                "/command=runGame".into(),
                format!("/gameId={game_id}"),
                format!("/path={install_path}"),
            ],
            cwd: client.parent().map(|p| p.to_string_lossy().into_owned()),
        });
    }
    exe.map(|exe| LaunchAction::Exe {
        path: exe.to_string(),
        args: vec![],
        cwd: Some(install_path.to_string()),
    })
}

pub fn to_games(entries: &[GogEntry], galaxy: Option<&Path>) -> Vec<InstalledGame> {
    entries
        .iter()
        .filter(|e| !e.name.is_empty() && !e.path.is_empty())
        .filter_map(|e| {
            Some(InstalledGame {
                store: StoreId::Gog,
                external_id: e.game_id.clone(),
                title: e.name.clone(),
                install_path: Some(e.path.clone()),
                size_bytes: None,
                playtime_minutes: None,
                last_played: None,
                covers: Covers::default(),
                launch: launch(&e.game_id, &e.path, e.exe.as_deref(), galaxy)?,
            })
        })
        .collect()
}

pub fn detect() -> (Vec<GogEntry>, Option<PathBuf>) {
    const GAMES: &str = r"SOFTWARE\WOW6432Node\GOG.com\Games";
    let entries = registry::subkeys(Hive::LocalMachine, GAMES)
        .into_iter()
        .map(|id| {
            let key = format!(r"{GAMES}\{id}");
            let get = |n: &str| registry::string(Hive::LocalMachine, &key, n);
            GogEntry {
                game_id: get("gameID").unwrap_or(id),
                name: get("gameName").unwrap_or_default(),
                path: get("path").unwrap_or_default(),
                exe: get("exe"),
            }
        })
        .collect();
    let galaxy = registry::string(
        Hive::LocalMachine,
        r"SOFTWARE\WOW6432Node\GOG.com\GalaxyClient\paths",
        "client",
    )
    .map(|dir| PathBuf::from(dir).join("GalaxyClient.exe"))
    .filter(|p| p.is_file());
    (entries, galaxy)
}
