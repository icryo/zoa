//! A common interface for anything zoa can animate and draw.

use crate::renderer::{AsciiBuffer, Renderer};

/// Something that can be animated and rendered into an [`AsciiBuffer`].
///
/// All built-in shapes, meshes, particle systems, SDF scenes, GIFs and
/// countdowns implement this, and so can your own types; any `Scene` can be
/// shown in a [`ZoaWidget`](crate::ZoaWidget) via `set_scene`.
pub trait Scene {
    /// Advance the animation by `dt` seconds.
    fn update(&mut self, dt: f32);

    /// Draw into `buffer`. 3D scenes use the renderer's camera, light and
    /// render mode; 2D scenes may ignore it.
    fn render(&self, renderer: &Renderer, buffer: &mut AsciiBuffer);

    /// Manually rotate the scene, if it supports rotation.
    fn rotate(&mut self, _dx: f32, _dy: f32) {}
}
