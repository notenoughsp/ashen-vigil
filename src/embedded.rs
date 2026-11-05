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
pub const LICENSES: [(&str, &str); 4] = [
    ("Cinzel font by Natanael Gama (SIL Open Font License 1.1)", include_str!("../assets/licenses/cinzel-OFL.txt")),
    ("RPG Audio by Kenney (CC0)", include_str!("../assets/licenses/kenney-rpg-audio.txt")),
    ("Impact Sounds by Kenney (CC0)", include_str!("../assets/licenses/kenney-impact-sounds.txt")),
    ("Voices by Kokoro-82M, hexgrad (Apache 2.0)", include_str!("../assets/licenses/kokoro.txt")),
];

pub fn sfx(name: &str) -> Option<&'static [u8]> {
    SFX.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

/// The lords' spoken lines, recorded with Kokoro-82M (Apache-2.0) and given each lord's voice
/// by `voice::process_from` (see `tests::voice_bake`); the Hound's growls are synthesized.
/// Each with its length in seconds, echo included.
pub const VOICE: [(&str, f32, &[u8]); 31] = [
    ("bogmother_0", 2.77, include_bytes!("../assets/voice/bogmother_0.ogg")),
    ("bride_0", 2.76, include_bytes!("../assets/voice/bride_0.ogg")),
    ("bride_1", 1.30, include_bytes!("../assets/voice/bride_1.ogg")),
    ("bride_2", 2.94, include_bytes!("../assets/voice/bride_2.ogg")),
    ("bride_3", 2.65, include_bytes!("../assets/voice/bride_3.ogg")),
    ("choir_0", 3.61, include_bytes!("../assets/voice/choir_0.ogg")),
    ("choir_1", 1.98, include_bytes!("../assets/voice/choir_1.ogg")),
    ("choir_2", 2.70, include_bytes!("../assets/voice/choir_2.ogg")),
    ("choir_3", 3.22, include_bytes!("../assets/voice/choir_3.ogg")),
    ("gravewarden_0", 2.21, include_bytes!("../assets/voice/gravewarden_0.ogg")),
    ("gravewarden_1", 2.76, include_bytes!("../assets/voice/gravewarden_1.ogg")),
    ("gravewarden_2", 2.42, include_bytes!("../assets/voice/gravewarden_2.ogg")),
    ("gravewarden_3", 2.81, include_bytes!("../assets/voice/gravewarden_3.ogg")),
    ("hermit_0", 3.98, include_bytes!("../assets/voice/hermit_0.ogg")),
    ("hermit_1", 3.24, include_bytes!("../assets/voice/hermit_1.ogg")),
    ("hermit_3", 2.83, include_bytes!("../assets/voice/hermit_3.ogg")),
    ("hound_0", 3.10, include_bytes!("../assets/voice/hound_0.ogg")),
    ("hound_1", 3.31, include_bytes!("../assets/voice/hound_1.ogg")),
    ("hound_3", 2.29, include_bytes!("../assets/voice/hound_3.ogg")),
    ("matriarch_0", 4.33, include_bytes!("../assets/voice/matriarch_0.ogg")),
    ("penitent_0", 4.05, include_bytes!("../assets/voice/penitent_0.ogg")),
    ("penitent_1", 3.55, include_bytes!("../assets/voice/penitent_1.ogg")),
    ("penitent_2", 2.56, include_bytes!("../assets/voice/penitent_2.ogg")),
    ("penitent_3", 2.80, include_bytes!("../assets/voice/penitent_3.ogg")),
    ("seraph_0", 4.06, include_bytes!("../assets/voice/seraph_0.ogg")),
    ("seraph_1", 2.91, include_bytes!("../assets/voice/seraph_1.ogg")),
    ("seraph_3", 2.46, include_bytes!("../assets/voice/seraph_3.ogg")),
    ("sovereign_0", 3.68, include_bytes!("../assets/voice/sovereign_0.ogg")),
    ("sovereign_1", 3.90, include_bytes!("../assets/voice/sovereign_1.ogg")),
    ("sovereign_2", 3.85, include_bytes!("../assets/voice/sovereign_2.ogg")),
    ("sovereign_3", 2.79, include_bytes!("../assets/voice/sovereign_3.ogg")),
];

pub fn voice(name: &str) -> Option<&'static [u8]> {
    VOICE.iter().find(|(n, _, _)| *n == name).map(|(_, _, b)| *b)
}
