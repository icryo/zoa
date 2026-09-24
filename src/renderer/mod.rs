pub mod math;
pub mod rasterizer;

pub use math::{Camera, Mat3, Vec3, CHAR_ASPECT};
pub use rasterizer::{AsciiBuffer, CharStyle, ColorPalette, Fragment, RenderMode, Renderer};
