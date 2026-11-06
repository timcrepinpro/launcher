//! Textes de l'interface. Au Jalon 2 ils passeront dans des fichiers de traduction fr/en
//! (crate `i18n`) ; ici ils sont regroupés pour ne pas traîner dans la logique.

pub const NAV: [&str; 4] = ["Bibliothèque", "Boutique", "Téléchargements", "Amis"];
pub const SEARCH: &str = "Rechercher dans tous mes jeux";
pub const ALL: &str = "Tous";
pub const ALL_GAMES: &str = "Tous les jeux";
pub const RESCAN: &str = "↻  Rescanner mes jeux";
pub const PLAY: &str = "▶  Jouer";
pub const PLAYTIME: &str = "TEMPS DE JEU";
pub const SIZE: &str = "TAILLE SUR DISQUE";
pub const RECENT: &str = "Récemment joués";
pub const INSTALLED: &str = "INSTALLÉ";
pub const RESUME: &str = "REPRENDRE";
pub const SCANNING: &str = "Scan en cours…";
pub const NO_GAME: &str = "Aucun jeu";
pub const NO_GAME_HINT: &str = "Installe un jeu Steam, Epic ou GOG puis rescanne";
pub const NO_MATCH: &str = "Aucun jeu ne correspond.";
pub const NOT_FOUND: &str = "introuvable";
pub const LAUNCHING: &str = "Lancement de";
pub const VIA: &str = "via";
pub const SPIKE: &str = "ESSAI";
pub const HELP: &str = "F2 images/s · F3 mode · F4 transition · F5 accent · ↑↓ Entrée";

pub fn games_count(n: usize) -> String {
    if n > 1 {
        format!("{n} jeux installés")
    } else {
        format!("{n} jeu installé")
    }
}
