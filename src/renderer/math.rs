use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vec3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            self
        }
    }

    pub fn rotate_x(self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    pub fn rotate_y(self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    pub fn rotate_z(self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
            z: self.z,
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

/// A 3x3 rotation matrix, stored as columns.
///
/// Building one per frame and reusing it is much cheaper than calling
/// `rotate_x`/`rotate_y`/`rotate_z` (six trig calls) for every point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3 {
    cols: [Vec3; 3],
}

impl Mat3 {
    pub const IDENTITY: Self = Self {
        cols: [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
    };

    /// Rotation equivalent to `v.rotate_y(r.y).rotate_x(r.x).rotate_z(r.z)`,
    /// the order every built-in shape uses.
    pub fn from_rotation(r: Vec3) -> Self {
        let rot = |v: Vec3| v.rotate_y(r.y).rotate_x(r.x).rotate_z(r.z);
        Self {
            cols: [
                rot(Vec3::new(1.0, 0.0, 0.0)),
                rot(Vec3::new(0.0, 1.0, 0.0)),
                rot(Vec3::new(0.0, 0.0, 1.0)),
            ],
        }
    }

    pub fn transform(&self, v: Vec3) -> Vec3 {
        self.cols[0] * v.x + self.cols[1] * v.y + self.cols[2] * v.z
    }
}

/// Terminal character cells are roughly twice as tall as they are wide.
pub const CHAR_ASPECT: f32 = 2.0;

/// Rows per world unit (at depth 1) for `scale == 1.0`, as a fraction of the
/// render area's height. Sized so a radius-3 shape at the default distance
/// fits in any orientation (perspective enlarges its near side by up to 1.5x).
const FIT: f32 = 0.6;

/// Perspective camera looking down +z from `distance` units away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub distance: f32,
    /// Zoom relative to the render area: `1.0` fits the built-in shapes
    /// (radius ~3) in the area, whatever its size.
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            distance: 5.0,
            scale: 1.0,
        }
    }
}

impl Camera {
    /// Rows per world unit at depth 1 for a render area of this size, whose
    /// pixels are `pixel_aspect` times taller than wide.
    pub fn pixels_per_unit(&self, screen_width: u16, screen_height: u16, pixel_aspect: f32) -> f32 {
        let fit = (screen_height as f32).min(screen_width as f32 / pixel_aspect);
        self.scale * FIT * fit
    }

    /// Project to continuous screen coordinates (cell `(x, y)` spans
    /// `[x, x + 1) x [y, y + 1)`), without clipping to the screen.
    /// Returns `(x, y, 1/z)`, or `None` if the point is behind the near plane.
    pub fn project_f(&self, point: Vec3, screen_width: u16, screen_height: u16, pixel_aspect: f32) -> Option<(f32, f32, f32)> {
        let z = self.distance + point.z;
        if z <= 0.1 {
            return None;
        }
        let inv_z = 1.0 / z;
        let ppu = self.pixels_per_unit(screen_width, screen_height, pixel_aspect) * inv_z;
        Some((
            screen_width as f32 / 2.0 + point.x * ppu * pixel_aspect,
            screen_height as f32 / 2.0 - point.y * ppu,
            inv_z,
        ))
    }

    /// Project to the screen cell containing `point`, if it is visible.
    pub fn project(&self, point: Vec3, screen_width: u16, screen_height: u16, pixel_aspect: f32) -> Option<(u16, u16, f32)> {
        if screen_width == 0 || screen_height == 0 {
            return None;
        }
        let (x, y, inv_z) = self.project_f(point, screen_width, screen_height, pixel_aspect)?;
        if x >= 0.0 && x < screen_width as f32 && y >= 0.0 && y < screen_height as f32 {
            Some((x as u16, y as u16, inv_z))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn mat3_matches_euler_rotation() {
        let r = Vec3::new(0.3, -1.2, 2.1);
        let m = Mat3::from_rotation(r);
        for v in [Vec3::new(1.0, 2.0, 3.0), Vec3::new(-0.5, 0.0, 4.0)] {
            assert!(close(m.transform(v), v.rotate_y(r.y).rotate_x(r.x).rotate_z(r.z)));
        }
    }

    #[test]
    fn projection_keeps_proportions_and_scales_with_area() {
        let cam = Camera::default();
        for (w, h) in [(80, 24), (40, 40), (200, 60)] {
            let (cx, cy, _) = cam.project_f(Vec3::ZERO, w, h, CHAR_ASPECT).unwrap();
            let (x, _, _) = cam.project_f(Vec3::new(1.0, 0.0, 0.0), w, h, CHAR_ASPECT).unwrap();
            let (_, y, _) = cam.project_f(Vec3::new(0.0, 1.0, 0.0), w, h, CHAR_ASPECT).unwrap();
            // One unit spans twice as many columns as rows
            assert!(((x - cx) - 2.0 * (cy - y)).abs() < 1e-3);
        }
        let small = cam.pixels_per_unit(80, 24, CHAR_ASPECT);
        let large = cam.pixels_per_unit(200, 60, CHAR_ASPECT);
        assert!(large > 2.0 * small);
    }
}
