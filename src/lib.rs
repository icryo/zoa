//! # zoa
//!
//! A 3D ASCII renderer library for terminal UIs.
//!
//! Zoa provides tools to render 3D shapes as ASCII art in the terminal,
//! with support for custom models (OBJ/STL), multiple character styles,
//! color palettes, and visual effects.
//!
//! ## Quick Start
//!
//! The easiest way to embed zoa in your ratatui app:
//!
//! ```no_run
//! use zoa::ZoaWidget;
//! use ratatui::Frame;
//!
//! let mut widget = ZoaWidget::default();
//!
//! // In your render loop:
//! fn draw(frame: &mut Frame, widget: &mut ZoaWidget) {
//!     widget.update(0.016); // delta time in seconds
//!     frame.render_widget(widget, frame.area());
//! }
//! ```
//!
//! ## Low-level API
//!
//! For more control, use the renderer directly:
//!
//! ```no_run
//! use zoa::{Renderer, AsciiBuffer, Torus, CharStyle, ColorPalette};
//!
//! let renderer = Renderer::default();
//! let mut buffer = AsciiBuffer::new(80, 24);
//! let torus = Torus::default();
//! torus.render(&renderer, &mut buffer);
//!
//! for y in 0..buffer.height {
//!     for x in 0..buffer.width {
//!         if let Some(fragment) = buffer.get(x, y) {
//!             let ch = CharStyle::Ascii.to_char(fragment.luminance);
//!             let color = ColorPalette::Cyan.to_color(fragment.luminance);
//!             // render ch with color...
//!         }
//!     }
//! }
//! ```

pub mod renderer;
pub mod shapes;
pub mod widget;

// Re-export main types for convenience
pub use renderer::{AsciiBuffer, Camera, CharStyle, ColorPalette, Fragment, RenderMode, Renderer, Vec3};
#[cfg(feature = "gif")]
pub use shapes::AnimatedGif;
pub use shapes::{Countdown, Cube, Mesh, ParticlePreset, ParticleSystem, SdfPreset, SdfScene, Sphere, Torus, Triangle};
pub use widget::{ZoaConfig, ZoaWidget, Shape};
