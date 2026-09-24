//! Sub-cell "pixel" output: render at a multiple of the cell resolution and
//! pack each cell's pixels into one block, sextant, octant or braille glyph
//! with a foreground and background color.
//!
//! Everything here is plain Unicode text, so it works in any terminal with a
//! suitable font (no Sixel/Kitty graphics needed).

use super::rasterizer::{AsciiBuffer, CharStyle, ColorPalette, Fragment};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

/// How a character cell is used to display rendered pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PixelMode {
    /// One pixel per cell, drawn with a `CharStyle` brightness ramp
    #[default]
    Cell,
    /// 1x2 pixels per cell (`▀▄`): square pixels, two colors per cell.
    /// Works in essentially every terminal.
    HalfBlock,
    /// 2x2 pixels per cell (`▘▞▟`...), two colors per cell. Widely supported.
    Quadrant,
    /// 2x3 pixels per cell (Unicode 13 sextants). Needs a recent font.
    Sextant,
    /// 2x4 pixels per cell (Unicode 16 octants). Needs a very recent font.
    Octant,
    /// 2x4 braille dots per cell, one color, shading by ordered dithering
    Braille,
}

impl PixelMode {
    pub fn next(self) -> Self {
        match self {
            Self::Cell => Self::HalfBlock,
            Self::HalfBlock => Self::Quadrant,
            Self::Quadrant => Self::Sextant,
            Self::Sextant => Self::Octant,
            Self::Octant => Self::Braille,
            Self::Braille => Self::Cell,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cell => "Cell",
            Self::HalfBlock => "HalfBlock",
            Self::Quadrant => "Quadrant",
            Self::Sextant => "Sextant",
            Self::Octant => "Octant",
            Self::Braille => "Braille",
        }
    }

    /// Pixels per cell as `(columns, rows)`
    pub fn subdivisions(self) -> (u16, u16) {
        match self {
            Self::Cell => (1, 1),
            Self::HalfBlock => (1, 2),
            Self::Quadrant => (2, 2),
            Self::Sextant => (2, 3),
            Self::Octant | Self::Braille => (2, 4),
        }
    }

    /// Height/width ratio of one pixel, given the cell's height/width ratio
    pub fn pixel_aspect(self, cell_aspect: f32) -> f32 {
        let (sx, sy) = self.subdivisions();
        cell_aspect * sx as f32 / sy as f32
    }
}

/// 4x4 ordered-dither (Bayer) threshold in `[0, 1)` for a pixel position
pub fn bayer4(x: u16, y: u16) -> f32 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (M[(y % 4) as usize][(x % 4) as usize] as f32 + 0.5) / 16.0
}

fn to_rgb(color: Color) -> Option<(f32, f32, f32)> {
    match color {
        Color::Rgb(r, g, b) => Some((r as f32, g as f32, b as f32)),
        _ => None,
    }
}

/// Average of several colors (RGB colors are averaged, others pass through)
fn average(colors: impl Iterator<Item = Color>) -> Option<Color> {
    let mut first = None;
    let (mut r, mut g, mut b, mut n) = (0.0, 0.0, 0.0, 0.0);
    for color in colors {
        first.get_or_insert(color);
        if let Some((cr, cg, cb)) = to_rgb(color) {
            r += cr;
            g += cg;
            b += cb;
            n += 1.0;
        }
    }
    if n > 0.0 {
        Some(Color::Rgb((r / n) as u8, (g / n) as u8, (b / n) as u8))
    } else {
        first
    }
}

/// Draw a pixel buffer (sized `area` x `mode.subdivisions()`) into `buf`.
///
/// Each cell picks the glyph whose filled sub-cells match its pixels, with the
/// brighter pixels as foreground and the darker ones as background. Empty
/// pixels leave the terminal background showing through.
pub(crate) fn draw_pixels(pixels: &AsciiBuffer, area: Rect, buf: &mut Buffer, mode: PixelMode, palette: ColorPalette) {
    let (sx, sy) = mode.subdivisions();
    let area = area.intersection(buf.area);
    let color_of = |f: &Fragment| f.color.unwrap_or_else(|| palette.to_color(f.luminance));

    for cy in 0..area.height.min(pixels.height / sy) {
        for cx in 0..area.width.min(pixels.width / sx) {
            // Row-major sub-pixels of this cell: bit i = column i % sx, row i / sx
            let mut cell: [(u16, u16, Option<Fragment>); 8] = [(0, 0, None); 8];
            let n = (sx * sy) as usize;
            for (i, slot) in cell.iter_mut().take(n).enumerate() {
                let (px, py) = (cx * sx + i as u16 % sx, cy * sy + i as u16 / sx);
                *slot = (px, py, pixels.get(px, py).copied());
            }
            let cell = &cell[..n];
            if cell.iter().all(|(_, _, f)| f.is_none()) {
                continue;
            }

            let target = &mut buf[(area.x + cx, area.y + cy)];
            if mode == PixelMode::Braille {
                // One color: show brightness through dot density
                let mut mask = 0usize;
                for (i, (px, py, f)) in cell.iter().enumerate() {
                    if f.is_some_and(|f| f.luminance > bayer4(*px, *py)) {
                        mask |= 1 << i;
                    }
                }
                if mask == 0 {
                    continue;
                }
                // Color from the lit dots only, so dim neighbours don't wash it out
                let lit = cell.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0);
                let fg = average(lit.filter_map(|(_, (_, _, f))| f.as_ref()).map(color_of));
                target.set_char(braille(mask));
                if let Some(fg) = fg {
                    target.set_fg(fg);
                }
                continue;
            }

            let any_empty = cell.iter().any(|(_, _, f)| f.is_none());
            let lums = cell.iter().filter_map(|(_, _, f)| f.map(|f| f.luminance));
            let (lo, hi) = lums.fold((f32::MAX, f32::MIN), |(lo, hi), l| (lo.min(l), hi.max(l)));

            // Foreground = filled pixels (if any are empty) or the brighter half
            let in_fg = |f: &Option<Fragment>| match f {
                None => false,
                Some(_) if any_empty || hi - lo < 0.05 => true,
                Some(f) => f.luminance >= (lo + hi) / 2.0,
            };
            let mask = cell.iter().enumerate().filter(|(_, (_, _, f))| in_fg(f)).fold(0, |m, (i, _)| m | 1 << i);
            let fg = average(cell.iter().filter(|(_, _, f)| in_fg(f)).filter_map(|(_, _, f)| f.as_ref()).map(color_of));
            let bg = average(cell.iter().filter(|(_, _, f)| !in_fg(f)).filter_map(|(_, _, f)| f.as_ref()).map(color_of));

            let glyph = match mode {
                PixelMode::HalfBlock => [' ', '▀', '▄', '█'][mask],
                PixelMode::Quadrant => QUADRANTS[mask],
                PixelMode::Sextant => SEXTANTS[mask],
                PixelMode::Octant => OCTANTS[mask],
                PixelMode::Cell | PixelMode::Braille => unreachable!(),
            };
            let mut style = Style::default();
            if let Some(fg) = fg {
                style = style.fg(fg);
            }
            if let Some(bg) = bg {
                style = style.bg(bg);
            }
            target.set_char(glyph).set_style(style);
        }
    }
}

/// Braille glyph for a row-major 2x4 bit mask
fn braille(mask: usize) -> char {
    // Braille dot bits, indexed by row-major position
    const DOTS: [u32; 8] = [0x01, 0x08, 0x02, 0x10, 0x04, 0x20, 0x40, 0x80];
    let bits = (0..8).filter(|i| mask & (1 << i) != 0).fold(0, |acc, i| acc | DOTS[i]);
    char::from_u32(0x2800 + bits).unwrap_or(' ')
}

/// Draw one pixel per cell using a character ramp, optionally dithered
pub(crate) fn draw_cells(pixels: &AsciiBuffer, area: Rect, buf: &mut Buffer, style: CharStyle, palette: ColorPalette, dither: bool) {
    let area = area.intersection(buf.area);
    for y in 0..area.height.min(pixels.height) {
        for x in 0..area.width.min(pixels.width) {
            if let Some(fragment) = pixels.get(x, y) {
                let ch = match fragment.glyph {
                    Some(glyph) => glyph,
                    None if dither => style.to_char_dithered(fragment.luminance, bayer4(x, y)),
                    None => style.to_char(fragment.luminance),
                };
                let color = fragment.color.unwrap_or_else(|| palette.to_color(fragment.luminance));
                buf[(area.x + x, area.y + y)].set_char(ch).set_style(Style::default().fg(color));
            }
        }
    }
}

// Glyph tables indexed by row-major sub-pixel bit pattern (from ratatui-core,
// MIT licensed)
const QUADRANTS: [char; 16] = [
    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
];
const SEXTANTS: [char; 64] = [
    ' ', '🬀', '🬁', '🬂', '🬃', '🬄', '🬅', '🬆', '🬇', '🬈', '🬉', '🬊', '🬋', '🬌', '🬍', '🬎', '🬏', '🬐', '🬑',
    '🬒', '🬓', '▌', '🬔', '🬕', '🬖', '🬗', '🬘', '🬙', '🬚', '🬛', '🬜', '🬝', '🬞', '🬟', '🬠', '🬡', '🬢', '🬣',
    '🬤', '🬥', '🬦', '🬧', '▐', '🬨', '🬩', '🬪', '🬫', '🬬', '🬭', '🬮', '🬯', '🬰', '🬱', '🬲', '🬳', '🬴', '🬵',
    '🬶', '🬷', '🬸', '🬹', '🬺', '🬻', '█',
];
const OCTANTS: [char; 256] = [
    ' ', '𜺨', '𜺫', '🮂', '𜴀', '▘', '𜴁', '𜴂', '𜴃', '𜴄', '▝', '𜴅', '𜴆', '𜴇', '𜴈', '▀', '𜴉', '𜴊', '𜴋',
    '𜴌', '🯦', '𜴍', '𜴎', '𜴏', '𜴐', '𜴑', '𜴒', '𜴓', '𜴔', '𜴕', '𜴖', '𜴗', '𜴘', '𜴙', '𜴚', '𜴛', '𜴜', '𜴝',
    '𜴞', '𜴟', '🯧', '𜴠', '𜴡', '𜴢', '𜴣', '𜴤', '𜴥', '𜴦', '𜴧', '𜴨', '𜴩', '𜴪', '𜴫', '𜴬', '𜴭', '𜴮', '𜴯',
    '𜴰', '𜴱', '𜴲', '𜴳', '𜴴', '𜴵', '🮅', '𜺣', '𜴶', '𜴷', '𜴸', '𜴹', '𜴺', '𜴻', '𜴼', '𜴽', '𜴾', '𜴿', '𜵀',
    '𜵁', '𜵂', '𜵃', '𜵄', '▖', '𜵅', '𜵆', '𜵇', '𜵈', '▌', '𜵉', '𜵊', '𜵋', '𜵌', '▞', '𜵍', '𜵎', '𜵏', '𜵐',
    '▛', '𜵑', '𜵒', '𜵓', '𜵔', '𜵕', '𜵖', '𜵗', '𜵘', '𜵙', '𜵚', '𜵛', '𜵜', '𜵝', '𜵞', '𜵟', '𜵠', '𜵡', '𜵢',
    '𜵣', '𜵤', '𜵥', '𜵦', '𜵧', '𜵨', '𜵩', '𜵪', '𜵫', '𜵬', '𜵭', '𜵮', '𜵯', '𜵰', '𜺠', '𜵱', '𜵲', '𜵳', '𜵴',
    '𜵵', '𜵶', '𜵷', '𜵸', '𜵹', '𜵺', '𜵻', '𜵼', '𜵽', '𜵾', '𜵿', '𜶀', '𜶁', '𜶂', '𜶃', '𜶄', '𜶅', '𜶆', '𜶇',
    '𜶈', '𜶉', '𜶊', '𜶋', '𜶌', '𜶍', '𜶎', '𜶏', '▗', '𜶐', '𜶑', '𜶒', '𜶓', '▚', '𜶔', '𜶕', '𜶖', '𜶗', '▐',
    '𜶘', '𜶙', '𜶚', '𜶛', '▜', '𜶜', '𜶝', '𜶞', '𜶟', '𜶠', '𜶡', '𜶢', '𜶣', '𜶤', '𜶥', '𜶦', '𜶧', '𜶨', '𜶩',
    '𜶪', '𜶫', '▂', '𜶬', '𜶭', '𜶮', '𜶯', '𜶰', '𜶱', '𜶲', '𜶳', '𜶴', '𜶵', '𜶶', '𜶷', '𜶸', '𜶹', '𜶺', '𜶻',
    '𜶼', '𜶽', '𜶾', '𜶿', '𜷀', '𜷁', '𜷂', '𜷃', '𜷄', '𜷅', '𜷆', '𜷇', '𜷈', '𜷉', '𜷊', '𜷋', '𜷌', '𜷍', '𜷎',
    '𜷏', '𜷐', '𜷑', '𜷒', '𜷓', '𜷔', '𜷕', '𜷖', '𜷗', '𜷘', '𜷙', '𜷚', '▄', '𜷛', '𜷜', '𜷝', '𜷞', '▙', '𜷟',
    '𜷠', '𜷡', '𜷢', '▟', '𜷣', '▆', '𜷤', '𜷥', '█',
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_bits_match_unicode() {
        assert_eq!(braille(0), '\u{2800}');
        assert_eq!(braille(0b1), '⠁'); // top-left = dot 1
        assert_eq!(braille(0b10), '⠈'); // top-right = dot 4
        assert_eq!(braille(0b0100_0000), '⡀'); // bottom-left = dot 7
        assert_eq!(braille(0xFF), '⣿');
    }

    #[test]
    fn glyph_tables_are_row_major() {
        assert_eq!(QUADRANTS[0b0011], '▀');
        assert_eq!(QUADRANTS[0b0101], '▌');
        assert_eq!(SEXTANTS[0b010101], '▌');
        assert_eq!(OCTANTS[0b1111_0000], '▄');
    }

    fn render(mode: PixelMode, fill: impl Fn(u16, u16) -> Option<f32>) -> Buffer {
        let (sx, sy) = mode.subdivisions();
        let mut pixels = AsciiBuffer::new(sx, sy);
        for y in 0..sy {
            for x in 0..sx {
                if let Some(l) = fill(x, y) {
                    pixels.plot(x, y, 1.0, l);
                }
            }
        }
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        draw_pixels(&pixels, area, &mut buf, mode, ColorPalette::Grayscale);
        buf
    }

    #[test]
    fn half_block_uses_both_colors() {
        let buf = render(PixelMode::HalfBlock, |_, y| Some(if y == 0 { 1.0 } else { 0.2 }));
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "▀");
        assert_eq!(cell.fg, ColorPalette::Grayscale.to_color(1.0));
        assert_eq!(cell.bg, ColorPalette::Grayscale.to_color(0.2));
    }

    #[test]
    fn empty_pixels_keep_terminal_background() {
        let buf = render(PixelMode::Quadrant, |x, _| (x == 0).then_some(0.8));
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), "▌");
        assert_eq!(cell.bg, Color::Reset);
    }
}
