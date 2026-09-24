use super::{impl_rotating_scene, Surface};
use crate::renderer::{AsciiBuffer, Mat3, Renderer, Vec3};
use std::f32::consts::TAU;

#[derive(Debug, Clone)]
pub struct Torus {
    pub r1: f32, // tube radius
    pub r2: f32, // torus radius (distance from center to tube center)
    pub rotation: Vec3,
    pub rotation_speed: Vec3,
    theta_steps: usize,
    phi_steps: usize,
}

impl Default for Torus {
    fn default() -> Self {
        Self {
            r1: 1.0,
            r2: 2.0,
            rotation: Vec3::default(),
            rotation_speed: Vec3::new(0.8, 0.4, 0.2),
            theta_steps: 100,
            phi_steps: 50,
        }
    }
}

impl Torus {
    pub fn new(r1: f32, r2: f32) -> Self {
        Self {
            r1,
            r2,
            ..Default::default()
        }
    }

    pub fn with_detail(mut self, theta_steps: usize, phi_steps: usize) -> Self {
        self.theta_steps = theta_steps;
        self.phi_steps = phi_steps;
        self
    }

    pub fn set_detail(&mut self, theta_steps: usize, phi_steps: usize) {
        self.theta_steps = theta_steps.max(10);
        self.phi_steps = phi_steps.max(5);
    }

    pub fn set_speed_multiplier(&mut self, multiplier: f32) {
        self.rotation_speed = Vec3::new(0.8, 0.4, 0.2) * multiplier;
    }

    pub fn update(&mut self, dt: f32) {
        self.rotation.x += self.rotation_speed.x * dt;
        self.rotation.y += self.rotation_speed.y * dt;
        self.rotation.z += self.rotation_speed.z * dt;
    }

    pub fn render(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let (r1, r2) = (self.r1, self.r2);
        let surface = Surface {
            // u: angle around the tube, v: angle around the torus
            sample: |u: f32, v: f32| {
                let (sin_theta, cos_theta) = (u * TAU).sin_cos();
                let (sin_phi, cos_phi) = (v * TAU).sin_cos();
                let circle_x = r2 + r1 * cos_theta;
                (
                    Vec3::new(circle_x * cos_phi, r1 * sin_theta, circle_x * sin_phi),
                    Vec3::new(cos_theta * cos_phi, sin_theta, cos_theta * sin_phi),
                )
            },
            rotation: Mat3::from_rotation(self.rotation),
        };
        surface.render(renderer, buffer, (self.theta_steps, self.phi_steps), (12, 8));
    }
}

impl_rotating_scene!(Torus);
