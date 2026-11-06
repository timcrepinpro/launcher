//! Mesures du spike : images par seconde, temps de démarrage, temps de filtrage.

use std::time::Duration;

use launcher_core::{Covers, InstalledGame, LaunchAction, StoreId};

/// Jeux fictifs pour atteindre `n` jeux au total (test des 2 000 jeux).
pub fn fake_games(n: usize) -> Vec<InstalledGame> {
    const A: [&str; 12] = [
        "Ombre", "Royaume", "Galaxie", "Forge", "Écho", "Pixel", "Nuit", "Tempête", "Cité",
        "Abysse", "Légende", "Horizon",
    ];
    const B: [&str; 10] = [
        "perdu",
        "d'acier",
        "éternel",
        "sauvage",
        "oublié",
        "de cendre",
        "infini",
        "sombre",
        "doré",
        "lointain",
    ];
    (0..n)
        .map(|i| {
            let store = StoreId::ALL[i % 3];
            InstalledGame {
                store,
                external_id: format!("demo{i}"),
                title: format!(
                    "{} {} {}",
                    A[i % A.len()],
                    B[(i / A.len()) % B.len()],
                    i + 1
                ),
                install_path: None,
                size_bytes: Some(((i % 90) as u64 + 1) * 700_000_000),
                playtime_minutes: (i % 4 != 0).then_some((i as u64 * 37) % 9000),
                last_played: None,
                covers: Covers::default(),
                launch: LaunchAction::Uri { uri: String::new() },
            }
        })
        .collect()
}

#[derive(Debug, Default)]
pub struct Report {
    pub games: usize,
    pub startup: Option<Duration>,
    pub scan: Option<Duration>,
    pub frames: Vec<Duration>,
    pub filter: Option<Duration>,
    pub transition_frames: Vec<Duration>,
}

fn pct(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

fn fps_block(name: &str, frames: &[Duration]) -> String {
    if frames.is_empty() {
        return format!("{name} : pas de mesure\n");
    }
    let mut s = frames.to_vec();
    s.sort();
    let total: Duration = frames.iter().sum();
    let fps = frames.len() as f64 / total.as_secs_f64().max(1e-9);
    format!(
        "{name} : {fps:.0} images/s en moyenne sur {n} images\n  temps par image : médiane {p50}, 95 % sous {p95}, pire {max}\n",
        n = frames.len(),
        p50 = ms(pct(&s, 0.5)),
        p95 = ms(pct(&s, 0.95)),
        max = ms(pct(&s, 1.0)),
    )
}

impl Report {
    pub fn render(&self) -> String {
        let mut out =
            String::from("Mesures du spike iced — Jalon 0\n================================\n\n");
        out += &format!("Jeux dans la liste : {}\n", self.games);
        if let Some(d) = self.startup {
            out += &format!(
                "Démarrage à froid (lancement → première image) : {}\n",
                ms(d)
            );
        }
        if let Some(d) = self.scan {
            out += &format!("Scan des boutiques : {}\n", ms(d));
        }
        if let Some(d) = self.filter {
            out += &format!(
                "Filtrer les {} jeux (une frappe dans la recherche) : {}\n",
                self.games,
                ms(d)
            );
        }
        out += "\n";
        out += &fps_block("Défilement continu de la liste", &self.frames);
        out += &fps_block("Transition pixel (dissolution)", &self.transition_frames);
        out += "\nObjectifs du PROMPT : 60 images/s sur 2 000 jeux, démarrage sous 500 ms.\n";
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_games_have_unique_ids() {
        let g = fake_games(2000);
        let mut ids: Vec<_> = g.iter().map(|x| x.key()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 2000);
    }

    #[test]
    fn report_computes_fps() {
        let r = Report {
            games: 2,
            frames: vec![Duration::from_millis(10); 100],
            ..Default::default()
        };
        assert!(r.render().contains("100 images/s"));
    }
}
