//! Dynamic lighting: lights are added into a dark light map, which then multiplies the scene.

use super::*;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const FRAGMENT: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

/// Darkness where no light reaches: a cold, dim blue.
/// Moonlight: dark, a little blue, enough to make out the stones away from the fires.
const AMBIENT: Color = Color::new(0.26, 0.25, 0.33, 1.0);
/// Cute mode is a soft evening, not a dark night.
const AMBIENT_CUTE: Color = Color::new(0.72, 0.62, 0.74, 1.0);
/// Light map size as a fraction of the screen, per quality level; it is blurry anyway.
const SCALES: [f32; 3] = [0.25, 0.5, 1.0];

pub struct Lighting {
    target: RenderTarget,
    size: (u32, u32),
    add: Material,
    multiply: Material,
    falloff: Texture2D,
    scale: f32,
}

fn material(blend: BlendState) -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        MaterialParams { pipeline_params: PipelineParams { color_blend: Some(blend), ..Default::default() }, ..Default::default() },
    )
    .ok()
}

fn light_size(scale: f32) -> (u32, u32) {
    (((screen_width() * scale) as u32).max(16), ((screen_height() * scale) as u32).max(16))
}

fn make_target(size: (u32, u32)) -> RenderTarget {
    let t = render_target(size.0, size.1);
    t.texture.set_filter(FilterMode::Linear);
    t
}

impl Lighting {
    /// None when the shaders can't be built; the game then simply runs unlit.
    pub fn new() -> Option<Lighting> {
        let add = material(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One))?;
        let multiply = material(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::DestinationColor), BlendFactor::Zero))?;
        // Premultiplied radial falloff: brightness lives in the color channels.
        let n = 128u16;
        let mut bytes = Vec::with_capacity(n as usize * n as usize * 4);
        for y in 0..n {
            for x in 0..n {
                let dx = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let dy = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let d2 = (dx * dx + dy * dy).min(1.0);
                // Soft core, quick fade at the rim.
                let v = ((1.0 - d2) * (1.0 - d2) * 255.0) as u8;
                bytes.extend_from_slice(&[v, v, v, v]);
            }
        }
        let falloff = Texture2D::from_rgba8(n, n, &bytes);
        falloff.set_filter(FilterMode::Linear);
        let scale = SCALES[1];
        let size = light_size(scale);
        Some(Lighting { target: make_target(size), size, add, multiply, falloff, scale })
    }

    /// Recreates the light map when the window size or quality changes.
    pub fn fit_screen(&mut self, quality: u8) {
        self.scale = SCALES[(quality as usize).min(2)];
        let size = light_size(self.scale);
        if size != self.size {
            self.target = make_target(size);
            self.size = size;
        }
    }
}

/// A light: position, radius and color (brightness is in the color).
type Light = (Vec2, f32, Color);

fn tint(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, 1.0)
}

impl World {
    fn collect_lights(&self) -> Vec<Light> {
        let mut out: Vec<Light> = Vec::with_capacity(512);
        let flicker = 0.92 + 0.08 * (self.time * 11.0).sin() * (self.time * 7.3).cos();
        for h in &self.heroes {
            if h.downed {
                out.push((h.p.pos, 90.0, Color::new(0.5, 0.1, 0.08, 1.0)));
            } else {
                let warm = if self.cute { Color::new(1.0, 0.8, 0.9, 1.0) } else { Color::new(1.0, 0.82, 0.62, 1.0) };
                let reach = if self.cursed_by(Curse::DarkHour) { 0.55 } else { 1.0 } * if h.class == Class::LanternWarden { 1.35 } else { 1.0 };
                out.push((h.p.pos, 400.0 * flicker * reach, warm));
                out.push((h.p.pos, 170.0, Color::new(0.45, 0.38, 0.3, 1.0)));
                if h.buff(Buff::Fury) > 0.0 {
                    out.push((h.p.pos, 120.0, Color::new(0.6, 0.05, 0.05, 1.0)));
                }
                if h.buff(Buff::Aegis) > 0.0 {
                    out.push((h.p.pos, 140.0, Color::new(0.6, 0.5, 0.25, 1.0)));
                }
            }
            for (p, _) in h.lantern_positions() {
                out.push((p, 110.0, Color::new(0.25, 0.5, 0.75, 1.0)));
            }
        }
        if let Some(cat) = &self.cat {
            out.push((cat.pos + vec2(0.0, -10.0), 26.0, Color::new(0.5, 0.45, 0.1, 1.0)));
        }
        for b in self.braziers.iter().filter(|b| b.burn > 0.0) {
            // Dims in its last ten seconds.
            let k = (b.burn / 10.0).min(1.0);
            out.push((b.pos + vec2(0.0, -18.0), 380.0 * flicker * (0.5 + 0.5 * k), Color::new(1.0, 0.6, 0.25, 1.0)));
        }
        for s in &self.shrines {
            let k = if s.used { 0.5 } else { 1.0 };
            out.push((s.pos, 420.0 * flicker, tint(Color::new(1.0, 0.55, 0.22, 1.0), k)));
        }
        if let Some(r) = &self.remnant {
            out.push((r.pos, 150.0, Color::new(0.2, 0.7, 0.55, 1.0)));
        }
        if let Some(c) = &self.cursed {
            out.push((c.pos, if c.curse > 0.0 { 320.0 } else { 220.0 }, Color::new(0.45, 0.15, 0.6, 1.0)));
        }
        if let Some(m) = &self.merchant {
            out.push((m.pos + vec2(22.0, -25.0), 260.0 * flicker, Color::new(0.85, 0.6, 0.3, 1.0)));
        }
        if let Some(g) = &self.hourglass {
            out.push((g.pos, 220.0, Color::new(0.55, 0.72, 1.0, 1.0)));
        }
        for r in self.runes.iter().filter(|r| r.ready()) {
            out.push((r.pos, 140.0, Color::new(0.3, 0.8, 0.75, 1.0)));
        }
        if let Some(g) = &self.gate {
            out.push((g.pos - vec2(0.0, 30.0), 260.0, Color::new(0.45, 0.9, 0.8, 1.0)));
        }
        if let Some(r) = &self.rift {
            out.push((r.pos - vec2(0.0, 30.0), if r.sealed > 0.0 { 340.0 } else { 220.0 }, Color::new(0.55, 0.35, 0.95, 1.0)));
        }
        if let Some(g) = &self.portal {
            out.push((g.pos - vec2(0.0, 26.0), 260.0, Color::new(0.95, 0.65, 0.45, 1.0)));
        }
        for a in &self.altars {
            out.push((a.pos, 230.0 * flicker, Color::new(0.85, 0.12, 0.1, 1.0)));
        }
        if let Some(v) = self.vault.as_ref().filter(|v| !v.open) {
            out.push((v.pos, if self.vault_key { 260.0 } else { 150.0 }, Color::new(0.85, 0.65, 0.3, 1.0)));
        }
        for p in self.pickups.iter().filter(|p| matches!(p.kind, PickKind::CursedCoffer)) {
            out.push((p.pos, 200.0, Color::new(0.55, 0.2, 0.85, 1.0)));
        }
        if self.biome == Biome::Cathedral && !self.cute {
            out.extend(self.window_light());
        }
        for p in &self.pools {
            let k = (p.life / p.max_life * 2.0).min(1.0) * 0.6;
            let (c, reach) = match p.kind {
                PoolKind::Hex => (Color::new(0.6, 0.15, 0.8, 1.0), 2.6),
                PoolKind::Holy => (Color::new(0.45, 0.65, 0.9, 1.0), 2.2),
                PoolKind::Candle => (Color::new(1.0, 0.65, 0.3, 1.0), 9.0),
                PoolKind::Smoke | PoolKind::Blood => continue,
                PoolKind::Fire => (Color::new(1.0, 0.45, 0.12, 1.0), 2.6),
            };
            out.push((p.pos, p.r * reach, tint(c, k)));
        }
        for f in &self.fx {
            let k = f.life / f.max_life;
            match f.kind {
                FxKind::Explosion { r, color } => out.push((f.pos, r * 3.0, tint(color, k))),
                FxKind::Lightning { .. } => out.push((f.pos, 320.0, tint(Color::new(0.55, 0.65, 1.0, 1.0), k * 1.2))),
                FxKind::Ring { r, color } => out.push((f.pos, r * (1.0 - k) + 60.0, tint(color, k * 0.5))),
                FxKind::Slash { evo: true, .. } | FxKind::Sweep { evo: true, .. } => out.push((f.pos, 160.0, tint(Color::new(0.8, 0.25, 0.1, 1.0), k * 0.6))),
                _ => {}
            }
        }
        for pr in &self.projs {
            match pr.kind {
                PKind::Orb => out.push((pr.pos, pr.r * 9.0, Color::new(0.9, 0.45, 0.12, 1.0))),
                PKind::Bolt => out.push((pr.pos, 70.0, Color::new(0.25, 0.5, 0.6, 1.0))),
                PKind::Lance => out.push((pr.pos, 80.0, Color::new(0.6, 0.06, 0.06, 1.0))),
                PKind::Rune => out.push((pr.pos, 60.0, Color::new(0.7, 0.55, 0.25, 1.0))),
                PKind::Ember => out.push((pr.pos, 50.0, Color::new(0.8, 0.45, 0.15, 1.0))),
                PKind::Shard => out.push((pr.pos, 40.0, Color::new(0.3, 0.5, 0.7, 1.0))),
                _ => {}
            }
        }
        for b in self.ebullets.iter().take(250) {
            let c = self.bullet_color;
            out.push((b.pos, b.r * 4.5, Color::new(c.r * 0.65, c.g * 0.65, c.b * 0.65, 1.0)));
        }
        for p in self.pillars.iter().filter(|p| p.delay <= 0.0) {
            out.push((p.pos, p.r * 3.0, tint(Color::new(1.0, 0.4, 0.1, 1.0), (p.after / 0.35).max(0.0))));
        }
        let mut souls = 0;
        // A sleeping mimic glows like the coffer it pretends to be.
        for e in self.enemies.iter().filter(|e| e.kind == EnemyKind::Mimic(false)) {
            out.push((e.pos, 170.0, Color::new(0.8, 0.6, 0.25, 1.0)));
        }
        for pk in &self.pickups {
            match pk.kind {
                PickKind::Soul(_) if souls < 160 => {
                    souls += 1;
                    out.push((pk.pos, 30.0, Color::new(0.12, 0.2, 0.3, 1.0)));
                }
                PickKind::Coffer(_) => out.push((pk.pos, 170.0, Color::new(0.8, 0.6, 0.25, 1.0))),
                _ if pk.is_relic() => out.push((pk.pos, 130.0, Color::new(0.7, 0.2, 0.15, 1.0))),
                _ => {}
            }
        }
        for e in &self.enemies {
            match e.kind {
                EnemyKind::Boss(BossKind::Sovereign) => out.push((e.pos, 280.0, Color::new(0.9, 0.35, 0.1, 1.0))),
                EnemyKind::Boss(BossKind::Hermit) => out.push((e.pos, 140.0, tint(Color::new(0.3, 0.8, 0.5, 1.0), e.alpha))),
                EnemyKind::Boss(BossKind::Bride) => out.push((e.pos, 200.0, Color::new(0.25, 0.55, 0.6, 1.0))),
                EnemyKind::Boss(BossKind::Choir) => out.push((e.pos, 220.0, tint(Color::new(0.55, 0.4, 0.8, 1.0), e.alpha))),
                EnemyKind::Boss(BossKind::Seraph) => out.push((e.pos, 200.0, Color::new(0.7, 0.6, 0.4, 1.0))),
                EnemyKind::Banshee => out.push((e.pos, 70.0, tint(Color::new(0.4, 0.5, 0.7, 1.0), e.alpha))),
                EnemyKind::Boss(_) => out.push((e.pos, 120.0, Color::new(0.5, 0.25, 0.15, 1.0))),
                EnemyKind::Necromancer => out.push((e.pos, 120.0, Color::new(0.2, 0.6, 0.3, 1.0))),
                EnemyKind::Cultist if e.fuse > 0.0 || e.flash > 0.0 => out.push((e.pos, 110.0, Color::new(0.8, 0.25, 0.08, 1.0))),
                EnemyKind::Wraith => out.push((e.pos, 60.0, tint(Color::new(0.2, 0.35, 0.7, 1.0), e.alpha))),
                _ if e.elite => out.push((e.pos, 90.0, Color::new(0.6, 0.45, 0.15, 1.0))),
                _ if e.burn > 0.0 => out.push((e.pos, 50.0, Color::new(0.6, 0.25, 0.05, 1.0))),
                _ => {}
            }
        }
        for p in self.particles.iter().filter(|p| p.glow && p.size >= 2.5).take(200) {
            let k = p.life / p.max_life;
            out.push((p.pos, p.size * 8.0, tint(p.color, 0.35 * k)));
        }
        self.static_lights(&mut out);
        // Lights that cannot reach the screen cost fill rate for nothing.
        out.retain(|(p, r, _)| self.on_screen(*p, *r));
        out
    }

    /// Light through the cathedral's coloured windows: here and there on the floor a rose of
    /// six colours, the same spots every vigil.
    fn window_light(&self) -> Vec<(Vec2, f32, Color)> {
        let cell = 760.0;
        let half = super::view_size() * 0.5 + vec2(cell, cell);
        let (x0, x1) = (((self.cam.x - half.x) / cell).floor() as i32, ((self.cam.x + half.x) / cell).ceil() as i32);
        let (y0, y1) = (((self.cam.y - half.y) / cell).floor() as i32, ((self.cam.y + half.y) / cell).ceil() as i32);
        let cols = [
            Color::new(1.0, 0.3, 0.35, 1.0),
            Color::new(0.35, 0.5, 1.0, 1.0),
            Color::new(1.0, 0.85, 0.35, 1.0),
            Color::new(0.35, 0.9, 0.5, 1.0),
            Color::new(0.75, 0.4, 1.0, 1.0),
            Color::new(1.0, 0.6, 0.3, 1.0),
        ];
        let h = |x: i32, y: i32, s: u32| {
            let mut v = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663) ^ s.wrapping_mul(83492791);
            v ^= v >> 13;
            v = v.wrapping_mul(0x5bd1e995);
            (v ^ (v >> 15)) as f32 / u32::MAX as f32
        };
        let mut out = Vec::new();
        for cy in y0..y1 {
            for cx in x0..x1 {
                if h(cx, cy, 1) < 0.35 {
                    continue;
                }
                let c = vec2((cx as f32 + 0.2 + h(cx, cy, 2) * 0.6) * cell, (cy as f32 + 0.2 + h(cx, cy, 3) * 0.6) * cell);
                let turn = h(cx, cy, 4) * TAU + self.time * 0.02;
                for (k, col) in cols.iter().enumerate() {
                    let at = c + Vec2::from_angle(turn + k as f32 / 6.0 * TAU) * 85.0;
                    out.push((at, 150.0, Color::new(col.r * 0.32, col.g * 0.32, col.b * 0.32, 1.0)));
                }
                out.push((c, 120.0, Color::new(0.3, 0.28, 0.22, 1.0)));
            }
        }
        out
    }

    /// Renders the light map and multiplies it over what is on screen.
    pub(super) fn apply_lighting(&self, l: &Lighting, cam: &Camera2D) {
        let lights = self.collect_lights();
        // The scene's own camera: macroquad turns y around for a render target by itself.
        set_camera(&Camera2D { target: cam.target, zoom: cam.zoom, render_target: Some(l.target.clone()), ..Default::default() });
        // Under the Blood Moon the darkness itself turns red.
        let k = (self.blood_moon / 3.0).clamp(0.0, 1.0).min(((MOON_LENGTH - self.blood_moon) / 3.0).clamp(0.0, 1.0));
        let field = match self.biome {
            Biome::Graveyard => AMBIENT,
            // Bright and holy: daylight through the high windows.
            Biome::Cathedral => Color::new(0.62, 0.58, 0.52, 1.0),
            // A red, close dark.
            Biome::Flesh => Color::new(0.5, 0.27, 0.27, 1.0),
        };
        let (base, moon) = if self.cute { (AMBIENT_CUTE, Color::new(0.85, 0.5, 0.7, 1.0)) } else { (field, Color::new(0.34, 0.08, 0.08, 1.0)) };
        // The mist and the grave mist darken the night.
        let murk = (self.mist / 3.0).clamp(0.0, 1.0) * 0.35 + if self.weather.is_some_and(|(w, _)| w == Weather::GraveMist) { 0.25 } else { 0.0 };
        let b = super::brightness() * (1.0 - murk);
        let mix = |a: f32, m: f32| ((a + (m - a) * k) * b).min(1.0);
        clear_background(Color::new(mix(base.r, moon.r), mix(base.g, moon.g), mix(base.b, moon.b), 1.0));
        gl_use_material(&l.add);
        for (p, r, c) in lights {
            draw_texture_ex(&l.falloff, p.x - r, p.y - r, c, DrawTextureParams { dest_size: Some(vec2(r * 2.0, r * 2.0)), ..Default::default() });
        }
        gl_use_default_material();
        set_default_camera();
        gl_use_material(&l.multiply);
        draw_texture_ex(&l.target.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(screen_width(), screen_height())), ..Default::default() });
        gl_use_default_material();
    }
}
