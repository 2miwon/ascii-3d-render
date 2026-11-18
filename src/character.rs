//! "Mochi": the built-in original character, plus its animations.
//!
//! The character is a round rice-cake creature: a squashy body, two short
//! rounded ears, nub arms, little feet, and a face painted on as decals.

use crate::sdf::{capsule, ellipsoid, rot_y, rot_z, smin};
use clap::ValueEnum;
use glam::{vec3, Vec3};
use std::f32::consts::{PI, TAU};

/// Height of the ground plane; the body rests on it.
pub const GROUND_Y: f32 = -0.95;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Anim {
    /// Cycle through every animation.
    Auto,
    Idle,
    Hop,
    Wave,
    Dance,
}

/// Everything that moves, for a single frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pose {
    /// Whole-body translation (hop height lives in `offset.y`).
    pub offset: Vec3,
    /// > 0 squashes the body flat, < 0 stretches it tall.
    pub squash: f32,
    /// Turn around the vertical axis (radians).
    pub yaw: f32,
    /// Lean sideways around the feet (radians).
    pub tilt: f32,
    /// Extra ear rotation, [left, right] from the viewer's side.
    pub ear: [f32; 2],
    /// Arm angle from hanging down (0) to straight up (PI), [left, right].
    pub arm: [f32; 2],
    /// 0 = eyes open, 1 = shut.
    pub blink: f32,
}

impl Pose {
    pub fn lerp(&self, o: &Pose, k: f32) -> Pose {
        let l = |a: f32, b: f32| a + (b - a) * k;
        Pose {
            offset: self.offset.lerp(o.offset, k),
            squash: l(self.squash, o.squash),
            yaw: l(self.yaw, o.yaw),
            tilt: l(self.tilt, o.tilt),
            ear: [l(self.ear[0], o.ear[0]), l(self.ear[1], o.ear[1])],
            arm: [l(self.arm[0], o.arm[0]), l(self.arm[1], o.arm[1])],
            blink: l(self.blink, o.blink),
        }
    }

    pub fn hop_height(&self) -> f32 {
        self.offset.y
    }
}

fn blink(t: f32) -> f32 {
    // Blink every ~3.7 s, with a quick double blink every third time.
    let period = 3.7;
    let ph = t.rem_euclid(period);
    let one = |x: f32| {
        if (0.0..0.16).contains(&x) {
            (x / 0.16 * PI).sin()
        } else {
            0.0
        }
    };
    let double = ((t / period) as i32).rem_euclid(3) == 2;
    one(ph) + if double { one(ph - 0.25) } else { 0.0 }
}

fn idle(t: f32) -> Pose {
    let breath = (t * 2.0).sin();
    Pose {
        squash: breath * 0.03,
        yaw: (t * 0.6).sin() * 0.25,
        ear: [(t * 1.7).sin() * 0.08, -(t * 1.7 + 0.6).sin() * 0.08],
        arm: [0.35 + breath * 0.05, 0.35 + breath * 0.05],
        ..Default::default()
    }
}

fn hop(t: f32) -> Pose {
    let period = 0.62;
    let ph = (t / period).fract();
    let h = (ph * PI).sin(); // 0 on the ground, 1 at the top
    let squash = if h < 0.25 { (0.25 - h) * 0.6 } else { -0.06 * h };
    let flop = (ph * TAU).cos() * 0.3;
    Pose {
        offset: vec3(0.0, 0.55 * h, 0.0),
        squash,
        yaw: (t * 0.8).sin() * 0.2,
        ear: [flop, -flop],
        arm: [0.4 + 1.4 * h, 0.4 + 1.4 * h],
        ..Default::default()
    }
}

fn wave(t: f32) -> Pose {
    let w = (t * 8.0).sin();
    Pose {
        squash: (t * 2.0).sin() * 0.02,
        yaw: 0.2,
        tilt: -0.07,
        ear: [w * 0.06, w * 0.12],
        arm: [0.35, 2.5 + w * 0.4],
        ..Default::default()
    }
}

fn dance(t: f32) -> Pose {
    let b = t * TAU; // one sway per second
    let side = b.sin();
    let bounce = (b * 2.0).sin().abs();
    Pose {
        offset: vec3(side * 0.25, bounce * 0.15, 0.0),
        squash: (1.0 - bounce) * 0.08,
        yaw: (b * 0.5).sin() * 0.5,
        tilt: -side * 0.18,
        ear: [side * 0.35, side * 0.35],
        arm: [1.6 + side * 1.0, 1.6 - side * 1.0],
        ..Default::default()
    }
}

pub fn pose(anim: Anim, t: f32) -> Pose {
    let base = match anim {
        Anim::Idle => idle(t),
        Anim::Hop => hop(t),
        Anim::Wave => wave(t),
        Anim::Dance => dance(t),
        Anim::Auto => {
            const SEQ: [fn(f32) -> Pose; 4] = [idle, hop, wave, dance];
            const SLOT: f32 = 5.0;
            const FADE: f32 = 0.5;
            let i = (t / SLOT) as usize;
            let local = t - i as f32 * SLOT;
            let cur = SEQ[i % SEQ.len()](t);
            if local > SLOT - FADE {
                let next = SEQ[(i + 1) % SEQ.len()](t);
                let k = (local - (SLOT - FADE)) / FADE;
                cur.lerp(&next, k * k * (3.0 - 2.0 * k))
            } else {
                cur
            }
        }
    };
    Pose {
        blink: blink(t).min(1.0),
        ..base
    }
}

/// What a ray hit, used to pick the glyph and colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mat {
    Body,
    EarInner,
    Cheek,
    Eye,
    EyeClosed,
    Mouth,
}

/// Body-local frame for a pose, shared by the distance and material passes.
struct Frame {
    radii: Vec3,
    center: Vec3,
}

impl Frame {
    fn new(pose: &Pose) -> Self {
        let ry = 0.95 * (1.0 - pose.squash);
        let wide = 1.0 + pose.squash * 0.6;
        Frame {
            radii: vec3(1.15 * wide, ry, 1.0 * wide),
            center: vec3(0.0, GROUND_Y + ry, 0.0),
        }
    }

    /// World point -> body-local point (origin at the body centre).
    fn local(&self, p: Vec3, pose: &Pose) -> Vec3 {
        let pivot = vec3(0.0, GROUND_Y, 0.0);
        let q = p - pose.offset - pivot;
        let q = rot_y(-pose.yaw) * (rot_z(-pose.tilt) * q) + pivot;
        q - self.center
    }
}

const EAR_R: Vec3 = Vec3::new(0.24, 0.36, 0.2);
const ARM_LEN: f32 = 0.42;
const ARM_R: f32 = 0.15;

fn ear_local(b: Vec3, f: &Frame, pose: &Pose, i: usize) -> Vec3 {
    let s = if i == 0 { -1.0 } else { 1.0 };
    let pivot = vec3(s * 0.5 * f.radii.x, f.radii.y * 0.78, 0.0);
    let theta = -s * 0.35 + pose.ear[i];
    rot_z(-theta) * (b - pivot) - vec3(0.0, 0.28, 0.0)
}

fn arm_dist(b: Vec3, f: &Frame, pose: &Pose, i: usize) -> f32 {
    let s = if i == 0 { -1.0 } else { 1.0 };
    let a = pose.arm[i];
    let shoulder = vec3(s * 0.85 * f.radii.x, -0.05, 0.25);
    let hand = shoulder + ARM_LEN * vec3(s * a.sin(), -a.cos(), 0.2).normalize();
    capsule(b, shoulder, hand, ARM_R)
}

fn foot_dist(b: Vec3, f: &Frame, i: usize) -> f32 {
    let s = if i == 0 { -1.0 } else { 1.0 };
    let c = vec3(s * 0.42 * f.radii.x, -f.radii.y + 0.08, 0.45 * f.radii.z);
    ellipsoid(b - c, vec3(0.26, 0.13, 0.24))
}

/// Signed distance from `p` to the character.
pub fn distance(p: Vec3, pose: &Pose) -> f32 {
    let f = Frame::new(pose);
    let b = f.local(p, pose);
    let mut d = ellipsoid(b, f.radii);
    for i in 0..2 {
        d = smin(d, ellipsoid(ear_local(b, &f, pose, i), EAR_R), 0.15);
        d = smin(d, arm_dist(b, &f, pose, i), 0.1);
        d = smin(d, foot_dist(b, &f, i), 0.08);
    }
    d
}

/// A sphere that always contains the character, for early ray rejection.
pub fn bounds(pose: &Pose) -> (Vec3, f32) {
    (pose.offset + vec3(0.0, 0.2, 0.0), 2.1)
}

/// Material at a surface point `p`.
pub fn material(p: Vec3, pose: &Pose) -> Mat {
    let f = Frame::new(pose);
    let b = f.local(p, pose);

    // Inner ear: the front-centre stripe of each ear.
    for i in 0..2 {
        let e = ear_local(b, &f, pose, i);
        if ellipsoid(e, EAR_R) < 0.03 && e.z > 0.06 && e.x.abs() < 0.11 && e.y > -0.2 {
            return Mat::EarInner;
        }
    }

    // Face decals live on the front of the body only.
    if b.z < 0.3 * f.radii.z || ellipsoid(b, f.radii) > 0.03 {
        return Mat::Body;
    }
    let x = b.x / f.radii.x;
    let y = b.y / f.radii.y;

    for cx in [-0.33, 0.33] {
        let (cy, rx) = (0.14, 0.075);
        if pose.blink > 0.6 {
            if (y - cy + 0.02).abs() < 0.03 && (x - cx).abs() < rx * 1.3 {
                return Mat::EyeClosed;
            }
        } else {
            let ry = 0.12 * (1.0 - pose.blink).max(0.15);
            let e = ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2);
            if e < 1.0 {
                return Mat::Eye;
            }
        }
    }
    if x.abs() < 0.04 && (y + 0.04).abs() < 0.05 {
        return Mat::Mouth;
    }
    for cx in [-0.55, 0.55] {
        if ((x - cx) / 0.12).powi(2) + ((y + 0.05) / 0.09).powi(2) < 1.0 {
            return Mat::Cheek;
        }
    }
    Mat::Body
}
