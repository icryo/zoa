pub mod countdown;
pub mod cube;
#[cfg(feature = "gif")]
pub mod gif;
pub mod mesh;
pub mod particles;
pub mod sdf;
pub mod sphere;
pub mod torus;

pub use countdown::{Countdown, CountdownState};
pub use cube::Cube;
#[cfg(feature = "gif")]
pub use gif::AnimatedGif;
pub use mesh::{Mesh, Triangle};
pub use particles::{ParticlePreset, ParticleSystem};
pub use sdf::{SdfPreset, SdfScene};
pub use sphere::Sphere;
pub use torus::Torus;

use crate::renderer::{AsciiBuffer, Mat3, RenderMode, Renderer, Vec3};
use std::time::Duration;

/// Convert a frame delta to a `Duration`, treating negative, NaN, or
/// out-of-range values as zero instead of panicking.
pub(crate) fn dt_to_duration(dt: f32) -> Duration {
    Duration::try_from_secs_f32(dt).unwrap_or(Duration::ZERO)
}

/// Implement `Scene` for a rotating shape with inherent `update`/`render`
/// methods and a `rotation` field.
macro_rules! impl_rotating_scene {
    ($ty:ty) => {
        impl crate::scene::Scene for $ty {
            fn update(&mut self, dt: f32) {
                <$ty>::update(self, dt)
            }

            fn render(&self, renderer: &crate::renderer::Renderer, buffer: &mut crate::renderer::AsciiBuffer) {
                <$ty>::render(self, renderer, buffer)
            }

            fn rotate(&mut self, dx: f32, dy: f32) {
                self.rotation.x += dx;
                self.rotation.y += dy;
            }
        }
    };
}
pub(crate) use impl_rotating_scene;

/// A smooth parametric surface `f(u, v) -> (position, unit normal)` with
/// `u, v` in `[0, 1]`, rotated by `rotation` before drawing.
pub(crate) struct Surface<F: Fn(f32, f32) -> (Vec3, Vec3)> {
    pub sample: F,
    pub rotation: Mat3,
}

impl<F: Fn(f32, f32) -> (Vec3, Vec3)> Surface<F> {
    fn sample(&self, u: f32, v: f32) -> (Vec3, Vec3) {
        let (p, n) = (self.sample)(u, v);
        (self.rotation.transform(p), self.rotation.transform(n))
    }

    /// Tessellate into a `u_steps` x `v_steps` grid of quads and rasterize.
    pub fn render_solid(&self, renderer: &Renderer, buffer: &mut AsciiBuffer, u_steps: usize, v_steps: usize) {
        let (u_steps, v_steps) = (u_steps.max(1), v_steps.max(1));
        let cols = v_steps + 1;
        let grid: Vec<(Vec3, Vec3)> = (0..=u_steps)
            .flat_map(|i| (0..=v_steps).map(move |j| (i, j)))
            .map(|(i, j)| self.sample(i as f32 / u_steps as f32, j as f32 / v_steps as f32))
            .collect();

        for i in 0..u_steps {
            for j in 0..v_steps {
                let (a, b) = (grid[i * cols + j], grid[i * cols + j + 1]);
                let (c, d) = (grid[(i + 1) * cols + j], grid[(i + 1) * cols + j + 1]);
                renderer.draw_triangle(buffer, [a.0, b.0, d.0], [a.1, b.1, d.1]);
                renderer.draw_triangle(buffer, [a.0, d.0, c.0], [a.1, d.1, c.1]);
            }
        }
    }

    /// Draw iso-parameter lines: constant-`u` lines at `u_lines` and
    /// constant-`v` lines at `v_lines`, each as `segments` line segments.
    pub fn render_wireframe(
        &self,
        renderer: &Renderer,
        buffer: &mut AsciiBuffer,
        u_lines: impl IntoIterator<Item = f32>,
        v_lines: impl IntoIterator<Item = f32>,
        segments: usize,
    ) {
        let mut polyline = |point: &dyn Fn(f32) -> (Vec3, Vec3)| {
            let mut prev = point(0.0);
            for k in 1..=segments {
                let next = point(k as f32 / segments as f32);
                renderer.draw_line(buffer, prev.0, next.0, prev.1, next.1);
                prev = next;
            }
        };
        for u in u_lines {
            polyline(&|t| self.sample(u, t));
        }
        for v in v_lines {
            polyline(&|t| self.sample(t, v));
        }
    }

    pub fn render(&self, renderer: &Renderer, buffer: &mut AsciiBuffer, detail: (usize, usize), wire: (usize, usize)) {
        match renderer.mode {
            RenderMode::Solid => self.render_solid(renderer, buffer, detail.0, detail.1),
            RenderMode::Wireframe => self.render_wireframe(
                renderer,
                buffer,
                (0..wire.0).map(|i| i as f32 / wire.0 as f32),
                (0..wire.1).map(|i| i as f32 / wire.1 as f32),
                60,
            ),
        }
    }
}
