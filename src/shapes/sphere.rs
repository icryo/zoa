use crate::renderer::{AsciiBuffer, RenderMode, Renderer, Vec3};
use std::f32::consts::PI;

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
        self.render_with_mode(renderer, buffer, RenderMode::Solid);
    }

    pub fn render_with_mode(&self, renderer: &Renderer, buffer: &mut AsciiBuffer, mode: RenderMode) {
        match mode {
            RenderMode::Solid => self.render_solid(renderer, buffer),
            RenderMode::Wireframe => self.render_wireframe(renderer, buffer),
        }
    }

    fn render_solid(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let u_step = 2.0 * PI / self.u_steps as f32;
        let v_step = PI / self.v_steps as f32;

        for i in 0..self.u_steps {
            let u = i as f32 * u_step;
            let (sin_u, cos_u) = u.sin_cos();

            for j in 0..=self.v_steps {
                let v = j as f32 * v_step;
                let (sin_v, cos_v) = v.sin_cos();

                // Point on sphere surface (spherical coordinates)
                let position = Vec3::new(
                    self.radius * sin_v * cos_u,
                    self.radius * cos_v,
                    self.radius * sin_v * sin_u,
                );

                // Normal is just the normalized position for a sphere centered at origin
                let normal = Vec3::new(
                    sin_v * cos_u,
                    cos_v,
                    sin_v * sin_u,
                );

                // Apply rotations
                let rotated_pos = position
                    .rotate_y(self.rotation.y)
                    .rotate_x(self.rotation.x)
                    .rotate_z(self.rotation.z);

                let rotated_normal = normal
                    .rotate_y(self.rotation.y)
                    .rotate_x(self.rotation.x)
                    .rotate_z(self.rotation.z)
                    .normalize();

                renderer.render_point(buffer, rotated_pos, rotated_normal);
            }
        }
    }

    fn render_wireframe(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let latitude_lines = 8;
        let longitude_lines = 12;
        let points_per_line = 60;

        // Draw latitude lines (horizontal circles)
        for i in 1..latitude_lines {
            let v = (i as f32 / latitude_lines as f32) * PI;
            let (sin_v, cos_v) = v.sin_cos();

            for j in 0..points_per_line {
                let u = (j as f32 / points_per_line as f32) * 2.0 * PI;
                let (sin_u, cos_u) = u.sin_cos();

                let position = Vec3::new(
                    self.radius * sin_v * cos_u,
                    self.radius * cos_v,
                    self.radius * sin_v * sin_u,
                );

                let normal = Vec3::new(sin_v * cos_u, cos_v, sin_v * sin_u);

                let rotated_pos = position
                    .rotate_y(self.rotation.y)
                    .rotate_x(self.rotation.x)
                    .rotate_z(self.rotation.z);

                let rotated_normal = normal
                    .rotate_y(self.rotation.y)
                    .rotate_x(self.rotation.x)
                    .rotate_z(self.rotation.z)
                    .normalize();

                renderer.render_point(buffer, rotated_pos, rotated_normal);
            }
        }

        // Draw longitude lines (vertical semicircles)
        for i in 0..longitude_lines {
            let u = (i as f32 / longitude_lines as f32) * 2.0 * PI;
            let (sin_u, cos_u) = u.sin_cos();

            for j in 0..=points_per_line {
                let v = (j as f32 / points_per_line as f32) * PI;
                let (sin_v, cos_v) = v.sin_cos();

                let position = Vec3::new(
                    self.radius * sin_v * cos_u,
                    self.radius * cos_v,
                    self.radius * sin_v * sin_u,
                );

                let normal = Vec3::new(sin_v * cos_u, cos_v, sin_v * sin_u);

                let rotated_pos = position
                    .rotate_y(self.rotation.y)
                    .rotate_x(self.rotation.x)
                    .rotate_z(self.rotation.z);

                let rotated_normal = normal
                    .rotate_y(self.rotation.y)
                    .rotate_x(self.rotation.x)
                    .rotate_z(self.rotation.z)
                    .normalize();

                renderer.render_point(buffer, rotated_pos, rotated_normal);
            }
        }
    }
}
