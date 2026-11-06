//! Couleurs du thème sombre (PROMPT §11), accents et modes d'affichage.

use iced::{Color, Font};

const fn hex(v: u32) -> Color {
    Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub const BG: Color = hex(0x0C0E14);
pub const SIDE: Color = hex(0x10131B);
pub const CARD: Color = hex(0x131722);
pub const CARD2: Color = hex(0x1B2030);
pub const FIELD: Color = hex(0x171B27);
pub const LINE: Color = hex(0x232A36);
pub const LINE2: Color = hex(0x2B3246);
pub const TEXT: Color = hex(0xF2F4F8);
pub const MUTED: Color = hex(0xA9B0C0);
pub const SOFT: Color = hex(0xD5DAE6);

/// Dégradés de remplacement tant qu'il n'y a pas de jaquette.
pub const COVERS: [(Color, Color); 8] = [
    (hex(0xE4572E), hex(0x4A1148)),
    (hex(0x1D4E89), hex(0x0B132B)),
    (hex(0xE76F51), hex(0x264653)),
    (hex(0x588157), hex(0x1B2E1F)),
    (hex(0x8338EC), hex(0x240046)),
    (hex(0xC77D2E), hex(0x3D2208)),
    (hex(0x2A9D8F), hex(0x10333B)),
    (hex(0xBC4749), hex(0x3A0F12)),
];

pub fn cover_colors(title: &str) -> (Color, Color) {
    let h = title
        .chars()
        .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32));
    COVERS[(h as usize) % COVERS.len()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accent {
    Orange,
    Menthe,
    CitronVert,
    Rose,
    Bleu,
}

impl Accent {
    pub const ALL: [Accent; 5] = [
        Accent::Orange,
        Accent::Menthe,
        Accent::CitronVert,
        Accent::Rose,
        Accent::Bleu,
    ];

    pub fn color(self) -> Color {
        match self {
            Accent::Orange => hex(0xFF8A1F),
            Accent::Menthe => hex(0x5EEAD4),
            Accent::CitronVert => hex(0xA3E635),
            Accent::Rose => hex(0xF472B6),
            Accent::Bleu => hex(0x93C5FD),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Accent::Orange => "Orange",
            Accent::Menthe => "Menthe",
            Accent::CitronVert => "Citron vert",
            Accent::Rose => "Rose",
            Accent::Bleu => "Bleu",
        }
    }
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|a| *a == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// Normal, pixel à coins carrés, pixel à coins arrondis en escalier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    PixelCarre,
    PixelArrondi,
}

impl Mode {
    pub fn next(self) -> Self {
        match self {
            Mode::Normal => Mode::PixelCarre,
            Mode::PixelCarre => Mode::PixelArrondi,
            Mode::PixelArrondi => Mode::Normal,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Mode::Normal => "Normal",
            Mode::PixelCarre => "Pixel carré",
            Mode::PixelArrondi => "Pixel arrondi",
        }
    }
    pub fn is_pixel(self) -> bool {
        self != Mode::Normal
    }
    /// Rayon d'un élément. En pixel arrondi : marches de 4 px (petit 1, moyen 2, grand 3).
    /// Le spike approche l'escalier par un petit rayon ; le vrai escalier demandera un
    /// rendu dédié (voir docs/adr/0001-choix-interface.md).
    pub fn radius(self, normal: f32) -> f32 {
        match self {
            Mode::Normal => normal,
            Mode::PixelCarre => 0.0,
            Mode::PixelArrondi => {
                let steps = if normal <= 10.0 {
                    1.0
                } else if normal <= 14.0 {
                    2.0
                } else {
                    3.0
                };
                steps * 4.0
            }
        }
    }
    pub fn font(self, weight: iced::font::Weight) -> Font {
        if self.is_pixel() {
            // Pixelify Sans n'a pas toutes les graisses : on garde la graisse normale.
            let _ = weight;
            Font::with_name("Pixelify Sans")
        } else {
            Font {
                weight,
                ..Font::with_name("Sora")
            }
        }
    }
    pub fn mono(self) -> Font {
        if self.is_pixel() {
            Font::with_name("Pixelify Sans")
        } else {
            Font::with_name("IBM Plex Mono")
        }
    }
}

pub const FONTS: [&[u8]; 3] = [
    include_bytes!("../assets/fonts/Sora.ttf"),
    include_bytes!("../assets/fonts/PixelifySans.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf"),
];
