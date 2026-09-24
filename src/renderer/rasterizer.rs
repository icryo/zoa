use super::math::{Camera, Vec3, CHAR_ASPECT};
use super::pixels::{self, PixelMode};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
};

/// Render mode: solid fill or wireframe edges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

    /// Like `to_char`, but rounds between neighbouring ramp characters using
    /// `threshold` in `[0, 1)` (e.g. an ordered-dither matrix value), so
    /// smooth gradients don't collapse into flat bands.
    pub fn to_char_dithered(self, luminance: f32, threshold: f32) -> char {
        let chars = self.chars();
        let level = luminance.clamp(0.0, 1.0) * (chars.len() - 1) as f32;
        let idx = (level + threshold - 0.5).round().max(0.0) as usize;
        chars[idx.min(chars.len() - 1)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fragment {
    pub luminance: f32,
    /// Exact character to show instead of one picked from the `CharStyle`
    /// (e.g. text labels)
    pub glyph: Option<char>,
    /// Exact color to use instead of one picked from the `ColorPalette`
    /// (e.g. true-color GIF pixels)
    pub color: Option<Color>,
}

impl Fragment {
    pub fn new(luminance: f32) -> Self {
        Self {
            luminance,
            glyph: None,
            color: None,
        }
    }

    pub fn with_glyph(mut self, glyph: char) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The character and color to display for this fragment.
    pub fn resolve(&self, style: CharStyle, palette: ColorPalette) -> (char, Color) {
        (
            self.glyph.unwrap_or_else(|| style.to_char(self.luminance)),
            self.color.unwrap_or_else(|| palette.to_color(self.luminance)),
        )
    }
}

/// Buffer for storing rendered ASCII data with z-buffering
pub struct AsciiBuffer {
    pub width: u16,
    pub height: u16,
    /// Height/width ratio of one buffer pixel (2.0 when a pixel is a whole
    /// terminal cell; see `PixelMode::pixel_aspect`)
    pub pixel_aspect: f32,
    buffer: Vec<Option<Fragment>>,
    z_buffer: Vec<f32>,
}

impl AsciiBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            pixel_aspect: CHAR_ASPECT,
            buffer: vec![None; size],
            z_buffer: vec![f32::NEG_INFINITY; size],
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(None);
        self.z_buffer.fill(f32::NEG_INFINITY);
    }

    /// Plot a fragment if it is closer than what is already there
    /// (larger `depth` = closer).
    pub fn plot(&mut self, x: u16, y: u16, depth: f32, luminance: f32) {
        self.plot_fragment(x, y, depth, Fragment::new(luminance));
    }

    pub fn plot_fragment(&mut self, x: u16, y: u16, depth: f32, fragment: Fragment) {
        if x >= self.width || y >= self.height {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + (x as usize);

        if depth > self.z_buffer[idx] {
            self.z_buffer[idx] = depth;
            self.buffer[idx] = Some(fragment);
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

    /// Draw the buffer (one pixel per cell) into a ratatui `Buffer` at
    /// `area`, leaving empty cells untouched.
    pub fn draw(&self, area: Rect, buf: &mut Buffer, style: CharStyle, palette: ColorPalette) {
        pixels::draw_cells(self, area, buf, style, palette, false);
    }

    /// Draw a buffer rendered at `mode.subdivisions()` pixels per cell.
    /// In `PixelMode::Cell`, `style` picks the characters, optionally with
    /// ordered dithering; other modes pick glyphs from the pixel pattern.
    pub fn draw_pixels(
        &self,
        area: Rect,
        buf: &mut Buffer,
        mode: PixelMode,
        style: CharStyle,
        palette: ColorPalette,
        dither: bool,
    ) {
        match mode {
            PixelMode::Cell => pixels::draw_cells(self, area, buf, style, palette, dither),
            _ => pixels::draw_pixels(self, area, buf, mode, palette),
        }
    }
}

pub struct Renderer {
    pub camera: Camera,
    pub light_dir: Vec3,
    /// Minimum brightness of surfaces facing away from the light, so they
    /// still show up instead of vanishing into blank cells
    pub ambient: f32,
    /// Strength of the specular (shiny) highlight, 0 to disable
    pub specular: f32,
    /// Tightness of the specular highlight; higher = smaller, sharper
    pub shininess: f32,
    /// How much surfaces darken toward the back of the scene, 0 to disable
    pub fog: f32,
    pub mode: RenderMode,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            light_dir: Vec3::new(0.0, 1.0, -1.0).normalize(),
            ambient: 0.15,
            specular: 0.4,
            shininess: 32.0,
            fog: 0.35,
            mode: RenderMode::default(),
        }
    }
}

/// Tolerance on barycentric weights so neighbouring triangles leave no seams
const EDGE_EPSILON: f32 = 1e-4;

impl Renderer {
    /// Brightness of a surface with unit `normal` at depth `inv_z` (as
    /// returned by the camera): ambient + diffuse + specular, dimmed by fog.
    pub fn shade(&self, normal: Vec3, inv_z: f32) -> f32 {
        let diffuse = normal.dot(self.light_dir).max(0.0);
        // Leave headroom so the highlight doesn't blow out whole lit areas
        let diffuse_weight = (1.0 - self.ambient) * (1.0 - 0.5 * self.specular);
        let mut lum = self.ambient + diffuse_weight * diffuse;

        if self.specular > 0.0 && diffuse > 0.0 {
            // Blinn-Phong with the viewer looking down +z
            let half = (self.light_dir + Vec3::new(0.0, 0.0, -1.0)).normalize();
            lum += self.specular * normal.dot(half).max(0.0).powf(self.shininess);
        }

        if self.fog > 0.0 && inv_z > 0.0 {
            // 0 at the front of a radius-3 scene around the origin, 1 at the back
            let t = ((1.0 / inv_z - self.camera.distance) / 6.0 + 0.5).clamp(0.0, 1.0);
            lum *= 1.0 - self.fog * t;
        }
        lum.min(1.0)
    }

    pub fn render_point(&self, buffer: &mut AsciiBuffer, position: Vec3, normal: Vec3) {
        if let Some((sx, sy, depth)) = self.camera.project(position, buffer.width, buffer.height, buffer.pixel_aspect) {
            buffer.plot(sx, sy, depth, self.shade(normal, depth));
        }
    }

    /// Rasterize a filled triangle (already in view space), interpolating
    /// the vertex normals per cell for smooth shading.
    ///
    /// Every cell whose center falls inside the projected triangle is drawn,
    /// so adjoining triangles cover a surface with no gaps at any size.
    pub fn draw_triangle(&self, buffer: &mut AsciiBuffer, positions: [Vec3; 3], normals: [Vec3; 3]) {
        let (w, h) = (buffer.width, buffer.height);
        if w == 0 || h == 0 {
            return;
        }
        let aspect = buffer.pixel_aspect;
        let project = |p| self.camera.project_f(p, w, h, aspect);
        let (Some(a), Some(b), Some(c)) = (project(positions[0]), project(positions[1]), project(positions[2])) else {
            return;
        };

        let edge = |p: (f32, f32, f32), q: (f32, f32, f32), x: f32, y: f32| {
            (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0)
        };
        let area = edge(a, b, c.0, c.1);
        if area.abs() < 1e-12 {
            return;
        }
        let inv_area = 1.0 / area;

        // Range of cells whose centers lie inside the bounding box
        let min_x = (a.0.min(b.0).min(c.0) - 0.5).ceil().max(0.0);
        let max_x = (a.0.max(b.0).max(c.0) - 0.5).floor().min(w as f32 - 1.0);
        let min_y = (a.1.min(b.1).min(c.1) - 0.5).ceil().max(0.0);
        let max_y = (a.1.max(b.1).max(c.1) - 0.5).floor().min(h as f32 - 1.0);

        if min_x > max_x || min_y > max_y {
            // Smaller than a pixel in both directions: draw it as a point so
            // fine detail (e.g. dense meshes) doesn't vanish. Thin slivers
            // (edge-on triangles) are skipped; their neighbours cover them.
            let span_x = a.0.max(b.0).max(c.0) - a.0.min(b.0).min(c.0);
            let span_y = a.1.max(b.1).max(c.1) - a.1.min(b.1).min(c.1);
            if span_x > 1.0 || span_y > 1.0 {
                return;
            }
            let centroid = (positions[0] + positions[1] + positions[2]) * (1.0 / 3.0);
            let normal = (normals[0] + normals[1] + normals[2]).normalize();
            self.render_point(buffer, centroid, normal);
            return;
        }

        for y in min_y as u16..=max_y as u16 {
            let py = y as f32 + 0.5;
            for x in min_x as u16..=max_x as u16 {
                let px = x as f32 + 0.5;
                let w0 = edge(b, c, px, py) * inv_area;
                let w1 = edge(c, a, px, py) * inv_area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -EDGE_EPSILON || w1 < -EDGE_EPSILON || w2 < -EDGE_EPSILON {
                    continue;
                }
                // 1/z is linear in screen space, so this depth is exact
                let depth = a.2 * w0 + b.2 * w1 + c.2 * w2;
                let normal = (normals[0] * w0 + normals[1] * w1 + normals[2] * w2).normalize();
                buffer.plot(x, y, depth, self.shade(normal, depth));
            }
        }
    }

    /// Draw a continuous line between two view-space points.
    pub fn draw_line(&self, buffer: &mut AsciiBuffer, from: Vec3, to: Vec3, from_normal: Vec3, to_normal: Vec3) {
        let (w, h) = (buffer.width, buffer.height);
        if w == 0 || h == 0 {
            return;
        }
        let aspect = buffer.pixel_aspect;
        let (Some(a), Some(b)) = (self.camera.project_f(from, w, h, aspect), self.camera.project_f(to, w, h, aspect)) else {
            return;
        };
        // One step per cell along the longer axis, capped for off-screen lines
        let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().clamp(1.0, 4096.0) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = a.0 + (b.0 - a.0) * t;
            let y = a.1 + (b.1 - a.1) * t;
            if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
                continue;
            }
            let depth = a.2 + (b.2 - a.2) * t;
            let normal = (from_normal * (1.0 - t) + to_normal * t).normalize();
            buffer.plot(x as u16, y as u16, depth, self.shade(normal, depth));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(buffer: &AsciiBuffer) -> usize {
        (0..buffer.height)
            .flat_map(|y| (0..buffer.width).map(move |x| (x, y)))
            .filter(|&(x, y)| buffer.get(x, y).is_some())
            .count()
    }

    #[test]
    fn adjacent_triangles_leave_no_gaps() {
        let renderer = Renderer::default();
        let mut buffer = AsciiBuffer::new(80, 40);
        let n = Vec3::new(0.0, 0.0, -1.0);
        let (p0, p1, p2, p3) = (
            Vec3::new(-1.5, -1.5, 0.0),
            Vec3::new(1.5, -1.5, 0.0),
            Vec3::new(1.5, 1.5, 0.0),
            Vec3::new(-1.5, 1.5, 0.0),
        );
        renderer.draw_triangle(&mut buffer, [p0, p1, p2], [n; 3]);
        renderer.draw_triangle(&mut buffer, [p0, p2, p3], [n; 3]);

        // The square must be filled solid between its projected corners
        let (x0, y0, _) = renderer.camera.project_f(p3, 80, 40, CHAR_ASPECT).unwrap();
        let (x1, y1, _) = renderer.camera.project_f(p1, 80, 40, CHAR_ASPECT).unwrap();
        for y in (y0.ceil() as u16)..(y1.floor() as u16) {
            for x in (x0.ceil() as u16)..(x1.floor() as u16) {
                assert!(buffer.get(x, y).is_some(), "gap at ({x}, {y})");
            }
        }
    }

    #[test]
    fn nearer_triangle_wins() {
        // Plain diffuse lighting, so luminance tells the two triangles apart
        let renderer = Renderer { specular: 0.0, fog: 0.0, ..Renderer::default() };
        let mut buffer = AsciiBuffer::new(40, 20);
        let tri = |z: f32| [Vec3::new(-2.0, -2.0, z), Vec3::new(2.0, -2.0, z), Vec3::new(0.0, 2.0, z)];
        let lit = Vec3::new(0.0, 1.0, -1.0).normalize();
        let dark = Vec3::new(0.0, -1.0, 0.0);
        renderer.draw_triangle(&mut buffer, tri(-1.0), [lit; 3]);
        renderer.draw_triangle(&mut buffer, tri(1.0), [dark; 3]);
        assert!(buffer.get(20, 10).unwrap().luminance > 0.9);
    }

    #[test]
    fn highlight_and_fog() {
        let renderer = Renderer::default();
        let facing_light = renderer.light_dir;
        let half = (renderer.light_dir + Vec3::new(0.0, 0.0, -1.0)).normalize();
        let depth = 1.0 / renderer.camera.distance;
        // The specular peak is brighter than plain diffuse lighting
        assert!(renderer.shade(half, depth) > renderer.shade(facing_light, depth));
        // The same surface is dimmer further back
        let near = 1.0 / (renderer.camera.distance - 2.0);
        let far = 1.0 / (renderer.camera.distance + 2.0);
        assert!(renderer.shade(facing_light, near) > renderer.shade(facing_light, far));
    }

    #[test]
    fn dithering_mixes_neighbouring_levels() {
        let style = CharStyle::Minimal; // 7 characters
        let level = 2.5 / 6.0; // halfway between chars[2] and chars[3]
        let chars: std::collections::HashSet<char> =
            (0..16).map(|i| style.to_char_dithered(level, (i as f32 + 0.5) / 16.0)).collect();
        assert_eq!(chars, [style.chars()[2], style.chars()[3]].into_iter().collect());
    }

    #[test]
    fn lines_are_continuous() {
        let renderer = Renderer::default();
        let mut buffer = AsciiBuffer::new(80, 40);
        let n = Vec3::new(0.0, 0.0, -1.0);
        renderer.draw_line(&mut buffer, Vec3::new(-3.0, 0.0, 0.0), Vec3::new(3.0, 0.0, 0.0), n, n);
        let row = (0..40).max_by_key(|&y| (0..80).filter(|&x| buffer.get(x, y).is_some()).count()).unwrap();
        let xs: Vec<u16> = (0..80).filter(|&x| buffer.get(x, row).is_some()).collect();
        assert!(xs.len() > 20);
        assert_eq!(xs.len() as u16, xs.last().unwrap() - xs[0] + 1);
    }

    #[test]
    fn fragment_overrides_style_and_palette() {
        let f = Fragment::new(0.5).with_glyph('A').with_color(Color::Rgb(1, 2, 3));
        assert_eq!(f.resolve(CharStyle::Blocks, ColorPalette::Fire), ('A', Color::Rgb(1, 2, 3)));
        let mut buffer = AsciiBuffer::new(2, 2);
        buffer.plot(0, 0, 1.0, 1.0);
        assert_eq!(filled(&buffer), 1);
    }
}
