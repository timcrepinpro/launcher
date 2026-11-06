#![allow(clippy::unwrap_used)]
use std::fs;
use std::path::{Path, PathBuf};

use launcher_core::{LaunchAction, StoreId};
use launcher_stores::{gog::GogEntry, scan, ClientStatus, Roots};

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap().flatten() {
        let to = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &to);
        } else {
            fs::copy(e.path(), &to).unwrap();
        }
    }
}

/// Copie les exemples dans un dossier temporaire et y écrit les vrais chemins.
fn fixtures() -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dst = std::env::temp_dir().join(format!("launcher-fixtures-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dst);
    copy_dir(&src, &dst);
    let lf = dst.join("steam/steamapps/libraryfolders.vdf");
    let esc = |p: PathBuf| p.to_string_lossy().replace('\\', "\\\\");
    let text = fs::read_to_string(&lf)
        .unwrap()
        .replace("STEAMDIR", &esc(dst.join("steam")))
        .replace("LIB2DIR", &esc(dst.join("lib2")));
    fs::write(lf, text).unwrap();
    dst
}

#[test]
fn scans_all_platforms() {
    let dir = fixtures();
    let roots = Roots {
        steam: Some(dir.join("steam")),
        epic_manifests: Some(dir.join("epic")),
        gog_entries: vec![
            GogEntry {
                game_id: "1423049311".into(),
                name: "Cyberpunk 2077".into(),
                path: r"C:\GOG Games\Cyberpunk 2077".into(),
                exe: Some(r"C:\GOG Games\Cyberpunk 2077\bin\x64\Cyberpunk2077.exe".into()),
            },
            GogEntry {
                game_id: "1".into(),
                name: String::new(),
                path: String::new(),
                exe: None,
            },
        ],
        gog_galaxy: None,
    };
    let res = scan(&roots);
    let titles: Vec<_> = res.games.iter().map(|g| g.title.as_str()).collect();
    // Hades et Stardew triés par dernière partie, puis les autres par ordre alphabétique.
    assert_eq!(
        titles,
        ["Hades", "Stardew Valley", "Cyberpunk 2077", "Fortnite"]
    );
    assert!(res.errors.is_empty(), "{:?}", res.errors);
    assert!(res
        .clients
        .iter()
        .all(|(_, c)| matches!(c, ClientStatus::Installed { .. })));

    let hades = &res.games[0];
    assert_eq!(hades.key(), "steam:1145360");
    assert_eq!(hades.playtime_minutes, Some(2530));
    assert_eq!(hades.last_played, Some(1762300000));
    assert_eq!(hades.size_bytes, Some(15300000000));
    assert!(hades
        .covers
        .portrait
        .as_deref()
        .unwrap_or_default()
        .ends_with("library_600x900.jpg"));
    assert!(hades
        .covers
        .hero
        .as_deref()
        .unwrap_or_default()
        .ends_with("library_hero.jpg"));
    assert_eq!(
        hades.launch,
        LaunchAction::Uri {
            uri: "steam://rungameid/1145360".into()
        }
    );

    let stardew = &res.games[1];
    assert!(stardew.install_path.as_deref().unwrap().contains("lib2"));

    let fortnite = res.games.iter().find(|g| g.store == StoreId::Epic).unwrap();
    assert_eq!(
        fortnite.launch,
        LaunchAction::Uri {
            uri: "com.epicgames.launcher://apps/fn%3A4fe75bbc5a674f4f9b356b5c90567da5%3AFortnite?action=launch&silent=true".into()
        }
    );

    let cp = res.games.iter().find(|g| g.store == StoreId::Gog).unwrap();
    assert!(
        matches!(&cp.launch, LaunchAction::Exe { path, .. } if path.ends_with("Cyberpunk2077.exe"))
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn gog_prefers_galaxy() {
    let galaxy = PathBuf::from(r"C:\GOG Galaxy\GalaxyClient.exe");
    let a = launcher_stores::gog::launch("42", r"C:\Jeu", Some(r"C:\Jeu\jeu.exe"), Some(&galaxy))
        .unwrap();
    match a {
        LaunchAction::Exe { path, args, .. } => {
            assert!(path.ends_with("GalaxyClient.exe"));
            assert_eq!(args, ["/command=runGame", "/gameId=42", r"/path=C:\Jeu"]);
        }
        _ => panic!(),
    }
}

#[test]
fn missing_platforms_are_warnings() {
    let res = scan(&Roots::default());
    assert!(res.games.is_empty());
    assert!(res
        .clients
        .iter()
        .all(|(_, c)| *c == ClientStatus::NotFound));
}
