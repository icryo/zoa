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

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
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

/// Terminal character cells are roughly twice as tall as they are wide.
const CHAR_ASPECT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub distance: f32,
    pub scale: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            distance: 5.0,
            scale: 40.0,
        }
    }
}

impl Camera {
    pub fn project(&self, point: Vec3, screen_width: u16, screen_height: u16) -> Option<(u16, u16, f32)> {
        // Guard against zero-size buffers
        if screen_width == 0 || screen_height == 0 {
            return None;
        }

        let z = self.distance + point.z;
        if z <= 0.1 {
            return None;
        }

        let inv_z = 1.0 / z;

        // Compensate for tall character cells only, so shapes keep their
        // proportions regardless of the render area's aspect ratio.
        let screen_x = (screen_width as f32 / 2.0 + point.x * self.scale * inv_z * CHAR_ASPECT) as i32;
        let screen_y = (screen_height as f32 / 2.0 - point.y * self.scale * inv_z) as i32;

        if screen_x >= 0 && screen_x < screen_width as i32 && screen_y >= 0 && screen_y < screen_height as i32 {
            Some((screen_x as u16, screen_y as u16, inv_z))
        } else {
            None
        }
    }
}
