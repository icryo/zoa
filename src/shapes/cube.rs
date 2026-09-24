use super::impl_rotating_scene;
use crate::renderer::{AsciiBuffer, Mat3, RenderMode, Renderer, Vec3};

#[derive(Debug, Clone)]
pub struct Cube {
    pub size: f32,
    pub rotation: Vec3,
    pub rotation_speed: Vec3,
}

impl Default for Cube {
    fn default() -> Self {
        Self {
            size: 2.0,
            rotation: Vec3::default(),
            rotation_speed: Vec3::new(0.5, 0.7, 0.3),
        }
    }
}

/// Corner indices of each face (counter-clockwise from outside) and its normal
const FACES: [([usize; 4], Vec3); 6] = [
    ([1, 2, 6, 5], Vec3::new(1.0, 0.0, 0.0)),
    ([0, 4, 7, 3], Vec3::new(-1.0, 0.0, 0.0)),
    ([3, 7, 6, 2], Vec3::new(0.0, 1.0, 0.0)),
    ([0, 1, 5, 4], Vec3::new(0.0, -1.0, 0.0)),
    ([4, 5, 6, 7], Vec3::new(0.0, 0.0, 1.0)),
    ([0, 3, 2, 1], Vec3::new(0.0, 0.0, -1.0)),
];

/// Pairs of corner indices joined by an edge
const EDGES: [(usize, usize); 12] = [
    (0, 1), (1, 2), (2, 3), (3, 0),
    (4, 5), (5, 6), (6, 7), (7, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
];

impl Cube {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            ..Default::default()
        }
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
        let h = self.size / 2.0;
        let rot = Mat3::from_rotation(self.rotation);
        let corners = [
            Vec3::new(-h, -h, -h),
            Vec3::new(h, -h, -h),
            Vec3::new(h, h, -h),
            Vec3::new(-h, h, -h),
            Vec3::new(-h, -h, h),
            Vec3::new(h, -h, h),
            Vec3::new(h, h, h),
            Vec3::new(-h, h, h),
        ]
        .map(|c| rot.transform(c));

        match renderer.mode {
            RenderMode::Solid => {
                for (idx, normal) in FACES {
                    let n = rot.transform(normal);
                    let [a, b, c, d] = idx.map(|i| corners[i]);
                    renderer.draw_triangle(buffer, [a, b, c], [n; 3]);
                    renderer.draw_triangle(buffer, [a, c, d], [n; 3]);
                }
            }
            RenderMode::Wireframe => {
                for (start, end) in EDGES {
                    // An edge's midpoint direction is the average of its two faces' normals
                    let normal = (corners[start] + corners[end]).normalize();
                    renderer.draw_line(buffer, corners[start], corners[end], normal, normal);
                }
            }
        }
    }
}

impl_rotating_scene!(Cube);
