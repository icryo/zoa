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

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};

use crate::renderer::{AsciiBuffer, CharStyle, ColorPalette, RenderMode, Renderer};
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
    pub density: usize,
    pub zoom: f32,
    pub auto_rotate: bool,
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
        }
    }
}

/// A ratatui Widget that renders a rotating 3D ASCII shape.
///
/// This widget can be easily embedded in any ratatui application.
/// Call `update(dt)` each frame to advance the animation.
pub struct ZoaWidget {
    torus: Torus,
    cube: Cube,
    sphere: Sphere,
    custom_mesh: Option<Mesh>,
    #[cfg(feature = "gif")]
    gif: Option<AnimatedGif>,
    countdown: Option<Countdown>,
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
            custom_mesh: None,
            #[cfg(feature = "gif")]
            gif: None,
            countdown: None,
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
        let mut widget = Self {
            custom_mesh: Some(mesh),
            ..Self::default()
        };
        widget.apply_config();
        widget
    }

    /// Get mutable access to config
    pub fn config_mut(&mut self) -> &mut ZoaConfig {
        &mut self.config
    }

    /// Get the current config
    pub fn config(&self) -> &ZoaConfig {
        &self.config
    }

    /// Set the shape to render
    pub fn set_shape(&mut self, shape: Shape) {
        self.config.shape = shape;
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

    /// Manually rotate the current shape
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        match self.config.shape {
            Shape::Torus => {
                self.torus.rotation.x += dx;
                self.torus.rotation.y += dy;
            }
            Shape::Cube => {
                self.cube.rotation.x += dx;
                self.cube.rotation.y += dy;
            }
            Shape::Sphere => {
                self.sphere.rotation.x += dx;
                self.sphere.rotation.y += dy;
            }
        }
        if let Some(ref mut mesh) = self.custom_mesh {
            mesh.rotation.x += dx;
            mesh.rotation.y += dy;
        }
    }

    /// Load a custom mesh from file
    pub fn load_mesh(&mut self, path: &std::path::Path) -> Result<(), String> {
        let mesh = Mesh::from_file(path)?;
        self.custom_mesh = Some(mesh);
        #[cfg(feature = "gif")]
        {
            self.gif = None; // Clear any loaded GIF
        }
        self.apply_config(); // Match the configured density and speed
        Ok(())
    }

    /// Load an animated GIF from file
    #[cfg(feature = "gif")]
    pub fn load_gif(&mut self, path: &std::path::Path) -> Result<(), String> {
        let gif = AnimatedGif::from_file(path)?;
        self.gif = Some(gif);
        self.custom_mesh = None; // Clear any loaded mesh
        Ok(())
    }

    /// Check if a GIF is currently loaded
    #[cfg(feature = "gif")]
    pub fn has_gif(&self) -> bool {
        self.gif.is_some()
    }

    /// Start a countdown timer with the given duration
    pub fn start_countdown(&mut self, duration: Duration) {
        let mut countdown = Countdown::new(duration);
        countdown.start();
        self.countdown = Some(countdown);
        #[cfg(feature = "gif")]
        {
            self.gif = None;
        }
        self.custom_mesh = None;
    }

    /// Parse and start a countdown from a string (e.g., "5m", "1:30")
    pub fn start_countdown_from_str(&mut self, duration_str: &str) -> Result<(), String> {
        let mut countdown = Countdown::parse(duration_str)?;
        countdown.start();
        self.countdown = Some(countdown);
        #[cfg(feature = "gif")]
        {
            self.gif = None;
        }
        self.custom_mesh = None;
        Ok(())
    }

    /// Check if a countdown is currently active
    pub fn has_countdown(&self) -> bool {
        self.countdown.is_some()
    }

    /// Toggle countdown pause/resume
    pub fn toggle_countdown_pause(&mut self) {
        if let Some(ref mut countdown) = self.countdown {
            countdown.toggle_pause();
        }
    }

    /// Reset countdown to initial duration
    pub fn reset_countdown(&mut self) {
        if let Some(ref mut countdown) = self.countdown {
            countdown.reset();
        }
    }

    fn apply_config(&mut self) {
        self.applied_config = self.config.clone();
        let density = self.config.density;
        let speed = self.config.speed;
        let zoom = self.config.zoom;

        // Apply density
        self.torus.set_detail(
            (50.0 * (density as f32 / 8.0)) as usize,
            (25.0 * (density as f32 / 8.0)) as usize,
        );
        self.cube.set_density(density * 3);
        self.sphere.set_detail(
            (40.0 * (density as f32 / 8.0)) as usize,
            (20.0 * (density as f32 / 8.0)) as usize,
        );
        if let Some(ref mut mesh) = self.custom_mesh {
            mesh.set_density(density);
        }

        // Apply speed
        self.torus.set_speed_multiplier(speed);
        self.cube.set_speed_multiplier(speed);
        self.sphere.set_speed_multiplier(speed);
        if let Some(ref mut mesh) = self.custom_mesh {
            mesh.set_speed_multiplier(speed);
        }

        // Apply zoom
        self.renderer.camera.scale = 40.0 * zoom;
        self.renderer.camera.distance = 5.0 / zoom;
    }

    /// Update animation state. Call this each frame with delta time in seconds.
    pub fn update(&mut self, dt: f32) {
        if self.config != self.applied_config {
            self.apply_config();
        }

        // GIF always animates (it's frame-based, not rotation)
        #[cfg(feature = "gif")]
        if let Some(ref mut gif) = self.gif {
            gif.update(dt);
            return;
        }

        // Countdown always updates
        if let Some(ref mut countdown) = self.countdown {
            countdown.update(dt);
            return;
        }

        if self.config.auto_rotate {
            match self.config.shape {
                Shape::Torus => self.torus.update(dt),
                Shape::Cube => self.cube.update(dt),
                Shape::Sphere => self.sphere.update(dt),
            }
            if let Some(ref mut mesh) = self.custom_mesh {
                mesh.update(dt);
            }
        }
    }

    fn render_to_buffer(&mut self, width: u16, height: u16) {
        self.buffer.resize(width, height);
        self.buffer.clear();

        // GIF renders directly to buffer (handles its own scaling)
        #[cfg(feature = "gif")]
        if let Some(ref gif) = self.gif {
            gif.render(&mut self.buffer);
            return;
        }

        // Countdown renders directly to buffer
        if let Some(ref countdown) = self.countdown {
            countdown.render(&mut self.buffer);
            return;
        }

        let mode = self.config.render_mode;

        if let Some(ref mesh) = self.custom_mesh {
            mesh.render(&self.renderer, &mut self.buffer);
        } else {
            match self.config.shape {
                Shape::Torus => self.torus.render_with_mode(&self.renderer, &mut self.buffer, mode),
                Shape::Cube => self.cube.render_with_mode(&self.renderer, &mut self.buffer, mode),
                Shape::Sphere => self.sphere.render_with_mode(&self.renderer, &mut self.buffer, mode),
            }
        }
    }
}

impl Widget for &mut ZoaWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_to_buffer(area.width, area.height);

        for y in 0..area.height.min(self.buffer.height) {
            for x in 0..area.width.min(self.buffer.width) {
                if let Some(fragment) = self.buffer.get(x, y) {
                    let ch = self.config.char_style.to_char(fragment.luminance);
                    let color = self.config.palette.to_color(fragment.luminance);
                    buf[(area.x + x, area.y + y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(color));
                }
            }
        }
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
        assert!(widget.custom_mesh.is_none());
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
