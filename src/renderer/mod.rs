pub mod math;
pub mod rasterizer;

pub use math::{Camera, Vec3};
pub use rasterizer::{AsciiBuffer, CharStyle, ColorPalette, Fragment, RenderMode, Renderer};
