//! Domaine pur : aucune entrée/sortie ici (règle du PROMPT §5).

use serde::{Deserialize, Serialize};

/// Boutique d'origine d'un jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StoreId {
    Steam,
    Epic,
    Gog,
}

impl StoreId {
    pub const ALL: [StoreId; 3] = [StoreId::Steam, StoreId::Epic, StoreId::Gog];

    pub fn key(self) -> &'static str {
        match self {
            StoreId::Steam => "steam",
            StoreId::Epic => "epic",
            StoreId::Gog => "gog",
        }
    }
    /// Étiquette technique affichée en IBM Plex Mono.
    pub fn label(self) -> &'static str {
        match self {
            StoreId::Steam => "STEAM",
            StoreId::Epic => "EPIC",
            StoreId::Gog => "GOG",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            StoreId::Steam => "Steam",
            StoreId::Epic => "Epic",
            StoreId::Gog => "GOG",
        }
    }
}

/// Comment démarrer un jeu : toujours via la boutique d'origine (PROMPT §2.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LaunchAction {
    /// Adresse de protocole (`steam://…`, `com.epicgames.launcher://…`).
    Uri { uri: String },
    /// Programme à démarrer (GOG Galaxy, ou l'exe du jeu en dernier recours).
    Exe {
        path: String,
        args: Vec<String>,
        cwd: Option<String>,
    },
}

/// Images locales d'un jeu, quand la boutique en garde en cache.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Covers {
    pub portrait: Option<String>,
    pub hero: Option<String>,
}

/// Un exemplaire installé d'un jeu, sur une boutique (le `copy` du schéma §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledGame {
    pub store: StoreId,
    /// Identifiant natif : appid Steam, AppName Epic, gameID GOG.
    pub external_id: String,
    pub title: String,
    pub install_path: Option<String>,
    pub size_bytes: Option<u64>,
    pub playtime_minutes: Option<u64>,
    /// Dernière partie, en secondes Unix.
    pub last_played: Option<u64>,
    pub covers: Covers,
    pub launch: LaunchAction,
}

impl InstalledGame {
    /// Identifiant unique dans le launcher : `boutique:id`.
    pub fn key(&self) -> String {
        format!("{}:{}", self.store.key(), self.external_id)
    }
}

/// Trie : joués récemment d'abord, puis par titre.
pub fn sort_library(games: &mut [InstalledGame]) {
    games.sort_by(|a, b| {
        b.last_played
            .unwrap_or(0)
            .cmp(&a.last_played.unwrap_or(0))
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
}

/// Filtre de la bibliothèque : boutique + texte. La recherche floue viendra au Jalon 1.
pub fn matches(game: &InstalledGame, store: Option<StoreId>, query_lower: &str) -> bool {
    store.is_none_or(|s| s == game.store)
        && (query_lower.is_empty() || game.title.to_lowercase().contains(query_lower))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(title: &str, store: StoreId, last: Option<u64>) -> InstalledGame {
        InstalledGame {
            store,
            external_id: title.into(),
            title: title.into(),
            install_path: None,
            size_bytes: None,
            playtime_minutes: None,
            last_played: last,
            covers: Covers::default(),
            launch: LaunchAction::Uri { uri: String::new() },
        }
    }

    #[test]
    fn sorts_recent_then_alpha() {
        let mut v = vec![
            g("b", StoreId::Gog, None),
            g("a", StoreId::Gog, None),
            g("z", StoreId::Steam, Some(5)),
        ];
        sort_library(&mut v);
        let t: Vec<_> = v.iter().map(|x| x.title.as_str()).collect();
        assert_eq!(t, ["z", "a", "b"]);
    }

    #[test]
    fn filters_by_store_and_text() {
        let x = g("Hollow Knight", StoreId::Steam, None);
        assert!(matches(&x, None, "knight"));
        assert!(matches(&x, Some(StoreId::Steam), ""));
        assert!(!matches(&x, Some(StoreId::Epic), ""));
        assert!(!matches(&x, None, "hades"));
    }
}
