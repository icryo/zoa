//! Countdown timer with large ASCII digits
//!
//! Displays a countdown timer using seven-segment style ASCII art digits,
//! inspired by https://github.com/antonmedv/countdown

use crate::renderer::AsciiBuffer;
use std::time::{Duration, Instant};

/// Large ASCII digit font (7 lines tall)
/// Uses Unicode box-drawing characters for a clean look
const DIGIT_HEIGHT: usize = 7;

fn digit_pattern(c: char) -> &'static [&'static str] {
    match c {
        '0' => &[
            "  █████╗ ",
            " ██╔══██╗",
            " ██║  ██║",
            " ██║  ██║",
            " ██║  ██║",
            " ╚█████╔╝",
            "         ",
        ],
        '1' => &[
            "    ██╗  ",
            "   ███║  ",
            "    ██║  ",
            "    ██║  ",
            "    ██║  ",
            "    ██║  ",
            "         ",
        ],
        '2' => &[
            "  █████╗ ",
            " ╚════██╗",
            "  █████╔╝",
            " ██╔════╝",
            " ███████╗",
            " ╚══════╝",
            "         ",
        ],
        '3' => &[
            "  █████╗ ",
            " ╚════██╗",
            "   ████╔╝",
            "   ╚══██╗",
            " █████╔╝ ",
            " ╚════╝  ",
            "         ",
        ],
        '4' => &[
            " ██╗ ██╗ ",
            " ██║ ██║ ",
            " ███████╗",
            " ╚════██║",
            "      ██║",
            "      ╚═╝",
            "         ",
        ],
        '5' => &[
            " ███████╗",
            " ██╔════╝",
            " ██████╗ ",
            " ╚════██╗",
            " ██████╔╝",
            " ╚═════╝ ",
            "         ",
        ],
        '6' => &[
            "  █████╗ ",
            " ██╔════╝",
            " ██████╗ ",
            " ██╔══██╗",
            " ╚█████╔╝",
            "  ╚════╝ ",
            "         ",
        ],
        '7' => &[
            " ███████╗",
            " ╚════██║",
            "     ██╔╝",
            "    ██╔╝ ",
            "   ██╔╝  ",
            "   ╚═╝   ",
            "         ",
        ],
        '8' => &[
            "  █████╗ ",
            " ██╔══██╗",
            "  █████╔╝",
            " ██╔══██╗",
            " ╚█████╔╝",
            "  ╚════╝ ",
            "         ",
        ],
        '9' => &[
            "  █████╗ ",
            " ██╔══██╗",
            " ╚██████║",
            "  ╚═══██║",
            "  █████╔╝",
            "  ╚════╝ ",
            "         ",
        ],
        ':' => &[
            "   ",
            "██╗",
            "╚═╝",
            "██╗",
            "╚═╝",
            "   ",
            "   ",
        ],
        ' ' => &[
            "   ",
            "   ",
            "   ",
            "   ",
            "   ",
            "   ",
            "   ",
        ],
        _ => &[
            "   ",
            "   ",
            "   ",
            "   ",
            "   ",
            "   ",
            "   ",
        ],
    }
}

/// Get the display width of a digit pattern
fn pattern_width(pattern: &[&str]) -> usize {
    pattern.iter().map(|s| s.chars().count()).max().unwrap_or(0)
}

/// Countdown timer state
#[derive(Clone, Copy, PartialEq)]
pub enum CountdownState {
    Running,
    Paused,
    Finished,
}

/// A countdown timer that renders as large ASCII digits
pub struct Countdown {
    /// Total duration to count down from
    total: Duration,
    /// Time remaining
    remaining: Duration,
    /// When the timer was last updated (for pause/resume)
    last_update: Option<Instant>,
    /// Current state
    state: CountdownState,
    /// Optional title to display below timer
    pub title: Option<String>,
    /// Blink colon (for visual effect)
    blink: bool,
    blink_elapsed: Duration,
    /// Zoom/scale factor (1.0 = fit to buffer, >1 = larger, <1 = smaller)
    scale: f32,
}

impl Countdown {
    /// Create a new countdown timer
    pub fn new(duration: Duration) -> Self {
        Self {
            total: duration,
            remaining: duration,
            last_update: None,
            state: CountdownState::Running,
            title: None,
            blink: true,
            blink_elapsed: Duration::ZERO,
            scale: 1.0,
        }
    }

    /// Set zoom/scale factor (0.3 to 3.0)
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale.clamp(0.3, 3.0);
    }

    /// Get current scale factor
    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    /// Create from hours, minutes, seconds
    pub fn from_hms(hours: u64, minutes: u64, seconds: u64) -> Self {
        let duration = Duration::from_secs(hours * 3600 + minutes * 60 + seconds);
        Self::new(duration)
    }

    /// Create from minutes and seconds
    pub fn from_ms(minutes: u64, seconds: u64) -> Self {
        Self::from_hms(0, minutes, seconds)
    }

    /// Create from seconds only
    pub fn from_secs(seconds: u64) -> Self {
        Self::new(Duration::from_secs(seconds))
    }

    /// Parse duration string like "1h2m3s", "5m", "30s", "1:30", "1:30:00"
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();

        // Try parsing as colon-separated (MM:SS or HH:MM:SS)
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            match parts.len() {
                2 => {
                    let mins: u64 = parts[0].parse().map_err(|_| "Invalid minutes")?;
                    let secs: u64 = parts[1].parse().map_err(|_| "Invalid seconds")?;
                    return Ok(Self::from_ms(mins, secs));
                }
                3 => {
                    let hrs: u64 = parts[0].parse().map_err(|_| "Invalid hours")?;
                    let mins: u64 = parts[1].parse().map_err(|_| "Invalid minutes")?;
                    let secs: u64 = parts[2].parse().map_err(|_| "Invalid seconds")?;
                    return Ok(Self::from_hms(hrs, mins, secs));
                }
                _ => return Err("Invalid time format".to_string()),
            }
        }

        // Try parsing as duration string (1h2m3s style)
        let mut total_secs: u64 = 0;
        let mut current_num = String::new();

        for c in s.chars() {
            if c.is_ascii_digit() {
                current_num.push(c);
            } else {
                let num: u64 = current_num.parse().unwrap_or(0);
                current_num.clear();
                match c {
                    'h' | 'H' => total_secs += num * 3600,
                    'm' | 'M' => total_secs += num * 60,
                    's' | 'S' => total_secs += num,
                    _ => {}
                }
            }
        }

        // If just a number, treat as seconds
        if !current_num.is_empty() && total_secs == 0 {
            total_secs = current_num.parse().unwrap_or(0);
        }

        if total_secs == 0 {
            return Err("Invalid duration".to_string());
        }

        Ok(Self::from_secs(total_secs))
    }

    /// Set a title to display below the timer
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Start or resume the timer
    pub fn start(&mut self) {
        if self.state != CountdownState::Finished {
            self.state = CountdownState::Running;
            self.last_update = Some(Instant::now());
        }
    }

    /// Pause the timer
    pub fn pause(&mut self) {
        if self.state == CountdownState::Running {
            self.state = CountdownState::Paused;
        }
    }

    /// Toggle pause/resume
    pub fn toggle_pause(&mut self) {
        match self.state {
            CountdownState::Running => self.pause(),
            CountdownState::Paused => self.start(),
            CountdownState::Finished => {}
        }
    }

    /// Reset to initial duration
    pub fn reset(&mut self) {
        self.remaining = self.total;
        self.state = CountdownState::Running;
        self.last_update = Some(Instant::now());
    }

    /// Get current state
    pub fn state(&self) -> CountdownState {
        self.state
    }

    /// Check if finished
    pub fn is_finished(&self) -> bool {
        self.state == CountdownState::Finished
    }

    /// Get remaining time
    pub fn remaining(&self) -> Duration {
        self.remaining
    }

    /// Update the timer (call each frame)
    pub fn update(&mut self, dt: f32) {
        // Update blink state
        self.blink_elapsed += Duration::from_secs_f32(dt);
        if self.blink_elapsed >= Duration::from_millis(500) {
            self.blink_elapsed = Duration::ZERO;
            self.blink = !self.blink;
        }

        if self.state != CountdownState::Running {
            return;
        }

        let elapsed = Duration::from_secs_f32(dt);
        if self.remaining > elapsed {
            self.remaining -= elapsed;
        } else {
            self.remaining = Duration::ZERO;
            self.state = CountdownState::Finished;
        }
    }

    /// Format remaining time as string
    fn format_time(&self) -> String {
        let total_secs = self.remaining.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("{:02}:{:02}", mins, secs)
        }
    }

    /// Render the countdown to an ASCII buffer
    pub fn render(&self, buffer: &mut AsciiBuffer) {
        let buf_width = buffer.width as usize;
        let buf_height = buffer.height as usize;

        // Guard against empty or tiny buffers
        if buf_width < 3 || buf_height < 1 {
            return;
        }

        let time_str = self.format_time();

        // Calculate total width needed
        let mut total_width = 0;
        let mut patterns: Vec<&[&str]> = Vec::new();

        for c in time_str.chars() {
            // Handle colon blinking when paused
            let pattern = if c == ':' && self.state == CountdownState::Paused && !self.blink {
                digit_pattern(' ')
            } else {
                digit_pattern(c)
            };
            patterns.push(pattern);
            total_width += pattern_width(pattern);
        }

        if total_width == 0 {
            return;
        }

        // Calculate base scaling to fit buffer
        let scale_x = total_width as f32 / buf_width as f32;
        let scale_y = DIGIT_HEIGHT as f32 / buf_height as f32;
        // Apply user zoom: higher self.scale = larger output (divide base scale)
        let scale = (scale_x.max(scale_y) / self.scale).max(0.1);

        let scaled_height = ((DIGIT_HEIGHT as f32 / scale) as usize).min(buf_height).max(1);
        let scaled_width = ((total_width as f32 / scale) as usize).min(buf_width).max(1);

        // Center vertically (leave room for title)
        let title_lines = if self.title.is_some() { 2 } else { 0 };
        let offset_y = buf_height.saturating_sub(scaled_height + title_lines) / 2;
        let offset_x = buf_width.saturating_sub(scaled_width) / 2;

        // Build a flat representation of the time display for sampling
        let mut source_chars: Vec<Vec<char>> = vec![Vec::new(); DIGIT_HEIGHT];
        for pattern in &patterns {
            for (row, line) in pattern.iter().enumerate() {
                for ch in line.chars() {
                    source_chars[row].push(ch);
                }
            }
        }

        // Iterate over output space and sample from source
        for out_y in 0..scaled_height {
            let buf_y = offset_y + out_y;
            if buf_y >= buf_height {
                break;
            }

            // Map output y back to source row
            let src_row = ((out_y as f32 * scale) as usize).min(DIGIT_HEIGHT - 1);

            for out_x in 0..scaled_width {
                let buf_x = offset_x + out_x;
                if buf_x >= buf_width {
                    break;
                }

                // Map output x back to source column
                let src_col = ((out_x as f32 * scale) as usize).min(total_width.saturating_sub(1));

                if src_col < source_chars[src_row].len() {
                    let ch = source_chars[src_row][src_col];

                    // Map character to luminance
                    let lum = match ch {
                        '█' => 1.0,
                        '╔' | '╗' | '╚' | '╝' | '║' | '═' => 0.7,
                        _ => 0.0,
                    };

                    if lum > 0.0 {
                        buffer.plot(buf_x as u16, buf_y as u16, 100.0, lum);
                    }
                }
            }
        }

        // Render title if present
        if let Some(ref title) = self.title {
            let title_y = offset_y + scaled_height + 1;
            if title_y < buf_height {
                let title_x = buf_width.saturating_sub(title.len()) / 2;
                for (i, ch) in title.chars().enumerate() {
                    let x = title_x + i;
                    if x < buf_width {
                        // Plot title characters with medium luminance
                        // We use a simple approach - just set luminance based on char
                        if !ch.is_whitespace() {
                            buffer.plot(x as u16, title_y as u16, 100.0, 0.5);
                        }
                    }
                }
            }
        }

        // Show "FINISHED" or "PAUSED" status
        let status = match self.state {
            CountdownState::Finished => Some("FINISHED"),
            CountdownState::Paused => Some("PAUSED"),
            CountdownState::Running => None,
        };

        if let Some(status_text) = status {
            let status_y = offset_y + scaled_height + if self.title.is_some() { 3 } else { 1 };
            if status_y < buf_height {
                let status_x = buf_width.saturating_sub(status_text.len()) / 2;
                for (i, _) in status_text.chars().enumerate() {
                    let x = status_x + i;
                    if x < buf_width {
                        buffer.plot(x as u16, status_y as u16, 100.0, 0.4);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        let c = Countdown::parse("5m").unwrap();
        assert_eq!(c.remaining().as_secs(), 300);

        let c = Countdown::parse("1h30m").unwrap();
        assert_eq!(c.remaining().as_secs(), 5400);

        let c = Countdown::parse("90s").unwrap();
        assert_eq!(c.remaining().as_secs(), 90);

        let c = Countdown::parse("1:30").unwrap();
        assert_eq!(c.remaining().as_secs(), 90);

        let c = Countdown::parse("1:30:00").unwrap();
        assert_eq!(c.remaining().as_secs(), 5400);
    }

    #[test]
    fn test_countdown_update() {
        let mut c = Countdown::from_secs(10);
        c.start();
        c.update(1.0);
        assert_eq!(c.remaining().as_secs(), 9);

        c.update(10.0);
        assert!(c.is_finished());
    }

    #[test]
    fn test_pause_resume() {
        let mut c = Countdown::from_secs(10);
        c.start();
        c.update(1.0);

        c.pause();
        c.update(5.0); // Should not decrease while paused
        assert_eq!(c.remaining().as_secs(), 9);

        c.start();
        c.update(1.0);
        assert_eq!(c.remaining().as_secs(), 8);
    }
}
