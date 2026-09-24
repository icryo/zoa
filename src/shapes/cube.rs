use crate::renderer::{AsciiBuffer, RenderMode, Renderer, Vec3};

#[derive(Debug, Clone)]
pub struct Cube {
    pub size: f32,
    pub rotation: Vec3,
    pub rotation_speed: Vec3,
    density: usize, // points per edge
}

impl Default for Cube {
    fn default() -> Self {
        Self {
            size: 2.0,
            rotation: Vec3::default(),
            rotation_speed: Vec3::new(0.5, 0.7, 0.3),
            density: 30,
        }
    }
}

impl Cube {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    pub fn with_density(mut self, density: usize) -> Self {
        self.density = density.max(1);
        self
    }

    pub fn set_density(&mut self, density: usize) {
        self.density = density.max(5);
    }

    pub fn set_speed_multiplier(&mut self, multiplier: f32) {
        self.rotation_speed = Vec3::new(0.5, 0.7, 0.3) * multiplier;
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
        let half = self.size / 2.0;
        let step = self.size / self.density as f32;

        // Render each face
        self.render_face(renderer, buffer, half, step, Axis::X, true);
        self.render_face(renderer, buffer, half, step, Axis::X, false);
        self.render_face(renderer, buffer, half, step, Axis::Y, true);
        self.render_face(renderer, buffer, half, step, Axis::Y, false);
        self.render_face(renderer, buffer, half, step, Axis::Z, true);
        self.render_face(renderer, buffer, half, step, Axis::Z, false);
    }

    fn render_wireframe(&self, renderer: &Renderer, buffer: &mut AsciiBuffer) {
        let half = self.size / 2.0;
        let points_per_edge = 40;

        // Define the 8 vertices
        let vertices = [
            Vec3::new(-half, -half, -half),
            Vec3::new( half, -half, -half),
            Vec3::new( half,  half, -half),
            Vec3::new(-half,  half, -half),
            Vec3::new(-half, -half,  half),
            Vec3::new( half, -half,  half),
            Vec3::new( half,  half,  half),
            Vec3::new(-half,  half,  half),
        ];

        // Define the 12 edges (vertex index pairs)
        let edges = [
            (0, 1), (1, 2), (2, 3), (3, 0), // front face
            (4, 5), (5, 6), (6, 7), (7, 4), // back face
            (0, 4), (1, 5), (2, 6), (3, 7), // connecting edges
        ];

        for (start, end) in edges {
            let v0 = vertices[start];
            let v1 = vertices[end];

            // Calculate edge direction for normal approximation
            let edge_dir = (v1 - v0).normalize();
            // Use a normal that's somewhat visible from multiple angles
            let normal = Vec3::new(
                if edge_dir.x.abs() < 0.5 { 1.0 } else { 0.0 },
                if edge_dir.y.abs() < 0.5 { 1.0 } else { 0.0 },
                if edge_dir.z.abs() < 0.5 { 1.0 } else { 0.0 },
            ).normalize();

            for i in 0..=points_per_edge {
                let t = i as f32 / points_per_edge as f32;
                let position = v0 + (v1 - v0) * t;

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

    fn render_face(
        &self,
        renderer: &Renderer,
        buffer: &mut AsciiBuffer,
        half: f32,
        step: f32,
        axis: Axis,
        positive: bool,
    ) {
        let fixed = if positive { half } else { -half };
        let normal_sign = if positive { 1.0 } else { -1.0 };

        let normal = match axis {
            Axis::X => Vec3::new(normal_sign, 0.0, 0.0),
            Axis::Y => Vec3::new(0.0, normal_sign, 0.0),
            Axis::Z => Vec3::new(0.0, 0.0, normal_sign),
        };

        // Integer steps so float accumulation can't drop the far edge row
        let steps = (self.size / step).round() as usize;
        for i in 0..=steps {
            let u = -half + i as f32 * step;
            for j in 0..=steps {
                let v = -half + j as f32 * step;
                let position = match axis {
                    Axis::X => Vec3::new(fixed, u, v),
                    Axis::Y => Vec3::new(u, fixed, v),
                    Axis::Z => Vec3::new(u, v, fixed),
                };

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

enum Axis {
    X,
    Y,
    Z,
}
