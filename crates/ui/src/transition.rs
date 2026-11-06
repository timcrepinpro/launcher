//! Transitions « pixel » entre deux états : une grille de 16 × 6 blocs couvre l'ancienne
//! vue puis découvre la nouvelle. L'ordre des blocs donne le style (PROMPT §11).
//! Le spike les dessine en canvas ; un shader wgpu dédié fera la même chose au Jalon 2.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

pub const COLS: usize = 16;
pub const ROWS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dissolution,
    Balayage,
    Rideau,
    Aucune,
}

impl Kind {
    pub fn next(self) -> Self {
        match self {
            Kind::Dissolution => Kind::Balayage,
            Kind::Balayage => Kind::Rideau,
            Kind::Rideau => Kind::Aucune,
            Kind::Aucune => Kind::Dissolution,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Dissolution => "Dissolution",
            Kind::Balayage => "Balayage",
            Kind::Rideau => "Rideau",
            Kind::Aucune => "Sans transition",
        }
    }
}

/// Moment d'apparition d'un bloc, entre 0 et 1.
pub fn order(kind: Kind, col: usize, row: usize) -> f32 {
    match kind {
        Kind::Balayage => col as f32 / COLS as f32,
        Kind::Rideau => row as f32 / ROWS as f32,
        _ => {
            // Aléatoire fixe : hachage entier du numéro de bloc.
            let mut x = (row * COLS + col) as u32 ^ 0x9E37_79B9;
            x ^= x >> 16;
            x = x.wrapping_mul(0x7FEB_352D);
            x ^= x >> 15;
            (x % 1000) as f32 / 1000.0
        }
    }
}

/// Le bloc est-il plein à l'avancement `p` (0 → 1) ? Il se remplit pendant la
/// première moitié et se vide pendant la seconde.
pub fn covered(kind: Kind, col: usize, row: usize, p: f32) -> bool {
    let o = order(kind, col, row);
    if p < 0.5 {
        o < p * 2.0
    } else {
        o >= (p - 0.5) * 2.0
    }
}

pub struct Blocks {
    pub kind: Kind,
    pub progress: f32,
    pub color: Color,
}

impl<Message> canvas::Program<Message> for Blocks {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width / COLS as f32;
        let h = bounds.height / ROWS as f32;
        for row in 0..ROWS {
            for col in 0..COLS {
                if covered(self.kind, col, row, self.progress) {
                    let x = (col as f32 * w).floor();
                    let y = (row as f32 * h).floor();
                    let size = Size::new(
                        ((col + 1) as f32 * w).ceil() - x,
                        ((row + 1) as f32 * h).ceil() - y,
                    );
                    frame.fill_rectangle(Point::new(x, y), size, self.color);
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_covered_at_middle_and_clear_at_ends() {
        for kind in [Kind::Dissolution, Kind::Balayage, Kind::Rideau] {
            let count = |p| {
                (0..ROWS)
                    .flat_map(|r| (0..COLS).map(move |c| (c, r)))
                    .filter(|&(c, r)| covered(kind, c, r, p))
                    .count()
            };
            assert_eq!(count(0.0), 0);
            assert_eq!(count(0.4999), COLS * ROWS - count_late(kind));
            assert_eq!(count(0.5), COLS * ROWS);
            assert_eq!(count(1.0), 0);
        }
    }

    // Blocs dont l'ordre est ≥ 0,9998 (aucun, sauf hasard) : juste pour garder le test exact.
    fn count_late(kind: Kind) -> usize {
        (0..ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (c, r)))
            .filter(|&(c, r)| order(kind, c, r) >= 0.9998)
            .count()
    }

    #[test]
    fn sweep_goes_left_to_right() {
        assert!(covered(Kind::Balayage, 0, 3, 0.1));
        assert!(!covered(Kind::Balayage, 15, 3, 0.1));
    }
}
