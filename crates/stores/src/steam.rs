//! Steam : `libraryfolders.vdf` → bibliothèques, `appmanifest_<appid>.acf` → jeux,
//! `userdata/*/config/localconfig.vdf` → temps de jeu, `appcache/librarycache` → images.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use launcher_core::{Covers, InstalledGame, LaunchAction, StoreId};
use launcher_platform::registry::{self, Hive};

use crate::{vdf, StoreError};

/// Outils Steam qui ne sont pas des jeux.
const IGNORED_APPIDS: &[&str] = &["228980", "1070560", "1391110", "1628350", "1493710"];
const IGNORED_PREFIXES: &[&str] = &[
    "Steamworks Common",
    "Proton",
    "Steam Linux Runtime",
    "SteamVR",
];

pub fn launch(appid: &str) -> LaunchAction {
    LaunchAction::Uri {
        uri: format!("steam://rungameid/{appid}"),
    }
}

pub fn scan(steam_dir: &Path) -> Result<Vec<InstalledGame>, StoreError> {
    let stats = local_stats(steam_dir);
    let mut games = Vec::new();
    for lib in library_folders(steam_dir)? {
        let apps = lib.join("steamapps");
        let Ok(entries) = fs::read_dir(&apps) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("appmanifest_") && name.ends_with(".acf") {
                if let Some(g) = read_manifest(&entry.path(), &apps, steam_dir, &stats) {
                    games.push(g);
                }
            }
        }
    }
    Ok(games)
}

fn library_folders(steam_dir: &Path) -> Result<Vec<PathBuf>, StoreError> {
    let file = steam_dir.join("steamapps").join("libraryfolders.vdf");
    let mut out = vec![steam_dir.to_path_buf()];
    let Ok(text) = fs::read_to_string(&file) else {
        return Ok(out);
    };
    let root = vdf::parse(&text).map_err(|reason| StoreError::Parse {
        path: file.clone(),
        reason,
    })?;
    let Some(folders) = root.obj("libraryfolders") else {
        return Ok(out);
    };
    for (_, v) in folders.iter() {
        if let vdf::Value::Obj(o) = v {
            if let Some(p) = o.str("path") {
                let p = PathBuf::from(p);
                if !out.iter().any(|x| same_path(x, &p)) {
                    out.push(p);
                }
            }
        }
    }
    Ok(out)
}

fn same_path(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    n(a) == n(b)
}

#[derive(Default, Clone, Copy)]
struct Stat {
    playtime: Option<u64>,
    last_played: Option<u64>,
}

fn local_stats(steam_dir: &Path) -> HashMap<String, Stat> {
    let mut out: HashMap<String, Stat> = HashMap::new();
    let Ok(users) = fs::read_dir(steam_dir.join("userdata")) else {
        return out;
    };
    for user in users.flatten() {
        let Ok(text) = fs::read_to_string(user.path().join("config").join("localconfig.vdf"))
        else {
            continue;
        };
        let Ok(root) = vdf::parse(&text) else {
            continue;
        };
        let Some(apps) = root.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
        else {
            continue;
        };
        for (appid, v) in apps.iter() {
            let vdf::Value::Obj(o) = v else { continue };
            let num = |k: &str| {
                o.str(k)
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|n| *n > 0)
            };
            let s = out.entry(appid.clone()).or_default();
            if let Some(p) = num("Playtime") {
                s.playtime = Some(s.playtime.unwrap_or(0) + p);
            }
            if let Some(l) = num("LastPlayed") {
                s.last_played = Some(s.last_played.unwrap_or(0).max(l));
            }
        }
    }
    out
}

fn read_manifest(
    file: &Path,
    apps_dir: &Path,
    steam_dir: &Path,
    stats: &HashMap<String, Stat>,
) -> Option<InstalledGame> {
    let root = vdf::parse(&fs::read_to_string(file).ok()?).ok()?;
    let st = root.obj("AppState")?;
    let appid = st.str("appid")?.to_string();
    let title = st.str("name")?.to_string();
    if IGNORED_APPIDS.contains(&appid.as_str())
        || IGNORED_PREFIXES.iter().any(|p| title.starts_with(p))
    {
        return None;
    }
    let stat = stats.get(&appid).copied().unwrap_or_default();
    Some(InstalledGame {
        store: StoreId::Steam,
        title,
        install_path: st.str("installdir").map(|d| {
            apps_dir
                .join("common")
                .join(d)
                .to_string_lossy()
                .into_owned()
        }),
        size_bytes: st.str("SizeOnDisk").and_then(|s| s.parse().ok()),
        playtime_minutes: stat.playtime,
        last_played: stat.last_played.or_else(|| {
            st.str("LastPlayed")
                .and_then(|s| s.parse().ok())
                .filter(|n: &u64| *n > 0)
        }),
        covers: covers(steam_dir, &appid),
        launch: launch(&appid),
        external_id: appid,
    })
}

/// Ancien format : `<appid>_library_600x900.jpg` ; nouveau : `<appid>/[hash/]library_600x900.jpg`.
fn covers(steam_dir: &Path, appid: &str) -> Covers {
    let cache = steam_dir.join("appcache").join("librarycache");
    let find = |kind: &str| -> Option<String> {
        let flat = cache.join(format!("{appid}_{kind}.jpg"));
        if flat.is_file() {
            return Some(flat.to_string_lossy().into_owned());
        }
        let dir = cache.join(appid);
        let direct = dir.join(format!("{kind}.jpg"));
        if direct.is_file() {
            return Some(direct.to_string_lossy().into_owned());
        }
        fs::read_dir(&dir).ok()?.flatten().find_map(|e| {
            let f = e.path().join(format!("{kind}.jpg"));
            f.is_file().then(|| f.to_string_lossy().into_owned())
        })
    };
    Covers {
        portrait: find("library_600x900"),
        hero: find("library_hero"),
    }
}

/// Dossier de Steam : registre, puis emplacement par défaut.
pub fn detect() -> Option<PathBuf> {
    let tries = [
        (Hive::CurrentUser, r"Software\Valve\Steam", "SteamPath"),
        (
            Hive::LocalMachine,
            r"SOFTWARE\WOW6432Node\Valve\Steam",
            "InstallPath",
        ),
        (Hive::LocalMachine, r"SOFTWARE\Valve\Steam", "InstallPath"),
    ];
    let mut candidates: Vec<PathBuf> = tries
        .iter()
        .filter_map(|(h, k, v)| registry::string(*h, k, v))
        .map(|p| PathBuf::from(p.replace('/', std::path::MAIN_SEPARATOR_STR)))
        .collect();
    candidates.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    if let Some(home) = launcher_platform::home() {
        candidates.push(home.join(".steam/steam"));
        candidates.push(home.join(".local/share/Steam"));
    }
    candidates
        .into_iter()
        .find(|p| p.join("steamapps").is_dir())
}
