use super::{impl_rotating_scene, Surface};
use crate::renderer::{AsciiBuffer, Mat3, RenderMode, Renderer, Vec3};
use std::f32::consts::{PI, TAU};

#[derive(Debug, Clone)]
pub struct Sphere {
    pub radius: f32,
    pub rotation: Vec3,
    pub rotation_speed: Vec3,
    u_steps: usize,
    v_steps: usize,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            radius: 2.0,
            rotation: Vec3::default(),
            rotation_speed: Vec3::new(0.3, 0.5, 0.1),
            u_steps: 80,
            v_steps: 40,
        }
    }
}

impl Sphere {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            ..Default::default()
        }
    }

    pub fn with_detail(mut self, u_steps: usize, v_steps: usize) -> Self {
        self.u_steps = u_steps;
        self.v_steps = v_steps;
        self
    }

    pub fn set_detail(&mut self, u_steps: usize, v_steps: usize) {
        self.u_steps = u_steps.max(10);
        self.v_steps = v_steps.max(5);
    }

    pub fn set_speed_multiplier(&mut self, multiplier: f32) {
        self.rotation_speed = Vec3::new(0.3, 0.5, 0.1) * multiplier;
    }

    pub fn update(&mut self, dt: f32) {
        self.rotation.x += self.rotation_speed.x * dt;
        self.rotation.y += self.rotation_speed.y * dt;
        self.rotation.z += self.rotation_speed.z * dt;
    }

    pub fn render(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let radius = self.radius;
        let surface = Surface {
            // u: longitude, v: latitude from pole to pole
            sample: |u: f32, v: f32| {
                let (sin_u, cos_u) = (u * TAU).sin_cos();
                let (sin_v, cos_v) = (v * PI).sin_cos();
                let normal = Vec3::new(sin_v * cos_u, cos_v, sin_v * sin_u);
                (normal * radius, normal)
            },
            rotation: Mat3::from_rotation(self.rotation),
        };
        match renderer.mode {
            RenderMode::Solid => surface.render_solid(renderer, buffer, self.u_steps, self.v_steps),
            RenderMode::Wireframe => surface.render_wireframe(
                renderer,
                buffer,
                (0..12).map(|i| i as f32 / 12.0),
                (1..8).map(|i| i as f32 / 8.0),
                60,
            ),
        }
    }
}

impl_rotating_scene!(Sphere);
