//! Assets compiled into the executable, so the game runs as a single file.
//! A file in `assets/` next to the game still overrides the embedded version.

pub const FONT: &[u8] = include_bytes!("../assets/font.ttf");

pub const SFX: [(&str, &[u8]); 9] = [
    ("bell", include_bytes!("../assets/sfx/bell.ogg")),
    ("clang", include_bytes!("../assets/sfx/clang.ogg")),
    ("die", include_bytes!("../assets/sfx/die.ogg")),
    ("hit", include_bytes!("../assets/sfx/hit.ogg")),
    ("hurt", include_bytes!("../assets/sfx/hurt.ogg")),
    ("knife", include_bytes!("../assets/sfx/knife.ogg")),
    ("roll", include_bytes!("../assets/sfx/roll.ogg")),
    ("select", include_bytes!("../assets/sfx/select.ogg")),
    ("slash", include_bytes!("../assets/sfx/slash.ogg")),
];

/// License texts shown on the credits screen (the font's license requires shipping it).
pub const LICENSES: [(&str, &str); 3] = [
    ("Cinzel font by Natanael Gama (SIL Open Font License 1.1)", include_str!("../assets/licenses/cinzel-OFL.txt")),
    ("RPG Audio by Kenney (CC0)", include_str!("../assets/licenses/kenney-rpg-audio.txt")),
    ("Impact Sounds by Kenney (CC0)", include_str!("../assets/licenses/kenney-impact-sounds.txt")),
];

pub fn sfx(name: &str) -> Option<&'static [u8]> {
    SFX.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}
