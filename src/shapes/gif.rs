//! GIF animation support for zoa
//!
//! This module provides animated GIF playback as ASCII art.
//! Frames are converted to ASCII at render time to adapt to terminal size.

use crate::error::{Error, Result};
use crate::renderer::{AsciiBuffer, Fragment, Renderer};
use ratatui::style::Color;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

/// A single frame of the GIF animation
struct GifFrame {
    /// RGBA image data
    image: image::RgbaImage,
    /// Frame duration
    delay: Duration,
}

/// Animated ASCII art from a GIF file
pub struct AnimatedGif {
    frames: Vec<GifFrame>,
    current_frame: usize,
    elapsed: Duration,
    pub name: String,
    /// Original GIF dimensions
    pub width: u32,
    pub height: u32,
    /// For compatibility - always false now
    pub used_chafa: bool,
    /// Zoom/scale factor (1.0 = fit to buffer, >1 = larger, <1 = smaller)
    scale: f32,
    /// Use the GIF's own colors instead of the color palette
    true_color: bool,
}

impl AnimatedGif {
    /// Load a GIF file
    pub fn from_file(path: &Path) -> Result<Self> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Animation")
            .to_string();

        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        let decoder =
            image::codecs::gif::GifDecoder::new(reader)?;

        use image::AnimationDecoder;
        let raw_frames: Vec<image::Frame> = decoder
            .into_frames()
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if raw_frames.is_empty() {
            return Err(Error::InvalidData("no frames in GIF".into()));
        }

        // Get dimensions from first frame
        let first = &raw_frames[0];
        let width = first.buffer().width();
        let height = first.buffer().height();

        let frames: Vec<GifFrame> = raw_frames
            .into_iter()
            .map(|frame| {
                let delay = Duration::from(frame.delay());
                GifFrame {
                    image: frame.into_buffer(),
                    delay: if delay.as_millis() == 0 {
                        Duration::from_millis(100)
                    } else {
                        delay
                    },
                }
            })
            .collect();

        Ok(Self {
            frames,
            current_frame: 0,
            elapsed: Duration::ZERO,
            name,
            width,
            height,
            used_chafa: false,
            scale: 1.0,
            true_color: false,
        })
    }

    /// Set zoom/scale factor (0.3 to 3.0)
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale.clamp(0.3, 3.0);
    }

    /// Show the GIF's own colors instead of mapping brightness through the
    /// color palette
    pub fn set_true_color(&mut self, true_color: bool) {
        self.true_color = true_color;
    }

    /// Get current scale factor
    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    /// Update animation state
    pub fn update(&mut self, dt: f32) {
        self.elapsed += crate::shapes::dt_to_duration(dt);

        // Carry leftover time into the next frame (and skip frames on long
        // updates) so playback speed matches the GIF's timing.
        while let Some(frame) = self.frames.get(self.current_frame) {
            if self.elapsed < frame.delay {
                break;
            }
            self.elapsed -= frame.delay;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }

    /// Render current frame to ASCII buffer, adapting to buffer size
    pub fn render(&self, buffer: &mut AsciiBuffer) {
        let buf_width = buffer.width as usize;
        let buf_height = buffer.height as usize;

        // Guard against empty or tiny buffers
        if buf_width < 1 || buf_height < 1 {
            return;
        }

        let Some(frame) = self.frames.get(self.current_frame) else {
            return;
        };

        let img = &frame.image;
        let img_width = img.width() as usize;
        let img_height = img.height() as usize;

        if img_width == 0 || img_height == 0 {
            return;
        }

        // Calculate scaling to fit buffer while maintaining aspect ratio
        // Terminal chars are ~2x taller than wide, so we compensate
        let scale_x = img_width as f32 / buf_width as f32;
        let scale_y = (img_height as f32 / buf_height as f32) * 0.5;
        // Apply user zoom: higher self.scale = larger output (divide base scale)
        let scale = (scale_x.max(scale_y) / self.scale).max(0.1);

        let out_width = ((img_width as f32 / scale) as usize).min(buf_width).max(1);
        let out_height = ((img_height as f32 / scale * 0.5) as usize).min(buf_height).max(1);

        // Center in buffer
        let offset_x = buf_width.saturating_sub(out_width) / 2;
        let offset_y = buf_height.saturating_sub(out_height) / 2;

        for y in 0..out_height {
            let buf_y = offset_y + y;
            if buf_y >= buf_height {
                break;
            }

            for x in 0..out_width {
                let buf_x = offset_x + x;
                if buf_x >= buf_width {
                    break;
                }

                // Average the source pixels this cell covers (box filter), so
                // downscaling doesn't alias or drop thin details
                let x0 = ((x as f32 * scale) as u32).min(img.width() - 1);
                let y0 = ((y as f32 * scale * 2.0) as u32).min(img.height() - 1);
                let x1 = (((x + 1) as f32 * scale) as u32).clamp(x0 + 1, img.width());
                let y1 = (((y + 1) as f32 * scale * 2.0) as u32).clamp(y0 + 1, img.height());
                // Cap the work per cell for very large GIFs
                let step_x = ((x1 - x0) / 8).max(1) as usize;
                let step_y = ((y1 - y0) / 8).max(1) as usize;

                let (mut sum, mut alpha, mut count) = ([0.0f32; 3], 0.0f32, 0.0f32);
                for sy in (y0..y1).step_by(step_y) {
                    for sx in (x0..x1).step_by(step_x) {
                        let [r, g, b, a] = img.get_pixel(sx, sy).0;
                        let a = a as f32 / 255.0;
                        sum[0] += r as f32 * a;
                        sum[1] += g as f32 * a;
                        sum[2] += b as f32 * a;
                        alpha += a;
                        count += 1.0;
                    }
                }

                // Mostly transparent -> leave the cell empty
                if alpha < count * 0.5 {
                    continue;
                }
                let [r, g, b] = sum.map(|c| c / alpha);
                let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;

                let mut fragment = Fragment::new(lum);
                if self.true_color {
                    fragment = fragment.with_color(Color::Rgb(r as u8, g as u8, b as u8));
                }
                // Use high depth so GIF is always visible
                buffer.plot_fragment(buf_x as u16, buf_y as u16, 100.0, fragment);
            }
        }
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

impl crate::scene::Scene for AnimatedGif {
    fn update(&mut self, dt: f32) {
        AnimatedGif::update(self, dt)
    }

    fn render(&self, _renderer: &Renderer, buffer: &mut AsciiBuffer) {
        AnimatedGif::render(self, buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_gif_scaling_logic() {
        // Test the scaling math at various buffer sizes
        let img_width = 498usize;
        let img_height = 498usize;

        let test_cases = [
            (80, 24, "small"),
            (120, 40, "medium"),
            (200, 60, "large"),
            (40, 20, "tiny"),
        ];

        for (buf_w, buf_h, _) in test_cases {
            let scale_x = img_width as f32 / buf_w as f32;
            let scale_y = (img_height as f32 / buf_h as f32) * 0.5;
            let scale = scale_x.max(scale_y).max(1.0);

            let out_w = ((img_width as f32 / scale) as usize).min(buf_w);
            let out_h = ((img_height as f32 / scale * 0.5) as usize).min(buf_h);

            // Output should fit within buffer
            assert!(out_w <= buf_w, "Output width exceeds buffer");
            assert!(out_h <= buf_h, "Output height exceeds buffer");
            // Output should use reasonable portion of buffer
            assert!(out_w > buf_w / 4, "Output too small horizontally");
            assert!(out_h > buf_h / 4, "Output too small vertically");
        }
    }

    #[test]
    fn test_frame_timing_carries_remainder() {
        let frame = |ms| GifFrame {
            image: image::RgbaImage::new(1, 1),
            delay: Duration::from_millis(ms),
        };
        let mut gif = AnimatedGif {
            frames: vec![frame(100), frame(100), frame(100)],
            current_frame: 0,
            elapsed: Duration::ZERO,
            name: String::new(),
            width: 1,
            height: 1,
            used_chafa: false,
            scale: 1.0,
            true_color: false,
        };

        // Three 60ms ticks = 180ms: one frame advanced, 80ms carried over
        for _ in 0..3 {
            gif.update(0.06);
        }
        assert_eq!(gif.current_frame, 1);

        // A long stall skips ahead instead of advancing a single frame
        gif.update(0.25);
        assert_eq!(gif.current_frame, 1); // 80+250ms: advances through 2, 0, then 1

        // Invalid deltas are ignored rather than panicking
        gif.update(-1.0);
        gif.update(f32::NAN);
    }

    #[test]
    fn test_load_and_render_gif() {
        let gif_path = Path::new("samples/test.gif");
        if !gif_path.exists() {
            return; // Skip if test file not present
        }

        let gif = AnimatedGif::from_file(gif_path).expect("Failed to load GIF");
        assert!(gif.frame_count() > 0, "GIF should have frames");

        // Test rendering at different sizes
        for (w, h) in [(80, 24), (120, 40), (40, 20)] {
            let mut buffer = crate::renderer::AsciiBuffer::new(w, h);
            gif.render(&mut buffer);
            // Just verify no panic - buffer contents depend on image
        }
    }
}
