//! Logo « feu de camp » : sprite 24 × 24 dessiné en Rust, flamme recolorée selon
//! l'accent (règles HSL du PROMPT §11). Les bûches ne changent jamais.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

pub const SPRITE: [&str; 24] = [
    "........................",
    "........................",
    "........................",
    "........................",
    "........................",
    "...............Y........",
    "........................",
    "........Y..RR...........",
    "...........RR...........",
    "...........RR...........",
    "..........ROOR..Y.......",
    ".......Y..ROOR..........",
    "..........ROOR..........",
    "....ab...ROYYOR...ca....",
    "....adbb.ROYYOR.ccca....",
    "....bcddROYYYYORcceb....",
    "......ccROYWWYORee......",
    "........ROYWWYOR........",
    "........ROYWWYOR........",
    "......cccceeccddbb......",
    "....acccee....ccddba....",
    "....acee........ccda....",
    "....be............cb....",
    "........................",
];

pub const TILE: Color = Color::from_rgb8(0x1F, 0x13, 0x10);

/// Couleurs de la flamme pour un accent : (R, O, Y, W).
pub fn flame(accent: Color) -> [Color; 4] {
    let (h, s, l) = to_hsl(accent);
    let shade =
        |dh: f32, dl: f32| from_hsl((h + dh).rem_euclid(360.0), s, (l + dl).clamp(0.10, 0.95));
    [
        shade(-12.0, -0.10),
        accent,
        shade(8.0, 0.17),
        shade(10.0, 0.38),
    ]
}

fn log(c: char) -> Option<Color> {
    let rgb = |r, g, b| Some(Color::from_rgb8(r, g, b));
    match c {
        'a' => rgb(0xFF, 0xD9, 0xA0),
        'b' => rgb(0xB0, 0x7A, 0x52),
        'c' => rgb(0x7A, 0x4A, 0x26),
        'd' => rgb(0xB5, 0x73, 0x3F),
        'e' => rgb(0x43, 0x25, 0x0F),
        _ => None,
    }
}

pub fn to_hsl(c: Color) -> (f32, f32, f32) {
    let (r, g, b) = (c.r, c.g, c.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s, l)
}

pub fn from_hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgb(
        (r + m).clamp(0.0, 1.0),
        (g + m).clamp(0.0, 1.0),
        (b + m).clamp(0.0, 1.0),
    )
}

/// Le logo comme widget canvas.
pub struct Logo {
    pub accent: Color,
    /// Rayon de la tuile, en fraction de sa taille (26/120 en normal, 0 en pixel carré).
    pub corner: f32,
}

impl<Message> canvas::Program<Message> for Logo {
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
        let side = bounds.width.min(bounds.height);
        let tile = Path::rounded_rectangle(
            Point::ORIGIN,
            Size::new(side, side),
            (side * self.corner).into(),
        );
        frame.fill(&tile, TILE);
        let [r, o, y, w] = flame(self.accent);
        // Viewbox 120 : marge 12, échelle 0,8, cellule de 5 → 4 unités.
        let unit = side / 120.0;
        let cell = 4.0 * unit;
        for (row, line) in SPRITE.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let color = match ch {
                    'R' => Some(r),
                    'O' => Some(o),
                    'Y' => Some(y),
                    'W' => Some(w),
                    c => log(c),
                };
                if let Some(color) = color {
                    // Cellules arrondies au pixel entier pour un rendu net.
                    let x = (12.0 * unit + col as f32 * cell).round();
                    let yy = (12.0 * unit + row as f32 * cell).round();
                    let x2 = (12.0 * unit + (col + 1) as f32 * cell).round();
                    let y2 = (12.0 * unit + (row + 1) as f32 * cell).round();
                    frame.fill_rectangle(Point::new(x, yy), Size::new(x2 - x, y2 - yy), color);
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb8(c: Color) -> [i32; 3] {
        [
            (c.r * 255.0).round() as i32,
            (c.g * 255.0).round() as i32,
            (c.b * 255.0).round() as i32,
        ]
    }

    fn close(c: Color, hex: u32) -> bool {
        let want = [
            (hex >> 16) as i32 & 255,
            (hex >> 8) as i32 & 255,
            hex as i32 & 255,
        ];
        rgb8(c).iter().zip(want).all(|(a, b)| (a - b).abs() <= 2)
    }

    /// Les couleurs attendues sont celles des fichiers logo-orange.svg et logo-menthe.svg.
    #[test]
    fn flame_matches_design_files() {
        let [r, o, y, w] = flame(Color::from_rgb8(0xFF, 0x8A, 0x1F));
        assert!(close(r, 0xEB4100), "{:?}", rgb8(r));
        assert!(close(o, 0xFF8A1F));
        assert!(close(y, 0xFFCA76), "{:?}", rgb8(y));
        assert!(close(w, 0xFFF4E1), "{:?}", rgb8(w));

        let [r, _, y, w] = flame(Color::from_rgb8(0x5E, 0xEA, 0xD4));
        assert!(close(r, 0x31E4A4), "{:?}", rgb8(r));
        assert!(close(y, 0xABF4F2), "{:?}", rgb8(y));
        assert!(close(w, 0xE8FCFC), "{:?}", rgb8(w));
    }

    #[test]
    fn sprite_is_24_by_24() {
        assert!(SPRITE.iter().all(|l| l.chars().count() == 24));
    }
}
