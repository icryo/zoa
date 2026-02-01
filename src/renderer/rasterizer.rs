use super::math::{Camera, Vec3};
use ratatui::style::Color;

/// Render mode: solid fill or wireframe edges
#[derive(Clone, Copy, PartialEq, Default)]
pub enum RenderMode {
    #[default]
    Solid,
    Wireframe,
}

impl RenderMode {
    pub fn next(self) -> Self {
        match self {
            Self::Solid => Self::Wireframe,
            Self::Wireframe => Self::Solid,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Wireframe => "Wire",
        }
    }
}

/// Character set styles for ASCII rendering (similar to chafa)
#[derive(Clone, Copy, PartialEq, Default)]
pub enum CharStyle {
    #[default]
    Ascii,      // Standard ASCII gradient
    Blocks,     // Unicode block characters
    Braille,    // Braille dot patterns
    Dense,      // Extended ASCII for finer gradients
    Minimal,    // Simple few-character set
    Hatching,   // Line-based hatching
    Dots,       // Dot/circle patterns
    Stars,      // Star/sparkle patterns
}

impl CharStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Ascii => Self::Blocks,
            Self::Blocks => Self::Braille,
            Self::Braille => Self::Dense,
            Self::Dense => Self::Minimal,
            Self::Minimal => Self::Hatching,
            Self::Hatching => Self::Dots,
            Self::Dots => Self::Stars,
            Self::Stars => Self::Ascii,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ascii => "ASCII",
            Self::Blocks => "Blocks",
            Self::Braille => "Braille",
            Self::Dense => "Dense",
            Self::Minimal => "Minimal",
            Self::Hatching => "Hatching",
            Self::Dots => "Dots",
            Self::Stars => "Stars",
        }
    }

    pub fn chars(self) -> &'static [char] {
        match self {
            Self::Ascii => &[' ', '.', ',', '-', '~', ':', ';', '=', '!', '*', '#', '$', '@'],
            Self::Blocks => &[' ', '░', '░', '▒', '▒', '▓', '▓', '█', '█'],
            Self::Braille => &[' ', '⠁', '⠃', '⠇', '⠏', '⠟', '⠿', '⡿', '⣿'],
            Self::Dense => &[' ', '`', '.', '-', '\'', ':', '_', ',', '^', '=', ';', '>', '<',
                            '+', '!', 'r', 'c', '*', '/', 'z', '?', 's', 'L', 'T', 'v',
                            ')', 'J', '7', '(', '|', 'F', 'i', '{', 'C', '}', 'f', 'I',
                            '3', '1', 't', 'l', 'u', '[', 'n', 'e', 'o', 'Z', '5', 'Y',
                            'x', 'j', 'y', 'a', ']', '2', 'E', 'S', 'w', 'q', 'k', 'P',
                            '6', 'h', '9', 'd', '4', 'V', 'p', 'O', 'G', 'b', 'U', 'A',
                            'K', 'X', 'H', 'm', '8', 'R', 'D', '#', '$', 'B', 'g', '0',
                            'M', 'N', 'W', 'Q', '%', '&', '@'],
            Self::Minimal => &[' ', '.', ':', '+', '*', '#', '@'],
            Self::Hatching => &[' ', '·', '-', '/', '|', '\\', '+', 'x', 'X', '#', '▓', '█'],
            Self::Dots => &[' ', '·', '∙', '•', '●', '◉', '◎', '○', '◌', '◍', '◉', '●', '⬤'],
            Self::Stars => &[' ', '·', '˙', '*', '✦', '✧', '★', '☆', '✪', '✫', '✬', '✯', '✵'],
        }
    }

    pub fn to_char(self, luminance: f32) -> char {
        let chars = self.chars();
        let idx = ((luminance.clamp(0.0, 1.0)) * (chars.len() - 1) as f32) as usize;
        chars[idx.min(chars.len() - 1)]
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ColorPalette {
    #[default]
    Cyan,
    Fire,
    Matrix,
    Purple,
    Grayscale,
    Rainbow,
}

impl ColorPalette {
    pub fn next(self) -> Self {
        match self {
            Self::Cyan => Self::Fire,
            Self::Fire => Self::Matrix,
            Self::Matrix => Self::Purple,
            Self::Purple => Self::Grayscale,
            Self::Grayscale => Self::Rainbow,
            Self::Rainbow => Self::Cyan,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cyan => "Cyan",
            Self::Fire => "Fire",
            Self::Matrix => "Matrix",
            Self::Purple => "Purple",
            Self::Grayscale => "Grayscale",
            Self::Rainbow => "Rainbow",
        }
    }

    pub fn to_color(self, luminance: f32) -> Color {
        let l = luminance.clamp(0.0, 1.0);
        match self {
            Self::Cyan => {
                let r = (l * l * 255.0) as u8;
                let g = (l * 200.0 + 55.0 * l * l) as u8;
                let b = (100.0 + l * 155.0) as u8;
                Color::Rgb(r, g, b)
            }
            Self::Fire => {
                let r = (50.0 + l * 205.0) as u8;
                let g = (l * l * 180.0) as u8;
                let b = (l * l * l * 80.0) as u8;
                Color::Rgb(r, g, b)
            }
            Self::Matrix => {
                let r = (l * l * 50.0) as u8;
                let g = (40.0 + l * 215.0) as u8;
                let b = (l * l * 80.0) as u8;
                Color::Rgb(r, g, b)
            }
            Self::Purple => {
                let r = (80.0 + l * 175.0) as u8;
                let g = (l * l * 100.0) as u8;
                let b = (100.0 + l * 155.0) as u8;
                Color::Rgb(r, g, b)
            }
            Self::Grayscale => {
                let v = (l * 255.0) as u8;
                Color::Rgb(v, v, v)
            }
            Self::Rainbow => {
                // HSV to RGB with hue based on luminance
                let h = l * 300.0; // 0-300 degrees (red to magenta)
                let s = 0.8;
                let v = 0.5 + l * 0.5;
                let (r, g, b) = hsv_to_rgb(h, s, v);
                Color::Rgb(r, g, b)
            }
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

/// Fragment stores per-pixel rendering data
#[derive(Clone, Copy)]
pub struct Fragment {
    pub luminance: f32,
}

/// Buffer for storing rendered ASCII data with z-buffering
pub struct AsciiBuffer {
    pub width: u16,
    pub height: u16,
    buffer: Vec<Option<Fragment>>,
    z_buffer: Vec<f32>,
}

impl AsciiBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            buffer: vec![None; size],
            z_buffer: vec![f32::NEG_INFINITY; size],
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(None);
        self.z_buffer.fill(f32::NEG_INFINITY);
    }

    pub fn plot(&mut self, x: u16, y: u16, depth: f32, luminance: f32) {
        if x >= self.width || y >= self.height {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + (x as usize);

        if depth > self.z_buffer[idx] {
            self.z_buffer[idx] = depth;
            self.buffer[idx] = Some(Fragment { luminance });
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Fragment> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.buffer[idx].as_ref()
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            let size = (width as usize) * (height as usize);
            self.buffer = vec![None; size];
            self.z_buffer = vec![f32::NEG_INFINITY; size];
        }
    }
}

pub struct Renderer {
    pub camera: Camera,
    pub light_dir: Vec3,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            light_dir: Vec3::new(0.0, 1.0, -1.0).normalize(),
        }
    }
}

impl Renderer {
    pub fn render_point(
        &self,
        buffer: &mut AsciiBuffer,
        position: Vec3,
        normal: Vec3,
    ) {
        if let Some((sx, sy, depth)) = self.camera.project(position, buffer.width, buffer.height) {
            let luminance = normal.dot(self.light_dir).max(0.0);
            buffer.plot(sx, sy, depth, luminance);
        }
    }
}
