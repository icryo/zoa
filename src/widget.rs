//! Embeddable ratatui widget for 3D ASCII rendering.
//!
//! # Example
//!
//! ```no_run
//! use zoa::ZoaWidget;
//! use ratatui::Frame;
//!
//! fn draw(frame: &mut Frame, widget: &mut ZoaWidget) {
//!     widget.update(0.016); // ~60fps delta
//!     frame.render_widget(widget, frame.area());
//! }
//! ```

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::error::Result;
use crate::renderer::{AsciiBuffer, CharStyle, ColorPalette, RenderMode, Renderer};
use crate::scene::Scene;
#[cfg(feature = "gif")]
use crate::shapes::AnimatedGif;
use crate::shapes::{Countdown, Cube, Mesh, Sphere, Torus};
use std::time::Duration;

/// Which shape to render
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Shape {
    #[default]
    Torus,
    Cube,
    Sphere,
}

impl Shape {
    pub fn next(self) -> Self {
        match self {
            Self::Torus => Self::Cube,
            Self::Cube => Self::Sphere,
            Self::Sphere => Self::Torus,
        }
    }
}

/// Configuration for the ZoaWidget
#[derive(Debug, Clone, PartialEq)]
pub struct ZoaConfig {
    pub shape: Shape,
    pub char_style: CharStyle,
    pub palette: ColorPalette,
    pub render_mode: RenderMode,
    pub speed: f32,
    /// Tessellation detail for the torus and sphere (8 = default)
    pub density: usize,
    pub zoom: f32,
    pub auto_rotate: bool,
    /// Show GIFs in their own colors instead of the palette
    pub true_color: bool,
}

impl Default for ZoaConfig {
    fn default() -> Self {
        Self {
            shape: Shape::default(),
            char_style: CharStyle::default(),
            palette: ColorPalette::default(),
            render_mode: RenderMode::default(),
            speed: 1.0,
            density: 8,
            zoom: 1.0,
            auto_rotate: true,
            true_color: false,
        }
    }
}

/// Content shown instead of the built-in shape
enum Content {
    Mesh(Mesh),
    #[cfg(feature = "gif")]
    Gif(AnimatedGif),
    Countdown(Countdown),
    Custom(Box<dyn Scene + Send>),
}

impl Content {
    fn scene(&self) -> &dyn Scene {
        match self {
            Self::Mesh(mesh) => mesh,
            #[cfg(feature = "gif")]
            Self::Gif(gif) => gif,
            Self::Countdown(countdown) => countdown,
            Self::Custom(scene) => scene.as_ref(),
        }
    }

    fn scene_mut(&mut self) -> &mut dyn Scene {
        match self {
            Self::Mesh(mesh) => mesh,
            #[cfg(feature = "gif")]
            Self::Gif(gif) => gif,
            Self::Countdown(countdown) => countdown,
            Self::Custom(scene) => scene.as_mut(),
        }
    }

    /// Whether this content only moves when auto-rotation is on
    fn is_rotating(&self) -> bool {
        matches!(self, Self::Mesh(_))
    }
}

/// A ratatui Widget that renders a rotating 3D ASCII shape.
///
/// This widget can be easily embedded in any ratatui application.
/// Call `update(dt)` each frame to advance the animation. Besides the
/// built-in shapes it can show a mesh, a GIF, a countdown, or any
/// [`Scene`] (particles, SDF scenes, or your own) via [`set_scene`](Self::set_scene).
pub struct ZoaWidget {
    torus: Torus,
    cube: Cube,
    sphere: Sphere,
    content: Option<Content>,
    renderer: Renderer,
    buffer: AsciiBuffer,
    config: ZoaConfig,
    /// Config last pushed into the shapes, so `config_mut` edits get picked up
    applied_config: ZoaConfig,
}

impl Default for ZoaWidget {
    fn default() -> Self {
        Self::new(ZoaConfig::default())
    }
}

impl ZoaWidget {
    /// Create a new ZoaWidget with the given configuration
    pub fn new(config: ZoaConfig) -> Self {
        let mut widget = Self {
            torus: Torus::default(),
            cube: Cube::default(),
            sphere: Sphere::default(),
            content: None,
            renderer: Renderer::default(),
            buffer: AsciiBuffer::new(80, 24),
            applied_config: config.clone(),
            config,
        };
        widget.apply_config();
        widget
    }

    /// Create with a custom mesh
    pub fn with_mesh(mesh: Mesh) -> Self {
        let mut widget = Self::default();
        widget.set_content(Content::Mesh(mesh));
        widget
    }

    /// Get mutable access to config. Changes take effect on the next `update`.
    pub fn config_mut(&mut self) -> &mut ZoaConfig {
        &mut self.config
    }

    /// Get the current config
    pub fn config(&self) -> &ZoaConfig {
        &self.config
    }

    /// Set the built-in shape to render (clears any loaded content)
    pub fn set_shape(&mut self, shape: Shape) {
        self.config.shape = shape;
        self.content = None;
    }

    /// Set the character style
    pub fn set_char_style(&mut self, style: CharStyle) {
        self.config.char_style = style;
    }

    /// Set the color palette
    pub fn set_palette(&mut self, palette: ColorPalette) {
        self.config.palette = palette;
    }

    /// Set the render mode (solid or wireframe)
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.config.render_mode = mode;
        self.apply_config();
    }

    /// Set zoom level (0.3 to 3.0)
    pub fn set_zoom(&mut self, zoom: f32) {
        self.config.zoom = zoom.clamp(0.3, 3.0);
        self.apply_config();
    }

    /// Set animation speed multiplier
    pub fn set_speed(&mut self, speed: f32) {
        self.config.speed = speed.clamp(0.1, 5.0);
        self.apply_config();
    }

    /// Toggle auto-rotation
    pub fn set_auto_rotate(&mut self, auto: bool) {
        self.config.auto_rotate = auto;
    }

    /// Manually rotate whatever is currently shown
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.active_scene_mut().rotate(dx, dy);
    }

    /// Show any [`Scene`] (e.g. a `ParticleSystem`, `SdfScene`, or your own
    /// type) instead of the built-in shape
    pub fn set_scene(&mut self, scene: impl Scene + Send + 'static) {
        self.set_content(Content::Custom(Box::new(scene)));
    }

    /// Remove any loaded mesh, GIF, countdown or scene, returning to the
    /// built-in shape
    pub fn clear_content(&mut self) {
        self.content = None;
    }

    /// Load a custom mesh from file
    pub fn load_mesh(&mut self, path: &std::path::Path) -> Result<()> {
        self.set_content(Content::Mesh(Mesh::from_file(path)?));
        Ok(())
    }

    /// Load an animated GIF from file
    #[cfg(feature = "gif")]
    pub fn load_gif(&mut self, path: &std::path::Path) -> Result<()> {
        self.set_content(Content::Gif(AnimatedGif::from_file(path)?));
        Ok(())
    }

    /// Check if a GIF is currently loaded
    #[cfg(feature = "gif")]
    pub fn has_gif(&self) -> bool {
        matches!(self.content, Some(Content::Gif(_)))
    }

    /// Check if a custom mesh is currently loaded
    pub fn has_mesh(&self) -> bool {
        matches!(self.content, Some(Content::Mesh(_)))
    }

    /// Start a countdown timer with the given duration
    pub fn start_countdown(&mut self, duration: Duration) {
        let mut countdown = Countdown::new(duration);
        countdown.start();
        self.set_content(Content::Countdown(countdown));
    }

    /// Parse and start a countdown from a string (e.g., "5m", "1:30")
    pub fn start_countdown_from_str(&mut self, duration_str: &str) -> Result<()> {
        let mut countdown = Countdown::parse(duration_str)?;
        countdown.start();
        self.set_content(Content::Countdown(countdown));
        Ok(())
    }

    /// Check if a countdown is currently active
    pub fn has_countdown(&self) -> bool {
        matches!(self.content, Some(Content::Countdown(_)))
    }

    /// Toggle countdown pause/resume
    pub fn toggle_countdown_pause(&mut self) {
        if let Some(countdown) = self.countdown_mut() {
            countdown.toggle_pause();
        }
    }

    /// Reset countdown to initial duration
    pub fn reset_countdown(&mut self) {
        if let Some(countdown) = self.countdown_mut() {
            countdown.reset();
        }
    }

    fn countdown_mut(&mut self) -> Option<&mut Countdown> {
        match &mut self.content {
            Some(Content::Countdown(countdown)) => Some(countdown),
            _ => None,
        }
    }

    fn set_content(&mut self, content: Content) {
        self.content = Some(content);
        self.apply_config(); // Match the configured detail, speed and zoom
    }

    fn active_scene(&self) -> &dyn Scene {
        match &self.content {
            Some(content) => content.scene(),
            None => match self.config.shape {
                Shape::Torus => &self.torus,
                Shape::Cube => &self.cube,
                Shape::Sphere => &self.sphere,
            },
        }
    }

    fn active_scene_mut(&mut self) -> &mut dyn Scene {
        match &mut self.content {
            Some(content) => content.scene_mut(),
            None => match self.config.shape {
                Shape::Torus => &mut self.torus,
                Shape::Cube => &mut self.cube,
                Shape::Sphere => &mut self.sphere,
            },
        }
    }

    fn apply_config(&mut self) {
        self.applied_config = self.config.clone();
        let ZoaConfig { density, speed, zoom, render_mode, .. } = self.config;
        let detail = density as f32 / 8.0;

        self.torus.set_detail((50.0 * detail) as usize, (25.0 * detail) as usize);
        self.sphere.set_detail((40.0 * detail) as usize, (20.0 * detail) as usize);

        self.torus.set_speed_multiplier(speed);
        self.cube.set_speed_multiplier(speed);
        self.sphere.set_speed_multiplier(speed);

        match &mut self.content {
            Some(Content::Mesh(mesh)) => mesh.set_speed_multiplier(speed),
            #[cfg(feature = "gif")]
            Some(Content::Gif(gif)) => {
                gif.set_scale(zoom);
                gif.set_true_color(self.config.true_color);
            }
            Some(Content::Countdown(countdown)) => countdown.set_scale(zoom),
            Some(Content::Custom(_)) | None => {}
        }

        self.renderer.camera.scale = zoom;
        self.renderer.camera.distance = 5.0 / zoom;
        self.renderer.mode = render_mode;
    }

    /// Update animation state. Call this each frame with delta time in seconds.
    pub fn update(&mut self, dt: f32) {
        if self.config != self.applied_config {
            self.apply_config();
        }

        // Rotating shapes pause when auto-rotate is off; GIFs, countdowns and
        // custom scenes always animate
        let rotating = self.content.as_ref().is_none_or(Content::is_rotating);
        if self.config.auto_rotate || !rotating {
            self.active_scene_mut().update(dt);
        }
    }

    fn render_to_buffer(&mut self, width: u16, height: u16) {
        // Move the buffer out so the scene can be borrowed alongside it
        let mut buffer = std::mem::replace(&mut self.buffer, AsciiBuffer::new(0, 0));
        buffer.resize(width, height);
        buffer.clear();
        self.active_scene().render(&self.renderer, &mut buffer);
        self.buffer = buffer;
    }
}

impl Widget for &mut ZoaWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_to_buffer(area.width, area.height);
        self.buffer.draw(area, buf, self.config.char_style, self.config.palette);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "gif")]
    use std::path::Path;

    #[cfg(feature = "gif")]
    #[test]
    fn test_widget_load_gif() {
        let gif_path = Path::new("samples/test.gif");
        if !gif_path.exists() {
            return; // Skip if test file not present
        }

        let mut widget = ZoaWidget::default();
        assert!(!widget.has_gif());

        widget.load_gif(gif_path).expect("Failed to load GIF");
        assert!(widget.has_gif());

        // Test that update advances animation
        widget.update(0.1);
        widget.update(0.1);

        // Test rendering at different sizes
        for (w, h) in [(80, 24), (120, 40), (40, 20)] {
            widget.render_to_buffer(w, h);
        }
    }

    #[cfg(feature = "gif")]
    #[test]
    fn test_widget_gif_clears_mesh() {
        let gif_path = Path::new("samples/test.gif");
        if !gif_path.exists() {
            return;
        }

        let mut widget = ZoaWidget::default();

        // Loading GIF should work even without prior mesh
        widget.load_gif(gif_path).expect("Failed to load GIF");
        assert!(widget.has_gif());
        assert!(!widget.has_mesh());
    }

    #[test]
    fn test_tiny_buffer_sizes() {
        // Test that rendering handles tiny/zero-size buffers gracefully
        let mut widget = ZoaWidget::default();

        // Test all shapes at extreme sizes
        for shape in [Shape::Torus, Shape::Cube, Shape::Sphere] {
            widget.set_shape(shape);
            // Zero-size buffers
            widget.render_to_buffer(0, 0);
            widget.render_to_buffer(0, 10);
            widget.render_to_buffer(10, 0);
            // Tiny buffers
            widget.render_to_buffer(1, 1);
            widget.render_to_buffer(2, 1);
            widget.render_to_buffer(1, 2);
            widget.render_to_buffer(3, 3);
        }

        // Test countdown at tiny sizes
        widget.start_countdown_from_str("1:00").unwrap();
        widget.render_to_buffer(0, 0);
        widget.render_to_buffer(1, 1);
        widget.render_to_buffer(2, 2);
        widget.render_to_buffer(5, 3);

        // Test GIF at tiny sizes if available
        #[cfg(feature = "gif")]
        {
            let gif_path = Path::new("samples/test.gif");
            if gif_path.exists() {
                let mut widget = ZoaWidget::default();
                widget.load_gif(gif_path).unwrap();
                widget.render_to_buffer(0, 0);
                widget.render_to_buffer(1, 1);
                widget.render_to_buffer(3, 2);
            }
        }
    }
}
