// The game's icon, drawn by code: a flame in an iron brazier on a dark, glowing disc.
// Shared by the window icon (main.rs) and the executable's icon (build.rs includes this
// file), so it must only use std.

/// RGBA pixels of the icon at `size` x `size`, 4x4 supersampled for smooth edges.
pub fn icon_rgba(size: usize) -> Vec<u8> {
    let ss = 4;
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let mut acc = [0.0f32; 4];
            for sy in 0..ss {
                for sx in 0..ss {
                    let u = ((x * ss + sx) as f32 + 0.5) / (size * ss) as f32 * 2.0 - 1.0;
                    let v = ((y * ss + sy) as f32 + 0.5) / (size * ss) as f32 * 2.0 - 1.0;
                    let c = icon_sample(u, v);
                    for k in 0..3 {
                        acc[k] += c[k] * c[3];
                    }
                    acc[3] += c[3];
                }
            }
            let n = (ss * ss) as f32;
            let i = (y * size + x) * 4;
            if acc[3] > 0.0 {
                for k in 0..3 {
                    out[i + k] = (acc[k] / acc[3] * 255.0).round().clamp(0.0, 255.0) as u8;
                }
                out[i + 3] = (acc[3] / n * 255.0).round() as u8;
            }
        }
    }
    out
}

fn mix3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    let k = k.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
}

/// Colour and coverage at a point; `u` goes right and `v` down, both from -1 to 1.
fn icon_sample(u: f32, v: f32) -> [f32; 4] {
    let d = (u * u + v * v).sqrt();
    if d > 0.97 {
        return [0.0; 4];
    }
    // A dark disc, warmer where the fire stands.
    let g = (1.0 - (u * u + (v - 0.1) * (v - 0.1)).sqrt() / 0.95).clamp(0.0, 1.0);
    let g = g * g;
    let mut col = [0.05 + 0.32 * g, 0.035 + 0.1 * g, 0.05 + 0.03 * g];
    // A thin bronze rim.
    if d > 0.88 {
        let k = (d - 0.88) / 0.09;
        let ring = 1.0 - (2.0 * k - 1.0).powi(2);
        col = mix3(col, [0.62, 0.4, 0.2], 0.75 * ring);
    }
    // The brazier: a bowl, a stem and a foot.
    let iron = [0.15, 0.13, 0.14];
    let bowl = v > 0.34 && (u * u) / 0.27 + ((v - 0.34) / 0.25).powi(2) < 1.0;
    let stem = u.abs() < 0.075 && (0.55..0.76).contains(&v);
    let foot = u.abs() < 0.3 - (v - 0.72) * 0.5 && (0.72..0.8).contains(&v);
    if bowl || stem || foot {
        col = iron;
        if bowl {
            // Firelight on the rim and a soft shade toward the bottom.
            let rim = (1.0 - (v - 0.34) / 0.06).clamp(0.0, 1.0);
            col = mix3(col, [0.75, 0.42, 0.16], rim * 0.9);
            col = mix3(col, [0.3, 0.22, 0.2], ((-u - 0.1) * 1.5).clamp(0.0, 0.4) * (1.0 - rim));
        }
        return [col[0], col[1], col[2], 1.0];
    }
    // The flame: a teardrop from the bowl up to a curling tip.
    let top = -0.78;
    let bottom = 0.36;
    if v > top && v < bottom {
        let t = (v - top) / (bottom - top);
        let sway = 0.1 * (t * 5.5).sin() * (1.0 - t).powf(1.5);
        let half = 0.4 * (std::f32::consts::PI * t.powf(0.75)).sin().max(0.0) * (0.55 + 0.45 * t);
        let k = (u - sway).abs() / half.max(1e-4);
        if k < 1.0 {
            let heat = (1.0 - k) * (0.35 + 0.65 * t);
            let deep = [0.85, 0.2, 0.05];
            let mid = [1.0, 0.55, 0.12];
            let core = [1.0, 0.95, 0.72];
            let c = if heat < 0.45 { mix3(deep, mid, heat / 0.45) } else { mix3(mid, core, (heat - 0.45) / 0.4) };
            // Soft outer edge.
            let a = ((1.0 - k) * 6.0).min(1.0);
            let c = mix3(col, c, a);
            return [c[0], c[1], c[2], 1.0];
        }
        // A faint halo around the flame.
        let halo = ((1.6 - k) / 0.6).clamp(0.0, 1.0) * 0.35 * t.sqrt();
        col = mix3(col, [1.0, 0.45, 0.12], halo);
    }
    // Embers rising beside it.
    for (ex, ey, r) in [(-0.48, -0.42, 0.045), (0.52, -0.28, 0.04), (0.34, -0.7, 0.032), (-0.28, -0.78, 0.028)] {
        let e = ((u - ex).powi(2) + (v - ey).powi(2)).sqrt();
        if e < r * 2.2 {
            let k = (1.0 - e / (r * 2.2)).powi(2);
            col = mix3(col, if e < r { [1.0, 0.8, 0.45] } else { [1.0, 0.45, 0.15] }, k.max(if e < r { 1.0 } else { 0.0 }));
        }
    }
    [col[0], col[1], col[2], 1.0]
}
