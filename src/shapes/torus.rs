use crate::renderer::{AsciiBuffer, RenderMode, Renderer, Vec3};
use std::f32::consts::PI;

#[derive(Clone)]
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
        self.render_with_mode(renderer, buffer, RenderMode::Solid);
    }

    pub fn render_with_mode(&self, renderer: &Renderer, buffer: &mut AsciiBuffer, mode: RenderMode) {
        match mode {
            RenderMode::Solid => self.render_solid(renderer, buffer),
            RenderMode::Wireframe => self.render_wireframe(renderer, buffer),
        }
    }

    fn render_solid(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let theta_step = 2.0 * PI / self.theta_steps as f32;
        let phi_step = 2.0 * PI / self.phi_steps as f32;

        for i in 0..self.theta_steps {
            let theta = i as f32 * theta_step;
            let (sin_theta, cos_theta) = theta.sin_cos();

            for j in 0..self.phi_steps {
                let phi = j as f32 * phi_step;
                let (sin_phi, cos_phi) = phi.sin_cos();

                // Point on torus surface
                let circle_x = self.r2 + self.r1 * cos_theta;
                let circle_y = self.r1 * sin_theta;

                let position = Vec3::new(
                    circle_x * cos_phi,
                    circle_y,
                    circle_x * sin_phi,
                );

                // Surface normal (points outward from tube surface)
                let normal = Vec3::new(
                    cos_theta * cos_phi,
                    sin_theta,
                    cos_theta * sin_phi,
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
        // Use fewer lines for wireframe - about 8-12 lines each way
        let wire_theta_lines = 12;
        let wire_phi_lines = 8;
        let points_per_line = 60;

        // Draw rings around the tube (phi circles)
        for i in 0..wire_phi_lines {
            let phi = (i as f32 / wire_phi_lines as f32) * 2.0 * PI;
            let (sin_phi, cos_phi) = phi.sin_cos();

            for j in 0..points_per_line {
                let theta = (j as f32 / points_per_line as f32) * 2.0 * PI;
                let (sin_theta, cos_theta) = theta.sin_cos();

                let circle_x = self.r2 + self.r1 * cos_theta;
                let circle_y = self.r1 * sin_theta;

                let position = Vec3::new(
                    circle_x * cos_phi,
                    circle_y,
                    circle_x * sin_phi,
                );

                let normal = Vec3::new(cos_theta * cos_phi, sin_theta, cos_theta * sin_phi);

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

        // Draw circles along the tube (theta circles)
        for i in 0..wire_theta_lines {
            let theta = (i as f32 / wire_theta_lines as f32) * 2.0 * PI;
            let (sin_theta, cos_theta) = theta.sin_cos();

            for j in 0..points_per_line {
                let phi = (j as f32 / points_per_line as f32) * 2.0 * PI;
                let (sin_phi, cos_phi) = phi.sin_cos();

                let circle_x = self.r2 + self.r1 * cos_theta;
                let circle_y = self.r1 * sin_theta;

                let position = Vec3::new(
                    circle_x * cos_phi,
                    circle_y,
                    circle_x * sin_phi,
                );

                let normal = Vec3::new(cos_theta * cos_phi, sin_theta, cos_theta * sin_phi);

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
