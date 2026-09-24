pub mod math;
pub mod pixels;
pub mod rasterizer;

pub use math::{Camera, Mat3, Vec3, CHAR_ASPECT};
pub use pixels::PixelMode;
pub use rasterizer::{AsciiBuffer, CharStyle, ColorPalette, Fragment, RenderMode, Renderer};
