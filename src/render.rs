//! Ray marching into a grid of character cells, and ANSI encoding.

use crate::character::{self, Mat, Pose, GROUND_Y};
use clap::ValueEnum;
use glam::{vec3, Vec3};
use rayon::prelude::*;
use std::fmt::Write;

/// Brightness ramp, darkest to brightest.
const RAMP: &[u8] = b".:nQ08B&#$@";

/// A terminal cell is roughly twice as tall as it is wide.
const CELL_ASPECT: f32 = 0.5;

const CAM_POS: Vec3 = Vec3::new(0.0, 0.45, 6.5);
const FOCAL: f32 = 1.8;
const MAX_STEPS: usize = 72;
const MAX_DIST: f32 = 12.0;
const HIT_EPS: f32 = 1e-3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: u8,
    pub rgb: [u8; 3],
}

const EMPTY: Cell = Cell {
    ch: b' ',
    rgb: [0, 0, 0],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    /// 24-bit colour.
    Truecolor,
    /// xterm 256-colour palette (e.g. macOS Terminal.app).
    #[value(name = "256")]
    Ansi256,
    /// Plain characters, no escape codes.
    None,
}

impl ColorMode {
    /// Pick a mode from the environment (`NO_COLOR`, `COLORTERM`).
    pub fn detect() -> Self {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return ColorMode::None;
        }
        match std::env::var("COLORTERM") {
            Ok(v) if v.contains("truecolor") || v.contains("24bit") => ColorMode::Truecolor,
            _ => ColorMode::Ansi256,
        }
    }
}

fn normal(p: Vec3, pose: &Pose) -> Vec3 {
    let e = 1e-3;
    let d = |o: Vec3| character::distance(p + o, pose) - character::distance(p - o, pose);
    vec3(d(Vec3::X * e), d(Vec3::Y * e), d(Vec3::Z * e)).normalize_or_zero()
}

/// Distance along the ray to the bounding sphere, if the ray hits it.
fn hit_sphere(ro: Vec3, rd: Vec3, c: Vec3, r: f32) -> Option<(f32, f32)> {
    let oc = ro - c;
    let b = oc.dot(rd);
    let h = b * b - (oc.length_squared() - r * r);
    if h < 0.0 {
        return None;
    }
    let h = h.sqrt();
    Some(((-b - h).max(0.0), -b + h))
}

fn march(ro: Vec3, rd: Vec3, pose: &Pose) -> Option<Vec3> {
    let (c, r) = character::bounds(pose);
    let (mut t, t_end) = hit_sphere(ro, rd, c, r)?;
    for _ in 0..MAX_STEPS {
        let p = ro + rd * t;
        let d = character::distance(p, pose);
        if d < HIT_EPS {
            return Some(p);
        }
        t += d;
        if t > t_end.min(MAX_DIST) {
            break;
        }
    }
    None
}

fn shade_surface(p: Vec3, rd: Vec3, pose: &Pose, mono: bool) -> Cell {
    let n = normal(p, pose);
    let light = vec3(-0.4, 0.75, 0.65).normalize();
    let diffuse = n.dot(light).max(0.0);
    let facing = n.dot(-rd).max(0.0);
    let b = (0.1 + 0.55 * diffuse + 0.4 * facing.powf(1.5)).clamp(0.0, 1.0);
    let lit = |c: [f32; 3]| {
        let k = 0.35 + 0.65 * b;
        [(c[0] * k) as u8, (c[1] * k) as u8, (c[2] * k) as u8]
    };
    let ramp = RAMP[((b * (RAMP.len() - 1) as f32).round() as usize).min(RAMP.len() - 1)];

    match character::material(p, pose) {
        Mat::Body => Cell {
            ch: ramp,
            rgb: lit([255.0, 246.0, 228.0]),
        },
        Mat::EarInner => Cell {
            ch: ramp,
            rgb: lit([255.0, 168.0, 186.0]),
        },
        Mat::Cheek => Cell {
            ch: ramp,
            rgb: lit([255.0, 128.0, 150.0]),
        },
        Mat::Eye => Cell {
            ch: if mono { b' ' } else { b'@' },
            rgb: [38, 28, 34],
        },
        Mat::EyeClosed => Cell {
            ch: b'-',
            rgb: [70, 45, 52],
        },
        Mat::Mouth => Cell {
            ch: b'w',
            rgb: [190, 70, 95],
        },
    }
}

/// Soft blob shadow on the ground under the character.
fn shade_ground(ro: Vec3, rd: Vec3, pose: &Pose) -> Cell {
    if rd.y >= 0.0 {
        return EMPTY;
    }
    let t = (GROUND_Y - 0.02 - ro.y) / rd.y;
    let g = ro + rd * t;
    let r = 1.0 - pose.hop_height() * 0.35;
    let d = vec3((g.x - pose.offset.x) / (1.25 * r), 0.0, (g.z - pose.offset.z) / r).length();
    let k = 1.0 - d;
    if k <= 0.0 {
        return EMPTY;
    }
    let ch = if k > 0.45 { b':' } else if k > 0.12 { b'.' } else { return EMPTY };
    let v = (60.0 + 80.0 * k) as u8;
    Cell { ch, rgb: [v, v, v] }
}

/// Render one frame of `w` x `h` cells.
pub fn render(w: usize, h: usize, pose: &Pose, mono: bool) -> Vec<Cell> {
    let mut cells = vec![EMPTY; w * h];
    if w == 0 || h == 0 {
        return cells;
    }
    // Fit the scene to whichever of width/height is tighter.
    let unit = (h as f32).min(w as f32 * CELL_ASPECT * 1.1);
    cells.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let v = (h as f32 * 0.5 - y as f32 - 0.5) / unit;
        for (x, cell) in row.iter_mut().enumerate() {
            let u = (x as f32 + 0.5 - w as f32 * 0.5) * CELL_ASPECT / unit;
            let rd = vec3(u, v, -FOCAL).normalize();
            *cell = match march(CAM_POS, rd, pose) {
                Some(p) => shade_surface(p, rd, pose, mono),
                None => shade_ground(CAM_POS, rd, pose),
            };
        }
    });
    cells
}

fn rgb_to_256(c: [u8; 3]) -> u8 {
    let [r, g, b] = c.map(|v| v as i32);
    if (r - g).abs() < 10 && (g - b).abs() < 10 {
        let avg = (r + g + b) / 3;
        if avg < 8 {
            return 16;
        }
        if avg > 238 {
            return 231;
        }
        return (232 + (avg - 8) * 24 / 231) as u8;
    }
    let q = |v: i32| ((v * 5 + 127) / 255) as u8;
    16 + 36 * q(r) + 6 * q(g) + q(b)
}

/// Turn a frame into a string of rows joined by `newline`.
pub fn encode(cells: &[Cell], w: usize, mode: ColorMode, newline: &str) -> String {
    let mut out = String::with_capacity(cells.len() * 4);
    let mut last: Option<[u8; 3]> = None;
    for (i, row) in cells.chunks(w).enumerate() {
        if i > 0 {
            out.push_str(newline);
        }
        for c in row {
            if c.ch != b' ' && last != Some(c.rgb) {
                match mode {
                    ColorMode::Truecolor => {
                        let [r, g, b] = c.rgb;
                        let _ = write!(out, "\x1b[38;2;{r};{g};{b}m");
                    }
                    ColorMode::Ansi256 => {
                        let _ = write!(out, "\x1b[38;5;{}m", rgb_to_256(c.rgb));
                    }
                    ColorMode::None => {}
                }
                last = Some(c.rgb);
            }
            out.push(c.ch as char);
        }
    }
    if mode != ColorMode::None {
        out.push_str("\x1b[0m");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::{pose, Anim};

    fn frame(anim: Anim, t: f32) -> String {
        let p = pose(anim, t);
        let cells = render(100, 50, &p, false);
        encode(&cells, 100, ColorMode::None, "\n")
    }

    #[test]
    fn draws_character_and_face() {
        let s = frame(Anim::Idle, 1.0);
        assert!(s.contains('@'), "no bright surface");
        assert!(s.contains('w'), "no mouth");
        let filled = s.chars().filter(|c| !c.is_whitespace()).count();
        assert!(filled > 800, "too few cells drawn: {filled}");
    }

    #[test]
    fn blink_closes_eyes() {
        // t = 0.08 is the middle of the first blink.
        assert!(frame(Anim::Idle, 0.08).contains('-'));
        assert!(!frame(Anim::Idle, 1.0).contains('-'));
    }

    #[test]
    fn all_animations_render() {
        for anim in [Anim::Auto, Anim::Idle, Anim::Hop, Anim::Wave, Anim::Dance] {
            for t in [0.0, 0.3, 2.7, 11.0] {
                assert!(frame(anim, t).contains('w'), "{anim:?} at {t}");
            }
        }
    }

    #[test]
    fn color_256_range() {
        assert_eq!(rgb_to_256([0, 0, 0]), 16);
        assert_eq!(rgb_to_256([255, 255, 255]), 231);
        assert_eq!(rgb_to_256([255, 0, 0]), 196);
    }
}
