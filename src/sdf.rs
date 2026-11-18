//! Signed distance primitives and combinators.

use glam::{Mat3, Vec3};

/// Approximate distance to an axis-aligned ellipsoid with radii `r`.
pub fn ellipsoid(p: Vec3, r: Vec3) -> f32 {
    let k0 = (p / r).length();
    let k1 = (p / (r * r)).length();
    if k1 == 0.0 {
        return -r.min_element();
    }
    k0 * (k0 - 1.0) / k1
}

/// Distance to a capsule (line segment `a`–`b` with radius `r`).
pub fn capsule(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    (pa - ba * h).length() - r
}

/// Smooth minimum: blends two shapes together with radius `k`.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

pub fn rot_y(a: f32) -> Mat3 {
    Mat3::from_rotation_y(a)
}

pub fn rot_z(a: f32) -> Mat3 {
    Mat3::from_rotation_z(a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;

    #[test]
    fn ellipsoid_sign() {
        let r = vec3(1.0, 0.5, 0.5);
        assert!(ellipsoid(Vec3::ZERO, r) < 0.0);
        assert!(ellipsoid(vec3(2.0, 0.0, 0.0), r) > 0.0);
        assert!(ellipsoid(vec3(1.0, 0.0, 0.0), r).abs() < 1e-4);
    }

    #[test]
    fn capsule_distance() {
        let d = capsule(vec3(0.0, 0.5, 1.0), Vec3::ZERO, Vec3::Y, 0.25);
        assert!((d - 0.75).abs() < 1e-5);
    }

    #[test]
    fn smin_is_below_min() {
        assert!(smin(0.1, 0.1, 0.2) < 0.1);
        assert!((smin(0.0, 5.0, 0.2) - 0.0).abs() < 1e-6);
    }
}
