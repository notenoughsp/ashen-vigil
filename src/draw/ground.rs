//! The ground: stone, ruins, the dead, blood and drifting fog.

use super::*;

const TILE: f32 = 64.0;
const OUTLINE_DARK: Color = Color::new(0.02, 0.016, 0.024, 1.0);
const REGION: i32 = 6;
/// Tiles per side of a cached floor chunk, and how many chunks stay cached.
const CHUNK: i32 = 8;
const MAX_CHUNKS: usize = 48;

/// Chunk position, cute, scale, biome and field size (walls cut chunks at the edge).
type ChunkKey = (i32, i32, bool, u32, u8, u32);

thread_local! {
    /// Floor chunks drawn so far, with the frame each was last used.
    static CHUNKS: std::cell::RefCell<std::collections::HashMap<ChunkKey, (RenderTarget, u64)>> = std::cell::RefCell::new(std::collections::HashMap::new());
    static CHUNK_FRAME: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn hash(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ salt.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn hf(x: i32, y: i32, salt: u32) -> f32 {
    (hash(x, y, salt) % 10000) as f32 / 10000.0
}

/// Visible tile range around the camera, with a margin.
fn tile_range(cam: Vec2, margin: f32) -> (i32, i32, i32, i32) {
    let half = super::view_size() * 0.5 + vec2(margin, margin);
    (((cam.x - half.x) / TILE).floor() as i32, ((cam.x + half.x) / TILE).ceil() as i32, ((cam.y - half.y) / TILE).floor() as i32, ((cam.y + half.y) / TILE).ceil() as i32)
}

fn gravestone(x: f32, y: f32, tilt: f32) {
    draw_ellipse(x, y + 12.0, 12.0, 4.0, 0.0, rgba(0, 0, 0, 100));
    let c = rgba(50, 48, 52, 255);
    draw_rectangle_ex(x, y + 12.0, 16.0, 22.0, DrawRectangleParams { offset: vec2(0.5, 1.0), rotation: tilt, color: c });
    let top = vec2(x, y + 12.0) + Vec2::from_angle(tilt - std::f32::consts::FRAC_PI_2) * 22.0;
    draw_circle(top.x, top.y, 8.0, c);
    draw_line(top.x, top.y - 2.0, top.x, top.y + 12.0, 2.0, rgba(28, 26, 30, 255));
    draw_line(top.x - 4.0, top.y + 3.0, top.x + 4.0, top.y + 3.0, 2.0, rgba(28, 26, 30, 255));
}

fn dead_tree(x: f32, y: f32, seed: u32) {
    let c = rgba(24, 20, 20, 255);
    draw_ellipse(x, y + 4.0, 22.0, 7.0, 0.0, rgba(0, 0, 0, 110));
    draw_line(x, y + 4.0, x + 2.0, y - 46.0, 6.0, c);
    for k in 0..5 {
        let s = hash(seed as i32, k, 3);
        let h = 14.0 + k as f32 * 8.0;
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let len = 14.0 + (s % 14) as f32;
        let from = vec2(x + 1.0, y - h);
        let to = from + vec2(side * len, -len * 0.6 - (s % 8) as f32);
        draw_line(from.x, from.y, to.x, to.y, 2.5, c);
        draw_line(to.x, to.y, to.x + side * 6.0, to.y - 7.0, 1.5, c);
    }
}

/// A body impaled on a stake: a warning to the living.
fn impaled(x: f32, y: f32, t: f32) {
    draw_ellipse(x, y + 6.0, 12.0, 4.0, 0.0, rgba(0, 0, 0, 110));
    draw_circle(x + 3.0, y + 6.0, 9.0, rgba(60, 6, 8, 140));
    draw_line(x, y + 6.0, x, y - 44.0, 3.0, rgba(60, 46, 34, 255));
    let sway = (t * 0.7 + x * 0.01).sin() * 1.5;
    draw_ellipse(x + sway, y - 26.0, 7.0, 12.0, 0.15, rgba(58, 50, 46, 255));
    draw_circle(x + sway + 1.0, y - 40.0, 5.0, rgba(78, 70, 64, 255));
    draw_line(x + sway - 6.0, y - 30.0, x + sway - 12.0, y - 18.0, 2.0, rgba(58, 50, 46, 255));
    draw_line(x + sway + 6.0, y - 30.0, x + sway + 11.0, y - 20.0, 2.0, rgba(58, 50, 46, 255));
    draw_line(x, y - 22.0, x, y - 12.0, 2.5, rgba(110, 10, 12, 230));
}

fn skull_pile(x: f32, y: f32) {
    draw_ellipse(x, y + 4.0, 16.0, 5.0, 0.0, rgba(0, 0, 0, 100));
    for (dx, dy) in [(-7.0, 2.0), (6.0, 2.0), (0.0, -4.0), (-3.0, 5.0), (8.0, -2.0)] {
        draw_circle(x + dx, y + dy, 4.5, rgba(150, 142, 124, 255));
        draw_circle(x + dx - 1.5, y + dy, 1.1, rgba(20, 18, 16, 255));
        draw_circle(x + dx + 1.5, y + dy, 1.1, rgba(20, 18, 16, 255));
    }
}

fn cage(x: f32, y: f32, t: f32) {
    let sway = (t * 0.9 + y * 0.02).sin() * 2.0;
    draw_ellipse(x, y + 18.0, 14.0, 4.0, 0.0, rgba(0, 0, 0, 90));
    draw_line(x - 18.0, y - 36.0, x + 18.0, y - 36.0, 3.0, rgba(40, 32, 26, 255));
    draw_line(x - 16.0, y - 36.0, x - 16.0, y + 18.0, 3.0, rgba(40, 32, 26, 255));
    draw_line(x + sway, y - 36.0, x + sway, y - 26.0, 1.0, rgba(60, 60, 64, 255));
    let c = rgba(56, 56, 62, 255);
    for k in 0..5 {
        let bx = x + sway - 8.0 + k as f32 * 4.0;
        draw_line(bx, y - 26.0, bx, y - 2.0, 1.2, c);
    }
    draw_line(x + sway - 9.0, y - 26.0, x + sway + 9.0, y - 26.0, 1.5, c);
    draw_line(x + sway - 9.0, y - 2.0, x + sway + 9.0, y - 2.0, 1.5, c);
    draw_circle(x + sway, y - 8.0, 3.5, rgba(130, 122, 106, 255));
}

impl World {
    /// The floor. Stones never change, so they are drawn once per chunk of tiles into a
    /// texture and reused; only the grass that sways is drawn every frame.
    pub(super) fn draw_ground(&self, cam: &Camera2D) {
        clear_background(if self.cute { rgba(58, 40, 60, 255) } else { rgba(7, 6, 8, 255) });
        let size = TILE * CHUNK as f32;
        let half = super::view_size() * 0.5 + vec2(TILE, TILE);
        let (cx0, cx1) = (((self.cam.x - half.x) / size).floor() as i32, ((self.cam.x + half.x) / size).ceil() as i32);
        let (cy0, cy1) = (((self.cam.y - half.y) / size).floor() as i32, ((self.cam.y + half.y) / size).ceil() as i32);
        // Sharp at any resolution: chunks are drawn at the screen's pixel density.
        let scale = super::view_scale().ceil().clamp(1.0, 3.0) as u32;
        let frame = CHUNK_FRAME.with(|f| {
            f.set(f.get() + 1);
            f.get()
        });
        CHUNKS.with(|cache| {
            let mut cache = cache.borrow_mut();
            for cy in cy0..cy1 {
                for cx in cx0..cx1 {
                    let key = (cx, cy, self.cute, scale, self.biome as u8, crate::world::field_half() as u32);
                    let entry = cache.entry(key).or_insert_with(|| {
                        let rt = self.render_chunk(cx, cy, scale);
                        set_camera(cam);
                        (rt, frame)
                    });
                    entry.1 = frame;
                    draw_texture_ex(&entry.0.texture, cx as f32 * size, cy as f32 * size, WHITE, DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() });
                }
            }
            // Forget the chunks that have not been seen for the longest time.
            while cache.len() > MAX_CHUNKS {
                let oldest = cache.iter().min_by_key(|(_, (_, used))| *used).map(|(k, _)| *k);
                match oldest {
                    Some(k) => {
                        cache.remove(&k);
                    }
                    None => break,
                }
            }
        });
        self.draw_earth();
    }

    /// Draws the stones of one chunk into a fresh texture.
    fn render_chunk(&self, cx: i32, cy: i32, scale: u32) -> RenderTarget {
        let size = TILE * CHUNK as f32;
        let px = (size as u32) * scale;
        let rt = render_target(px, px);
        rt.texture.set_filter(FilterMode::Linear);
        let center = vec2((cx as f32 + 0.5) * size, (cy as f32 + 0.5) * size);
        // Macroquad turns y around for a render target by itself.
        set_camera(&Camera2D { target: center, zoom: vec2(2.0 / size, 2.0 / size), render_target: Some(rt.clone()), ..Default::default() });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        for ty in cy * CHUNK..(cy + 1) * CHUNK {
            for tx in cx * CHUNK..(cx + 1) * CHUNK {
                self.draw_tile(tx, ty);
            }
        }
        rt
    }

    /// One tile with its stones, and in cute mode its flowers.
    fn draw_tile(&self, tx: i32, ty: i32) {
        let px = tx as f32 * TILE;
        let py = ty as f32 * TILE;
        if !inside_world(vec2(px + TILE * 0.5, py + TILE * 0.5), -TILE * 0.5) {
            return;
        }
        let v = hf(tx, ty, 1);
        // Large-scale variation so the floor reads as regions, not a grid.
        let region = hf(tx.div_euclid(REGION), ty.div_euclid(REGION), 9);
        let base = 13.0 + v * 7.0 + region * 5.0;
        let red = if hf(tx.div_euclid(3), ty.div_euclid(3), 17) > 0.93 { 4.0 } else { 0.0 };
        let c = if self.cute {
            // Soft lilac and rose flagstones.
            let b = base * 3.4;
            Color::new((b + 40.0 + red * 4.0) / 255.0, (b + 18.0) / 255.0, (b + 44.0) / 255.0, 1.0)
        } else {
            match self.biome {
                Biome::Graveyard => Color::new((base + red) / 255.0, (base - 1.5) / 255.0, (base + 1.0) / 255.0, 1.0),
                // Mossy, greener stone.
                Biome::Marsh => Color::new((base - 3.0) / 255.0, (base + 2.5) / 255.0, (base - 2.0) / 255.0, 1.0),
                // Warm marble, alternately light and dark like a cathedral floor.
                Biome::Cathedral => {
                    let check = if (tx + ty).rem_euclid(2) == 0 { 6.0 } else { -2.0 };
                    Color::new((base + check + 5.0) / 255.0, (base + check - 2.0) / 255.0, (base + check - 3.0) / 255.0, 1.0)
                }
            }
        };
        match self.biome {
            Biome::Marsh if !self.cute && hash(tx, ty, 140).is_multiple_of(4) => self.water_tile(tx, ty, px, py),
            Biome::Cathedral if !self.cute => self.marble_tile(tx, ty, px, py, c),
            _ => self.flagstones(tx, ty, px, py, c),
        }
        if self.cute && hash(tx, ty, 41).is_multiple_of(4) {
            // Little flowers between the stones.
            let (fx, fy) = (px + 8.0 + hf(tx, ty, 42) * (TILE - 16.0), py + 8.0 + hf(tx, ty, 43) * (TILE - 16.0));
            let petal = [rgba(255, 190, 220, 255), rgba(255, 240, 170, 255), rgba(200, 190, 255, 255), rgba(255, 255, 255, 255)][(hash(tx, ty, 44) % 4) as usize];
            for k in 0..5 {
                let o = Vec2::from_angle(k as f32 / 5.0 * TAU) * 2.6;
                draw_circle(fx + o.x, fy + o.y, 1.9, petal);
            }
            draw_circle(fx, fy, 1.4, rgba(255, 200, 90, 255));
        }
    }

    /// Black marsh water with a sheen, a few lily pads and a muddy rim.
    fn water_tile(&self, tx: i32, ty: i32, px: f32, py: f32) {
        draw_rectangle(px, py, TILE, TILE, rgba(14, 18, 14, 255));
        draw_rectangle(px + 3.0, py + 3.0, TILE - 6.0, TILE - 6.0, rgba(8, 20, 22, 255));
        for k in 0..3 {
            let y = py + 14.0 + k as f32 * 16.0 + hf(tx, ty, 141 + k) * 6.0;
            let x = px + 8.0 + hf(tx, ty, 144 + k) * 20.0;
            draw_line(x, y, x + 14.0 + hf(tx, ty, 147 + k) * 14.0, y, 1.0, rgba(40, 70, 72, 255));
        }
        if hash(tx, ty, 150).is_multiple_of(2) {
            let (lx, ly) = (px + 14.0 + hf(tx, ty, 151) * 36.0, py + 14.0 + hf(tx, ty, 152) * 36.0);
            draw_circle(lx, ly, 6.0, rgba(34, 64, 30, 255));
            draw_triangle(vec2(lx, ly), vec2(lx + 7.0, ly - 2.0), vec2(lx + 7.0, ly + 2.0), rgba(8, 20, 22, 255));
            if hash(tx, ty, 153).is_multiple_of(3) {
                draw_circle(lx - 2.0, ly - 2.0, 1.8, rgba(200, 170, 190, 255));
            }
        }
    }

    /// A big square of polished marble with a bevel, now and then a gold-inlaid star.
    fn marble_tile(&self, tx: i32, ty: i32, px: f32, py: f32, c: Color) {
        let lift = |c: Color, d: f32| Color::new((c.r + d / 255.0).clamp(0.0, 1.0), (c.g + d / 255.0).clamp(0.0, 1.0), (c.b + d / 255.0).clamp(0.0, 1.0), 1.0);
        let (x, y, w) = (px + 1.0, py + 1.0, TILE - 2.0);
        draw_rectangle(x, y, w, w, c);
        draw_rectangle(x, y, w, 2.0, lift(c, 7.0));
        draw_rectangle(x, y + w - 2.0, w, 2.0, lift(c, -7.0));
        // Veins in the stone.
        let mut q = vec2(x + hf(tx, ty, 160) * w, y);
        for s in 0..3 {
            let n = q + vec2((hf(tx, ty, 161 + s) - 0.5) * 30.0, w / 3.0);
            draw_line(q.x, q.y, n.x, n.y, 1.0, lift(c, 4.0));
            q = n;
        }
        if hash(tx, ty, 165).is_multiple_of(9) {
            let m = vec2(px + TILE * 0.5, py + TILE * 0.5);
            let gold = rgba(70, 52, 22, 255);
            for k in 0..4 {
                let d = Vec2::from_angle(k as f32 * std::f32::consts::FRAC_PI_2) * 18.0;
                draw_line(m.x, m.y, m.x + d.x, m.y + d.y, 2.0, gold);
            }
            draw_circle_lines(m.x, m.y, 10.0, 1.5, gold);
        }
        if hash(tx, ty, 166).is_multiple_of(11) {
            // A crack running through.
            draw_line(x + 4.0, y + w * 0.3, x + w - 6.0, y + w * 0.7, 1.0, lift(c, -10.0));
        }
    }

    /// One tile of the floor: one to four bevelled slabs, each a little different, some
    /// cracked, some with moss in the joints.
    fn flagstones(&self, tx: i32, ty: i32, px: f32, py: f32, base: Color) {
        let gap = 1.5;
        let half = TILE * 0.5;
        let slabs: &[(f32, f32, f32, f32)] = match hash(tx, ty, 21) % 7 {
            0 => &[(0.0, 0.0, TILE, half), (0.0, half, TILE, half)],
            1 => &[(0.0, 0.0, half, TILE), (half, 0.0, half, TILE)],
            2 => &[(0.0, 0.0, half, half), (half, 0.0, half, half), (0.0, half, TILE, half)],
            3 => &[(0.0, 0.0, TILE * 0.4, TILE), (TILE * 0.4, 0.0, TILE * 0.6, half), (TILE * 0.4, half, TILE * 0.6, half)],
            _ => &[(0.0, 0.0, TILE, TILE)],
        };
        let k = if self.cute { 3.0 } else { 1.0 };
        let lift = |c: Color, d: f32| Color::new((c.r + d * k / 255.0).clamp(0.0, 1.0), (c.g + d * k / 255.0).clamp(0.0, 1.0), (c.b + d * k / 255.0).clamp(0.0, 1.0), 1.0);
        for (i, &(sx, sy, sw, sh)) in slabs.iter().enumerate() {
            let salt = 50 + i as u32;
            // Each slab its own tone, some a little warmer or greener.
            let tone = (hf(tx, ty, salt) - 0.5) * 6.0;
            let warm = if hash(tx, ty, salt + 10).is_multiple_of(6) { 2.5 } else { 0.0 };
            let c = lift(Color::new(base.r + warm * k / 255.0, base.g, base.b, 1.0), tone);
            let (x, y, w, h) = (px + sx + gap, py + sy + gap, sw - gap * 2.0, sh - gap * 2.0);
            draw_rectangle(x, y, w, h, c);
            // Bevel: light from above, the lower edges in shadow.
            draw_rectangle(x, y, w, 2.0, lift(c, 6.0));
            draw_rectangle(x, y, 2.0, h, lift(c, 3.5));
            draw_rectangle(x, y + h - 2.5, w, 2.5, lift(c, -6.0));
            draw_rectangle(x + w - 2.0, y, 2.0, h, lift(c, -4.0));
            // A worn hollow in the middle of big slabs.
            if w > 40.0 && h > 40.0 && hash(tx, ty, salt + 20).is_multiple_of(3) {
                draw_ellipse(x + w * 0.5, y + h * 0.55, w * 0.3, h * 0.22, 0.0, Color::new(0.0, 0.0, 0.0, 0.12));
            }
            // Cracks.
            if hash(tx, ty, salt + 30).is_multiple_of(5) {
                let mut q = vec2(x + w * hf(tx, ty, salt + 31), y + 2.0);
                let crack = lift(c, -9.0);
                for s in 0..4 {
                    let n = q + vec2((hf(tx, ty, salt + 32 + s) - 0.5) * 12.0, h * 0.24);
                    draw_line(q.x, q.y, n.x, n.y, 1.0, crack);
                    q = n;
                }
            }
            // Moss creeping out of the lower joint.
            if !self.cute && hash(tx, ty, salt + 40).is_multiple_of(7) {
                // A clump or two, not a row: random spots along the joint, bigger in the middle.
                let clump = x + w * hf(tx, ty, salt + 41);
                for m in 0..6 {
                    let off = (hf(tx, ty, salt + 42 + m) - 0.5) * 18.0;
                    let r = 3.2 - off.abs() * 0.12 + hf(tx, ty, salt + 48 + m);
                    let shade = 0.8 + hf(tx, ty, salt + 54 + m) * 0.4;
                    draw_circle(clump + off, y + h - 1.0 - hf(tx, ty, salt + 60 + m) * 3.0, r.max(1.0), Color::new(0.08 * shade, 0.13 * shade, 0.06 * shade, 0.95));
                }
            }
        }
    }

    /// Bare earth and grass breaking through the stone here and there, over several tiles.
    fn draw_earth(&self) {
        let cell = TILE * 4.0;
        let half = super::view_size() * 0.5 + vec2(cell, cell);
        let (x0, x1) = (((self.cam.x - half.x) / cell).floor() as i32, ((self.cam.x + half.x) / cell).ceil() as i32);
        let (y0, y1) = (((self.cam.y - half.y) / cell).floor() as i32, ((self.cam.y + half.y) / cell).ceil() as i32);
        let (soil, grass) = if self.cute {
            (Color::new(0.55, 0.42, 0.5, 0.35), Color::new(0.55, 0.8, 0.5, 0.8))
        } else {
            match self.biome {
                Biome::Graveyard => (Color::new(0.12, 0.085, 0.06, 0.6), Color::new(0.14, 0.2, 0.09, 0.95)),
                Biome::Marsh => (Color::new(0.08, 0.09, 0.05, 0.7), Color::new(0.16, 0.24, 0.1, 0.95)),
                // Ash and soot instead of earth, and no grass grows.
                Biome::Cathedral => (Color::new(0.09, 0.085, 0.08, 0.55), Color::new(0.0, 0.0, 0.0, 0.0)),
            }
        };
        for cy in y0..y1 {
            for cx in x0..x1 {
                if !hash(cx, cy, 70).is_multiple_of(3) {
                    continue;
                }
                let c = vec2((cx as f32 + hf(cx, cy, 71)) * cell, (cy as f32 + hf(cx, cy, 72)) * cell);
                if !inside_world(c, 60.0) {
                    continue;
                }
                // A patch of soil from a few soft blobs.
                for b in 0..4 {
                    let o = vec2(hf(cx, cy, 73 + b) - 0.5, hf(cx, cy, 77 + b) - 0.5) * 70.0;
                    let r = 26.0 + hf(cx, cy, 81 + b) * 26.0;
                    draw_ellipse(c.x + o.x, c.y + o.y, r, r * 0.7, hf(cx, cy, 85 + b), soil);
                }
                // Tufts of grass around it.
                for g in 0..9 {
                    let a = hf(cx, cy, 90 + g) * TAU;
                    let at = c + Vec2::from_angle(a) * (20.0 + hf(cx, cy, 100 + g) * 45.0);
                    for blade in -1..=1 {
                        let lean = blade as f32 * 2.5 + (self.time * 1.3 + at.x * 0.05).sin() * 0.8;
                        draw_line(at.x, at.y, at.x + lean, at.y - 6.0 - hf(cx, cy, 110 + g) * 4.0, 1.2, grass);
                    }
                }
                // Pebbles.
                for s in 0..4 {
                    let at = c + vec2(hf(cx, cy, 120 + s) - 0.5, hf(cx, cy, 125 + s) - 0.5) * 80.0;
                    draw_circle(at.x, at.y, 1.5 + hf(cx, cy, 130 + s) * 1.5, if self.cute { rgba(220, 200, 220, 255) } else { rgba(40, 37, 36, 255) });
                }
            }
        }
    }

    /// Props stand on the ground but under everything that moves.
    pub(super) fn draw_props(&self) {
        let (x0, x1, y0, y1) = tile_range(self.cam, TILE * 3.0);
        for ty in y0..y1 {
            for tx in x0..x1 {
                let px = tx as f32 * TILE;
                let py = ty as f32 * TILE;
                let d = hash(tx, ty, 2) % 1000;
                let ox = px + 12.0 + hf(tx, ty, 3) * 40.0;
                let oy = py + 12.0 + hf(tx, ty, 4) * 40.0;
                if !inside_world(vec2(ox, oy), 90.0) {
                    continue;
                }
                // The marsh and the cathedral have props of their own; candles stay everywhere.
                if self.biome != Biome::Graveyard && !(171..=176).contains(&d) {
                    self.field_prop(d, tx, ty, ox, oy);
                    continue;
                }
                match d {
                    0..=12 => gravestone(ox, oy, (hf(tx, ty, 5) - 0.5) * 0.4),
                    13..=20 => {
                        draw_circle(ox, oy, 13.0, rgba(36, 34, 38, 255));
                        draw_circle(ox, oy, 10.0, rgba(46, 44, 48, 255));
                        draw_line(ox - 6.0, oy - 3.0, ox + 4.0, oy + 5.0, 1.5, rgba(22, 20, 24, 255));
                    }
                    21..=50 => {
                        let a = hf(tx, ty, 5) * std::f32::consts::PI;
                        let dir = Vec2::from_angle(a) * 7.0;
                        draw_line(ox - dir.x, oy - dir.y, ox + dir.x, oy + dir.y, 2.0, rgba(110, 102, 88, 200));
                        draw_circle(ox + dir.x + 6.0, oy + dir.y, 3.5, rgba(120, 112, 96, 200));
                    }
                    51..=130 => {
                        let a = hf(tx, ty, 6) * TAU;
                        let mut p = vec2(ox, oy);
                        for k in 0..3 {
                            let n = p + Vec2::from_angle(a + hf(tx, ty, 10 + k) - 0.5) * 9.0;
                            draw_line(p.x, p.y, n.x, n.y, 1.0, rgba(4, 3, 5, 255));
                            p = n;
                        }
                    }
                    131..=170 => draw_circle(ox, oy, 9.0 + hf(tx, ty, 7) * 8.0, rgba(20, 28, 18, 150)),
                    171..=176 => {
                        let fl = 0.8 + 0.2 * (self.time * 9.0 + ox).sin();
                        for k in 0..4 {
                            draw_circle(ox, oy, (14.0 + k as f32 * 12.0) * fl, rgba(255, 130, 50, 6));
                        }
                        for k in 0..3 {
                            let cx = ox + (k as f32 - 1.0) * 6.0;
                            let h = 6.0 + k as f32 * 2.0;
                            draw_rectangle(cx - 1.5, oy - h, 3.0, h, rgba(190, 180, 150, 255));
                            draw_circle(cx, oy - h - 2.0, 1.8 * fl, rgba(255, 190, 90, 255));
                        }
                    }
                    177..=182 => dead_tree(ox, oy, hash(tx, ty, 8)),
                    183..=186 => impaled(ox, oy, self.time),
                    187..=192 => skull_pile(ox, oy),
                    193..=195 => cage(ox, oy, self.time),
                    196..=215 => {
                        // Old blood soaked into the stone.
                        draw_circle(ox, oy, 10.0 + hf(tx, ty, 11) * 10.0, rgba(40, 4, 6, 110));
                        draw_circle(ox + 8.0, oy + 4.0, 5.0, rgba(40, 4, 6, 110));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Props of the marsh and the cathedral.
    fn field_prop(&self, d: u32, tx: i32, ty: i32, ox: f32, oy: f32) {
        let t = self.time;
        match (self.biome, d) {
            (Biome::Marsh, 0..=40) => {
                // Reeds with cattails, swaying.
                for k in 0..6 {
                    let bx = ox + (k as f32 - 2.5) * 3.5;
                    let h = 18.0 + hf(tx, ty, 20 + k) * 16.0;
                    let lean = (t * 1.2 + bx * 0.05).sin() * 2.5;
                    draw_line(bx, oy, bx + lean, oy - h, 1.5, rgba(60, 74, 34, 255));
                    if k % 2 == 0 {
                        draw_ellipse(bx + lean, oy - h, 1.8, 4.5, 0.0, rgba(70, 46, 28, 255));
                    }
                }
            }
            (Biome::Marsh, 41..=60) => dead_tree(ox, oy, hash(tx, ty, 8)),
            (Biome::Marsh, 61..=80) => {
                // A mound of mud with a skull sinking into it.
                draw_ellipse(ox, oy, 18.0, 8.0, 0.0, rgba(30, 28, 18, 255));
                draw_ellipse(ox - 3.0, oy - 2.0, 12.0, 5.0, 0.0, rgba(40, 38, 24, 255));
                draw_circle(ox + 4.0, oy - 3.0, 4.0, rgba(140, 132, 112, 255));
                draw_circle(ox + 3.0, oy - 3.0, 1.0, OUTLINE_DARK);
                draw_circle(ox + 5.5, oy - 3.0, 1.0, OUTLINE_DARK);
            }
            (Biome::Marsh, 81..=95) => {
                // Bubbles rising from a stagnant pool.
                draw_ellipse(ox, oy, 14.0, 7.0, 0.0, rgba(14, 26, 20, 220));
                let k = (t * 0.8 + hf(tx, ty, 9) * 5.0).fract();
                draw_circle_lines(ox + 3.0, oy - k * 4.0, 1.5 + k * 3.0, 1.0, Color::new(0.4, 0.6, 0.5, 0.6 * (1.0 - k)));
            }
            (Biome::Marsh, 96..=110) => skull_pile(ox, oy),
            (Biome::Marsh, 111..=150) => {
                // Hanging moss from an old stake.
                draw_line(ox, oy + 4.0, ox, oy - 30.0, 3.0, rgba(50, 40, 30, 255));
                for k in 0..4 {
                    let x = ox - 6.0 + k as f32 * 4.0;
                    draw_line(x, oy - 30.0, x + (t + k as f32).sin(), oy - 16.0 + k as f32 * 2.0, 1.2, rgba(54, 70, 40, 255));
                }
            }
            (Biome::Cathedral, 0..=25) => {
                // A broken pillar: base, a stump of column, rubble around it.
                draw_ellipse(ox, oy + 10.0, 20.0, 6.0, 0.0, rgba(0, 0, 0, 110));
                draw_rectangle(ox - 14.0, oy, 28.0, 10.0, rgba(64, 56, 52, 255));
                let h = 20.0 + hf(tx, ty, 21) * 30.0;
                draw_rectangle(ox - 10.0, oy - h, 20.0, h, rgba(84, 74, 68, 255));
                draw_rectangle(ox - 10.0, oy - h, 5.0, h, rgba(100, 90, 82, 255));
                draw_triangle(vec2(ox - 10.0, oy - h), vec2(ox + 10.0, oy - h), vec2(ox + 2.0, oy - h - 8.0), rgba(84, 74, 68, 255));
                for k in 0..3 {
                    draw_circle(ox + 16.0 + k as f32 * 5.0, oy + 6.0 - k as f32, 3.0, rgba(70, 62, 58, 255));
                }
            }
            (Biome::Cathedral, 26..=55) => {
                // A pew, some of them overturned.
                let tilt = if hash(tx, ty, 22).is_multiple_of(3) { 0.4 } else { 0.0 };
                draw_rectangle_ex(ox, oy, 46.0, 9.0, DrawRectangleParams { offset: vec2(0.5, 0.5), rotation: tilt, color: rgba(58, 36, 24, 255) });
                draw_rectangle_ex(ox, oy - 6.0, 46.0, 4.0, DrawRectangleParams { offset: vec2(0.5, 0.5), rotation: tilt, color: rgba(76, 48, 30, 255) });
            }
            (Biome::Cathedral, 56..=70) => {
                // A torn banner on a pole.
                draw_line(ox, oy + 4.0, ox, oy - 44.0, 2.5, rgba(60, 50, 40, 255));
                let sway = (t * 1.1 + ox * 0.01).sin() * 3.0;
                draw_triangle(vec2(ox, oy - 42.0), vec2(ox + 16.0 + sway, oy - 40.0), vec2(ox, oy - 16.0), rgba(110, 20, 24, 255));
                draw_triangle(vec2(ox + 16.0 + sway, oy - 40.0), vec2(ox + 12.0 + sway, oy - 20.0), vec2(ox, oy - 16.0), rgba(90, 16, 20, 255));
                draw_circle(ox + 7.0, oy - 32.0, 2.5, rgba(190, 150, 60, 255));
            }
            (Biome::Cathedral, 71..=100) => {
                // Shards of a stained-glass window.
                let cols = [rgba(120, 30, 40, 200), rgba(40, 60, 120, 200), rgba(150, 120, 40, 200), rgba(40, 100, 60, 200)];
                for k in 0..5 {
                    let a = hf(tx, ty, 30 + k) * TAU;
                    let q = vec2(ox, oy) + Vec2::from_angle(a) * (4.0 + hf(tx, ty, 40 + k) * 14.0);
                    draw_triangle(q, q + Vec2::from_angle(a + 1.0) * 5.0, q + Vec2::from_angle(a + 2.2) * 4.0, cols[(k % 4) as usize]);
                }
            }
            (Biome::Cathedral, 101..=130) => {
                // Scorched rubble from the fire.
                draw_circle(ox, oy, 14.0 + hf(tx, ty, 11) * 10.0, rgba(12, 10, 10, 140));
                for k in 0..4 {
                    draw_circle(ox + (hf(tx, ty, 50 + k) - 0.5) * 24.0, oy + (hf(tx, ty, 54 + k) - 0.5) * 14.0, 2.5 + k as f32, rgba(60, 54, 50, 255));
                }
            }
            (Biome::Cathedral, 131..=140) => {
                // Embers still glowing in the ashes.
                let fl = 0.6 + 0.4 * (t * 3.0 + ox).sin();
                draw_circle(ox, oy, 10.0, rgba(20, 14, 12, 160));
                for k in 0..3 {
                    draw_circle(ox + (k as f32 - 1.0) * 4.0, oy + (k % 2) as f32 * 2.0, 1.6, Color::new(1.0, 0.45, 0.15, fl));
                }
            }
            _ => {}
        }
    }

    /// Blood, bodies and scorch marks, fading over their lifetime.
    pub(super) fn draw_decals(&self) {
        let half = super::view_size() * 0.5 + vec2(80.0, 80.0);
        for d in &self.decals {
            let rel = d.pos - self.cam;
            if rel.x.abs() > half.x || rel.y.abs() > half.y {
                continue;
            }
            let fade = (d.life / d.max_life * 3.0).min(1.0);
            let c = with_alpha(d.color, fade);
            match d.kind {
                DecalKind::Blood => {
                    draw_ellipse(d.pos.x, d.pos.y, d.r, d.r * 0.7, d.rot, c);
                    let n = 2 + d.seed % 3;
                    for k in 0..n {
                        let s = d.seed.wrapping_mul(k + 7);
                        let a = d.rot + (s % 628) as f32 / 100.0;
                        let off = Vec2::from_angle(a) * d.r * (0.8 + (s % 50) as f32 / 60.0);
                        draw_circle(d.pos.x + off.x, d.pos.y + off.y, d.r * 0.22, c);
                    }
                }
                DecalKind::Scorch => {
                    draw_circle(d.pos.x, d.pos.y, d.r, c);
                    draw_circle(d.pos.x, d.pos.y, d.r * 0.6, with_alpha(c, 0.6));
                }
                DecalKind::Corpse(kind) => self.draw_corpse(d, kind, fade),
            }
        }
    }

    fn draw_corpse(&self, d: &Decal, kind: EnemyKind, fade: f32) {
        let p = d.pos;
        let r = d.r;
        let body = match kind {
            EnemyKind::Ghoul => rgba(70, 82, 60, 255),
            EnemyKind::Archer => rgba(150, 144, 130, 255),
            EnemyKind::Bloat => rgba(78, 84, 50, 255),
            EnemyKind::Knight => rgba(52, 54, 62, 255),
            EnemyKind::Boss(_) => rgba(48, 40, 38, 255),
            _ => rgba(56, 50, 46, 255),
        };
        let c = with_alpha(body, fade * 0.9);
        let dir = Vec2::from_angle(d.rot);
        // Torso, head and the head torn a little away.
        draw_ellipse(p.x, p.y, r, r * 0.55, d.rot, c);
        let head = p + dir * (r * 1.15);
        draw_circle(head.x, head.y, r * 0.42, with_alpha(body, fade * 0.8));
        draw_circle(p.x + dir.x * r * 0.7, p.y + dir.y * r * 0.7, r * 0.3, with_alpha(rgba(110, 6, 8, 255), fade * 0.8));
        if d.seed.is_multiple_of(3) {
            let limb = p - dir * r * 1.4 + vec2(-dir.y, dir.x) * r * 0.6;
            draw_line(p.x, p.y, limb.x, limb.y, 2.5, c);
        }
    }

    /// Slow banks of fog drifting across the field.
    pub(super) fn draw_fog(&self) {
        let (sw, sh) = super::view_size().into();
        let t = self.time * 12.0;
        let cell = 360.0;
        let x0 = ((self.cam.x - sw) / cell).floor() as i32;
        let x1 = ((self.cam.x + sw) / cell).ceil() as i32;
        let y0 = ((self.cam.y - sh) / cell).floor() as i32;
        let y1 = ((self.cam.y + sh) / cell).ceil() as i32;
        for cy in y0..y1 {
            for cx in x0..x1 {
                let v = hf(cx, cy, 31);
                if v < 0.45 {
                    continue;
                }
                let p = vec2(cx as f32 * cell + hf(cx, cy, 32) * cell + t, cy as f32 * cell + hf(cx, cy, 33) * cell + (t * 0.3 + v * 9.0).sin() * 20.0);
                let r = 110.0 + v * 130.0;
                for k in 0..3 {
                    let rr = r * (1.0 - k as f32 * 0.25);
                    let c = if self.cute {
                        Color::new(1.0, 0.85, 0.95, 0.03)
                    } else {
                        match self.biome {
                            Biome::Graveyard => Color::new(0.5, 0.5, 0.55, 0.022),
                            Biome::Marsh => Color::new(0.4, 0.55, 0.45, 0.03),
                            Biome::Cathedral => Color::new(0.45, 0.35, 0.3, 0.025),
                        }
                    };
                    draw_circle(p.x + k as f32 * 18.0, p.y, rr, c);
                }
            }
        }
    }

    /// The wall around the field, its torches, and the abyss beyond.
    pub(super) fn draw_walls(&self) {
        let half = super::view_size() * 0.5 + vec2(200.0, 200.0);
        let (lo, hi) = (self.cam - half, self.cam + half);
        let h = crate::world::field_half();
        // The abyss: the floor just ends in mist and nothing.
        for k in 0..6 {
            let d = 40.0 + k as f32 * 40.0;
            let c = Color::new(0.05, 0.05, 0.07, 0.25 - k as f32 * 0.04);
            draw_rectangle_lines(-h - d, -h - d, (h + d) * 2.0, (h + d) * 2.0, 40.0, c);
        }
        let block = 64.0;
        let thick = 90.0;
        let face = 26.0;
        let t = self.time;
        // (start, direction along the wall, outward normal)
        let edges = [
            (vec2(-h, -h), vec2(1.0, 0.0), vec2(0.0, -1.0)),
            (vec2(-h, h), vec2(1.0, 0.0), vec2(0.0, 1.0)),
            (vec2(-h, -h), vec2(0.0, 1.0), vec2(-1.0, 0.0)),
            (vec2(h, -h), vec2(0.0, 1.0), vec2(1.0, 0.0)),
        ];
        let n = (h * 2.0 / block).ceil() as i32;
        for (ei, (start, along, out)) in edges.iter().enumerate() {
            for i in 0..=n {
                let a = *start + *along * (i as f32 * block);
                if a.x < lo.x - block || a.x > hi.x + block || a.y < lo.y - block || a.y > hi.y + block {
                    continue;
                }
                let v = hf(i, ei as i32, 41);
                let stone = Color::new((44.0 + v * 14.0) / 255.0, (41.0 + v * 12.0) / 255.0, (46.0 + v * 14.0) / 255.0, 1.0);
                // The inner face of the wall, darker, with a shadow cast onto the floor.
                let f0 = a;
                let f1 = a + *along * block;
                let g0 = f0 - *out * face;
                let g1 = f1 - *out * face;
                let face_col = Color::new(stone.r * 0.55, stone.g * 0.55, stone.b * 0.6, 1.0);
                draw_triangle(f0, f1, g1, face_col);
                draw_triangle(f0, g1, g0, face_col);
                let s0 = g0 - *out * 30.0;
                let s1 = g1 - *out * 30.0;
                draw_triangle(g0, g1, s1, Color::new(0.0, 0.0, 0.0, 0.35));
                draw_triangle(g0, s1, s0, Color::new(0.0, 0.0, 0.0, 0.35));
                // A block of the wall, its top face lighter, mortar lines dark.
                let p0 = a;
                let p1 = a + *along * block;
                let q0 = p0 + *out * thick;
                let q1 = p1 + *out * thick;
                draw_triangle(p0, p1, q1, stone);
                draw_triangle(p0, q1, q0, stone);
                let rim = Color::new(stone.r * 1.5, stone.g * 1.5, stone.b * 1.5, 1.0);
                draw_line(p0.x, p0.y, p1.x, p1.y, 4.0, rim);
                draw_line(p0.x, p0.y, q0.x, q0.y, 1.5, rgba(8, 7, 9, 255));
                let mid0 = p0 + *out * (thick * 0.5);
                let mid1 = p1 + *out * (thick * 0.5);
                draw_line(mid0.x, mid0.y, mid1.x, mid1.y, 1.0, rgba(10, 9, 12, 255));
                // Crenellations on the inner face, and the odd crack.
                if i % 2 == 0 {
                    let c0 = p0 - *out * 10.0 + *along * 12.0;
                    let c1 = c0 + *along * 26.0;
                    draw_line(c0.x, c0.y, c1.x, c1.y, 10.0, stone);
                }
                if v > 0.8 {
                    let c = a + *along * 30.0 + *out * 10.0;
                    draw_line(c.x, c.y, c.x + out.x * 18.0 + along.x * 6.0, c.y + out.y * 18.0 + along.y * 6.0, 1.0, rgba(6, 5, 7, 255));
                }
                // Torches every eighth block.
                if i % 8 == 4 {
                    let base = a + *along * (block * 0.5) - *out * (face + 2.0);
                    let tip = base - *out * 14.0 - vec2(0.0, 16.0);
                    draw_line(base.x, base.y, tip.x, tip.y + 8.0, 4.0, rgba(60, 44, 30, 255));
                    draw_circle(base.x, base.y, 4.0, rgba(40, 40, 46, 255));
                    let fl = 0.8 + 0.2 * (t * 13.0 + i as f32).sin();
                    for k in 0..4u8 {
                        let o = vec2((t * 9.0 + k as f32 * 1.7 + i as f32).sin() * 2.5, -(k as f32) * 4.0);
                        draw_circle(tip.x + o.x, tip.y + o.y, (9.0 - k as f32 * 2.0) * fl, rgba(255, 110 + k * 30, 40, 220));
                    }
                    draw_circle(tip.x, tip.y, 3.0, rgba(255, 230, 160, 255));
                }
            }
        }
    }

    /// Fixed light sources of the scenery: candles and wall torches.
    pub(super) fn static_lights(&self, out: &mut Vec<(Vec2, f32, Color)>) {
        let fl = |seed: f32| 0.85 + 0.15 * (self.time * 9.0 + seed).sin();
        let (x0, x1, y0, y1) = tile_range(self.cam, TILE * 6.0);
        for ty in y0..y1 {
            for tx in x0..x1 {
                if (171..=176).contains(&(hash(tx, ty, 2) % 1000)) {
                    let ox = tx as f32 * TILE + 12.0 + hf(tx, ty, 3) * 40.0;
                    let oy = ty as f32 * TILE + 12.0 + hf(tx, ty, 4) * 40.0;
                    if inside_world(vec2(ox, oy), 90.0) {
                        out.push((vec2(ox, oy), 170.0 * fl(ox), Color::new(0.75, 0.42, 0.16, 1.0)));
                    }
                }
            }
        }
        let h = crate::world::field_half();
        let n = (h * 2.0 / 64.0).ceil() as i32;
        let half = super::view_size() * 0.5 + vec2(400.0, 400.0);
        for i in (4..=n).step_by(8) {
            let off = i as f32 * 64.0 + 32.0 - h;
            for p in [vec2(off, -h + 10.0), vec2(off, h - 10.0), vec2(-h + 10.0, off), vec2(h - 10.0, off)] {
                let d = p - self.cam;
                if d.x.abs() < half.x && d.y.abs() < half.y {
                    out.push((p, 340.0 * fl(off), Color::new(1.0, 0.55, 0.22, 1.0)));
                }
            }
        }
    }
}
