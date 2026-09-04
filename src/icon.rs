//! Genererer tray-ikoner i kode (ingen bildefiler nødvendig).

use tray_icon::Icon;

use crate::syncthing::State;

const SIZE: u32 = 32;
const SS: i32 = 3; // supersampling per akse

fn base_color(state: State) -> [f32; 3] {
    match state {
        State::Ok => [0.18, 0.70, 0.35],      // grønn
        State::Syncing => [0.16, 0.52, 0.90], // blå
        State::Scanning => [0.35, 0.62, 0.85],
        State::Paused => [0.95, 0.72, 0.15], // gul
        State::Error => [0.85, 0.22, 0.22],  // rød
        State::Offline => [0.55, 0.57, 0.60], // grå
    }
}

/// `phase` (0.0-1.0) brukes til å animere synk-ikonet.
pub fn build(state: State, phase: f32) -> Icon {
    Icon::from_rgba(rgba(state, phase), SIZE, SIZE).expect("gyldig ikon-buffer")
}

fn rgba(state: State, phase: f32) -> Vec<u8> {
    let n = SIZE as i32;
    let c = n as f32 / 2.0;
    let r = c - 1.0;
    let col = base_color(state);
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    for y in 0..n {
        for x in 0..n {
            let mut disc = 0.0f32;
            let mut glyph = 0.0f32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SS as f32;
                    let dx = px - c;
                    let dy = py - c;
                    if (dx * dx + dy * dy).sqrt() <= r {
                        disc += 1.0;
                    }
                    if in_glyph(state, dx / r, dy / r, phase) {
                        glyph += 1.0;
                    }
                }
            }
            let total = (SS * SS) as f32;
            let disc = disc / total;
            let glyph = (glyph / total).min(disc);

            let idx = ((y * n + x) * 4) as usize;
            let mix = |base: f32| -> u8 {
                let v = base * (1.0 - glyph) + 1.0 * glyph;
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            };
            rgba[idx] = mix(col[0]);
            rgba[idx + 1] = mix(col[1]);
            rgba[idx + 2] = mix(col[2]);
            rgba[idx + 3] = (disc * 255.0).round() as u8;
        }
    }

    rgba
}

/// ASCII-forhåndsvisning (feilsøking: `syncthing-status --preview`)
pub fn ascii_preview(state: State, phase: f32) -> String {
    let buf = rgba(state, phase);
    let n = SIZE as usize;
    let mut out = String::new();
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            let (r, g, b, a) = (buf[i], buf[i + 1], buf[i + 2], buf[i + 3]);
            let white = r > 200 && g > 200 && b > 200;
            out.push(match (a, white) {
                (0..=40, _) => ' ',
                (_, true) => '#',
                _ => '.',
            });
        }
        out.push('\n');
    }
    out
}

/// Glyf-test i normaliserte koordinater (-1..1, y peker ned).
fn in_glyph(state: State, x: f32, y: f32, phase: f32) -> bool {
    match state {
        // Hake
        State::Ok => {
            seg(x, y, -0.48, 0.02, -0.13, 0.38, 0.20) || seg(x, y, -0.13, 0.38, 0.50, -0.34, 0.20)
        }
        // To roterende buer (synk-piler)
        State::Syncing | State::Scanning => {
            let a0 = phase * std::f32::consts::TAU;
            arc(x, y, 0.36, 0.62, a0, a0 + 2.1) || arc(x, y, 0.36, 0.62, a0 + 3.14, a0 + 5.24)
        }
        // Pause: to loddrette streker
        State::Paused => x.abs() > 0.12 && x.abs() < 0.36 && y.abs() < 0.42,
        // Utropstegn
        State::Error => {
            (x.abs() < 0.13 && y > -0.48 && y < 0.16) || (x.abs() < 0.15 && y > 0.30 && y < 0.54)
        }
        // Kryss
        State::Offline => {
            seg(x, y, -0.34, -0.34, 0.34, 0.34, 0.13) || seg(x, y, 0.34, -0.34, -0.34, 0.34, 0.13)
        }
    }
}

/// Avstand fra punkt til linjestykke < halvbredde
fn seg(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32, width: f32) -> bool {
    let (vx, vy) = (bx - ax, by - ay);
    let (wx, wy) = (px - ax, py - ay);
    let len2 = vx * vx + vy * vy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (ax + vx * t, ay + vy * t);
    let (dx, dy) = (px - cx, py - cy);
    (dx * dx + dy * dy).sqrt() < width / 2.0
}

/// Ringsegment mellom radius r0..r1 og vinkel a0..a1 (radianer)
fn arc(px: f32, py: f32, r0: f32, r1: f32, a0: f32, a1: f32) -> bool {
    let d = (px * px + py * py).sqrt();
    if d < r0 || d > r1 {
        return false;
    }
    let ang = py.atan2(px).rem_euclid(std::f32::consts::TAU);
    let start = a0.rem_euclid(std::f32::consts::TAU);
    let span = (a1 - a0).rem_euclid(std::f32::consts::TAU);
    (ang - start).rem_euclid(std::f32::consts::TAU) <= span
}
