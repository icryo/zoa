//! Signed Distance Field rendering via ray marching.
//!
//! SDFs define shapes mathematically - the distance from any point to the surface.
//! Ray marching steps through space using these distances to find surfaces.

use crate::renderer::AsciiBuffer;

/// Preset SDF scenes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SdfPreset {
    #[default]
    Metaballs,
    Fractal,
    Blend,
    Twist,
    Rings,
    Terrain,
}

impl SdfPreset {
    pub fn next(self) -> Self {
        match self {
            Self::Metaballs => Self::Fractal,
            Self::Fractal => Self::Blend,
            Self::Blend => Self::Twist,
            Self::Twist => Self::Rings,
            Self::Rings => Self::Terrain,
            Self::Terrain => Self::Metaballs,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Metaballs => "Metaballs",
            Self::Fractal => "Fractal",
            Self::Blend => "Blend",
            Self::Twist => "Twist",
            Self::Rings => "Rings",
            Self::Terrain => "Terrain",
        }
    }
}

/// SDF ray marcher
pub struct SdfScene {
    preset: SdfPreset,
    time: f32,
    rotation: Vec3,
    rotation_speed: Vec3,
    camera_dist: f32,
}

impl Default for SdfScene {
    fn default() -> Self {
        Self {
            preset: SdfPreset::default(),
            time: 0.0,
            rotation: Vec3::ZERO,
            rotation_speed: Vec3::new(0.3, 0.5, 0.1),
            camera_dist: 5.0,
        }
    }
}

impl SdfScene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_preset(preset: SdfPreset) -> Self {
        Self {
            preset,
            ..Self::default()
        }
    }

    pub fn set_preset(&mut self, preset: SdfPreset) {
        self.preset = preset;
    }

    pub fn preset(&self) -> SdfPreset {
        self.preset
    }

    pub fn cycle_preset(&mut self) {
        self.preset = self.preset.next();
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.rotation.x += self.rotation_speed.x * dt;
        self.rotation.y += self.rotation_speed.y * dt;
        self.rotation.z += self.rotation_speed.z * dt;
    }

    /// Sample the SDF at a point (returns distance to nearest surface)
    fn sdf(&self, p: Vec3) -> f32 {
        // Apply scene rotation
        let p = p
            .rotate_y(self.rotation.y)
            .rotate_x(self.rotation.x);

        match self.preset {
            SdfPreset::Metaballs => self.sdf_metaballs(p),
            SdfPreset::Fractal => self.sdf_fractal(p),
            SdfPreset::Blend => self.sdf_blend(p),
            SdfPreset::Twist => self.sdf_twist(p),
            SdfPreset::Rings => self.sdf_rings(p),
            SdfPreset::Terrain => self.sdf_terrain(p),
        }
    }

    fn sdf_metaballs(&self, p: Vec3) -> f32 {
        // Three animated spheres that smoothly blend
        let t = self.time;

        let s1_pos = Vec3::new((t * 0.7).sin() * 1.2, (t * 0.5).cos() * 0.8, (t * 0.3).sin() * 0.5);
        let s2_pos = Vec3::new((t * 0.5).cos() * 1.0, (t * 0.8).sin() * 1.0, (t * 0.6).cos() * 0.8);
        let s3_pos = Vec3::new((t * 0.9).sin() * 0.8, (t * 0.4).cos() * 0.6, (t * 0.7).sin() * 1.0);

        let d1 = (p - s1_pos).length() - 0.6;
        let d2 = (p - s2_pos).length() - 0.5;
        let d3 = (p - s3_pos).length() - 0.55;

        // Smooth union
        let k = 0.5;
        let d = smooth_min(d1, d2, k);
        smooth_min(d, d3, k)
    }

    fn sdf_fractal(&self, p: Vec3) -> f32 {
        // Mandelbulb-inspired fractal
        let power = 8.0 + (self.time * 0.2).sin() * 2.0;
        let mut z = p;
        let mut dr = 1.0;
        let mut r;

        for _ in 0..4 {
            r = z.length();
            if r > 2.0 {
                break;
            }

            // Convert to spherical
            let theta = (z.z / r).acos();
            let phi = z.y.atan2(z.x);
            dr = r.powf(power - 1.0) * power * dr + 1.0;

            // Scale and rotate
            let zr = r.powf(power);
            let new_theta = theta * power;
            let new_phi = phi * power;

            z = Vec3::new(
                new_theta.sin() * new_phi.cos(),
                new_theta.sin() * new_phi.sin(),
                new_theta.cos(),
            ) * zr + p;
        }

        r = z.length();
        0.5 * r.ln() * r / dr
    }

    fn sdf_blend(&self, p: Vec3) -> f32 {
        // Morphing between shapes
        let t = (self.time * 0.5).sin() * 0.5 + 0.5;

        let sphere = p.length() - 1.0;
        let box_d = {
            let q = Vec3::new(p.x.abs() - 0.8, p.y.abs() - 0.8, p.z.abs() - 0.8);
            Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).length()
                + q.x.max(q.y.max(q.z)).min(0.0)
        };

        // Linear interpolation
        sphere * (1.0 - t) + box_d * t
    }

    fn sdf_twist(&self, p: Vec3) -> f32 {
        // Twisted torus
        let twist = self.time * 0.5;
        let c = (twist * p.y).cos();
        let s = (twist * p.y).sin();
        let twisted = Vec3::new(c * p.x - s * p.z, p.y, s * p.x + c * p.z);

        // Torus SDF
        let q = Vec3::new((twisted.x * twisted.x + twisted.z * twisted.z).sqrt() - 1.2, twisted.y, 0.0);
        q.length() - 0.4
    }

    fn sdf_rings(&self, p: Vec3) -> f32 {
        // Interlocking rings
        let t = self.time;

        // Three tori at different orientations
        let torus = |p: Vec3, major: f32, minor: f32| -> f32 {
            let q = Vec3::new((p.x * p.x + p.z * p.z).sqrt() - major, p.y, 0.0);
            q.length() - minor
        };

        let p1 = p.rotate_x(t * 0.3);
        let p2 = p.rotate_y(t * 0.4).rotate_z(std::f32::consts::FRAC_PI_2);
        let p3 = p.rotate_z(t * 0.5).rotate_x(std::f32::consts::FRAC_PI_2);

        let d1 = torus(p1, 1.0, 0.2);
        let d2 = torus(p2, 1.0, 0.2);
        let d3 = torus(p3, 1.0, 0.2);

        d1.min(d2).min(d3)
    }

    fn sdf_terrain(&self, p: Vec3) -> f32 {
        // Procedural terrain with noise
        let freq = 2.0;
        let height = noise2d(p.x * freq, p.z * freq + self.time * 0.2) * 0.5
            + noise2d(p.x * freq * 2.0, p.z * freq * 2.0) * 0.25;

        p.y - height + 0.5
    }

    /// Calculate surface normal via gradient
    fn normal(&self, p: Vec3) -> Vec3 {
        let e = 0.001;
        let d = self.sdf(p);
        Vec3::new(
            self.sdf(Vec3::new(p.x + e, p.y, p.z)) - d,
            self.sdf(Vec3::new(p.x, p.y + e, p.z)) - d,
            self.sdf(Vec3::new(p.x, p.y, p.z + e)) - d,
        )
        .normalize()
    }

    /// Ray march from origin in direction, return (hit, distance, iterations)
    fn ray_march(&self, origin: Vec3, dir: Vec3) -> Option<(Vec3, f32)> {
        let max_steps = 64;
        let max_dist = 20.0;
        let surface_dist = 0.01;

        let mut t = 0.0;

        for _ in 0..max_steps {
            let p = origin + dir * t;
            let d = self.sdf(p);

            if d < surface_dist {
                return Some((p, t));
            }

            t += d;

            if t > max_dist {
                break;
            }
        }

        None
    }

    pub fn render(&self, buffer: &mut AsciiBuffer) {
        let width = buffer.width as f32;
        let height = buffer.height as f32;

        if width < 1.0 || height < 1.0 {
            return;
        }

        let aspect = width / height * 0.5; // Terminal chars are ~2:1
        let light_dir = Vec3::new(0.5, 1.0, -0.5).normalize();

        for y in 0..buffer.height {
            for x in 0..buffer.width {
                // Convert to normalized device coordinates
                let u = (x as f32 / width - 0.5) * 2.0 * aspect;
                let v = (y as f32 / height - 0.5) * -2.0;

                // Camera setup
                let ray_origin = Vec3::new(0.0, 0.0, -self.camera_dist);
                let ray_dir = Vec3::new(u, v, 1.0).normalize();

                if let Some((hit_pos, _dist)) = self.ray_march(ray_origin, ray_dir) {
                    let normal = self.normal(hit_pos);
                    let luminance = normal.dot(light_dir).max(0.1);
                    let depth = 1.0 / (1.0 + hit_pos.z * 0.1);

                    buffer.plot(x, y, depth, luminance);
                }
            }
        }
    }
}

// ============ Math helpers ============

#[derive(Clone, Copy, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn normalize(self) -> Self {
        let len = self.length();
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

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn rotate_x(self, angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            x: self.x,
            y: self.y * c - self.z * s,
            z: self.y * s + self.z * c,
        }
    }

    fn rotate_y(self, angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            x: self.x * c + self.z * s,
            y: self.y,
            z: -self.x * s + self.z * c,
        }
    }

    fn rotate_z(self, angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            x: self.x * c - self.y * s,
            y: self.x * s + self.y * c,
            z: self.z,
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

/// Smooth minimum for metaball blending
fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

/// Simple 2D noise for terrain
fn noise2d(x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();

    let hash = |x: i32, y: i32| -> f32 {
        let n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
        let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        (n as f32) / (i32::MAX as f32)
    };

    let a = hash(ix, iy);
    let b = hash(ix + 1, iy);
    let c = hash(ix, iy + 1);
    let d = hash(ix + 1, iy + 1);

    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    a * (1.0 - ux) * (1.0 - uy) + b * ux * (1.0 - uy) + c * (1.0 - ux) * uy + d * ux * uy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets() {
        for preset in [
            SdfPreset::Metaballs,
            SdfPreset::Fractal,
            SdfPreset::Blend,
            SdfPreset::Twist,
            SdfPreset::Rings,
            SdfPreset::Terrain,
        ] {
            let mut scene = SdfScene::with_preset(preset);
            scene.update(0.1);
            let mut buffer = crate::renderer::AsciiBuffer::new(40, 20);
            scene.render(&mut buffer);
            // Should not crash
        }
    }
}
