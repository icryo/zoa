//! Storm demo - billowing cloud with lightning strikes
//!
//! Press SPACE to trigger lightning!

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    DefaultTerminal,
};

const TARGET_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

/// Simple RNG
fn rand() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = Cell::new(12345);
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        (x as f32) / (u32::MAX as f32)
    })
}

fn rand_range(min: f32, max: f32) -> f32 {
    min + rand() * (max - min)
}

/// A raindrop
struct Raindrop {
    x: f32,
    y: f32,
    speed: f32,
    length: f32,
}

impl Raindrop {
    fn new(width: f32) -> Self {
        Self {
            x: rand() * width,
            y: rand() * -20.0, // Start above screen
            speed: rand_range(15.0, 25.0),
            length: rand_range(1.0, 3.0),
        }
    }

    fn update(&mut self, dt: f32, width: f32, height: f32) {
        self.y += self.speed * dt;
        // Reset when off screen
        if self.y > height + 5.0 {
            self.x = rand() * width;
            self.y = rand_range(-10.0, -2.0);
            self.speed = rand_range(15.0, 25.0);
        }
    }
}

/// Lightning bolt segment
struct LightningBolt {
    segments: Vec<(f32, f32, f32, f32)>, // (x1, y1, x2, y2)
    brightness: f32,
    lifetime: f32,
}

impl LightningBolt {
    fn new(start_x: f32, start_y: f32, end_y: f32) -> Self {
        let mut segments = Vec::new();
        let mut x = start_x;
        let mut y = start_y;

        // Generate jagged path
        while y < end_y {
            let next_y = (y + rand_range(2.0, 5.0)).min(end_y);
            let next_x = x + rand_range(-3.0, 3.0);

            segments.push((x, y, next_x, next_y));

            // Chance for branch
            if rand() < 0.3 && y > start_y + 5.0 {
                let branch_len = rand_range(3.0, 8.0);
                let branch_dir = if rand() < 0.5 { -1.0 } else { 1.0 };
                let bx = next_x + branch_dir * rand_range(2.0, 5.0);
                let by = next_y + branch_len;
                segments.push((next_x, next_y, bx, by));
            }

            x = next_x;
            y = next_y;
        }

        Self {
            segments,
            brightness: 1.0,
            lifetime: 0.3,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        self.lifetime -= dt;
        self.brightness = (self.lifetime / 0.3).max(0.0);
        self.lifetime > 0.0
    }
}

/// Cloud puff (metaball-style)
struct CloudPuff {
    // Current position
    x: f32,
    y: f32,
    // Start position (compact center)
    start_x: f32,
    start_y: f32,
    // End position (spread out)
    end_x: f32,
    end_y: f32,
    // Size
    radius: f32,
    max_radius: f32,
    // Timing
    birth_time: f32,
    // Animation
    drift_speed: f32,
    drift_phase: f32,
    bob_speed: f32,
    bob_phase: f32,
    pulse_speed: f32,
    pulse_phase: f32,
    // Depth layer (0=back/dark, 1=front/light)
    depth: f32,
}

impl CloudPuff {
    fn new(start_x: f32, start_y: f32, end_x: f32, end_y: f32, max_radius: f32, birth_time: f32, depth: f32) -> Self {
        Self {
            x: start_x,
            y: start_y,
            start_x,
            start_y,
            end_x,
            end_y,
            radius: 0.0,
            max_radius,
            birth_time,
            drift_speed: rand_range(0.5, 1.2),
            drift_phase: rand() * std::f32::consts::TAU,
            bob_speed: rand_range(0.3, 0.7),
            bob_phase: rand() * std::f32::consts::TAU,
            pulse_speed: rand_range(0.8, 1.8),
            pulse_phase: rand() * std::f32::consts::TAU,
            depth,
        }
    }

    fn update(&mut self, time: f32, cloud_scale: f32, center_x: f32) {
        // Scale affects growth - each puff has a threshold based on birth_time
        let threshold = self.birth_time;
        let local_scale = ((cloud_scale - threshold) / (1.0 - threshold).max(0.01)).clamp(0.0, 1.0);

        // Ease-out for smooth expansion
        let eased = 1.0 - (1.0 - local_scale).powi(2);

        // Expand outward AND upward as it grows
        self.x = self.start_x + (self.end_x - self.start_x) * eased;
        self.y = self.start_y + (self.end_y - self.start_y) * eased;
        self.radius = self.max_radius * eased;

        // Horizontal drift (more at edges)
        let edge_factor = ((self.x - center_x).abs() / 20.0).min(1.0);
        let drift = (time * self.drift_speed + self.drift_phase).sin() * (0.05 + edge_factor * 0.1);
        self.x += drift;

        // Vertical bobbing
        let bob = (time * self.bob_speed + self.bob_phase).sin() * 0.3;
        self.y += bob;
    }

    fn current_radius(&self, time: f32) -> f32 {
        let pulse = (time * self.pulse_speed + self.pulse_phase).sin() * 0.2 + 1.0;
        self.radius * pulse
    }
}

struct Storm {
    cloud_puffs: Vec<CloudPuff>,
    raindrops: Vec<Raindrop>,
    lightning: Option<LightningBolt>,
    time: f32,
    flash: f32,        // Screen flash intensity
    cloud_scale: f32,  // User-controlled cloud size (0.0 to 1.0)
    center_x: f32,     // Cloud center
    width: f32,
    height: f32,
}

impl Storm {
    fn new(width: f32, height: f32) -> Self {
        let center_x = width * 0.5;
        let start_y = height * 0.25;  // All puffs start here (compact)
        let mut cloud_puffs = Vec::new();

        // Back layer (dark) - these spread wide
        for i in 0..8 {
            let angle = (i as f32 / 8.0) * std::f32::consts::TAU + rand_range(-0.2, 0.2);
            let start_dist = rand_range(2.0, 5.0);
            let end_dist = rand_range(20.0, 30.0);  // Spread far
            let start_x = center_x + angle.cos() * start_dist;
            let end_x = center_x + angle.cos() * end_dist;
            let end_y = height * 0.06 + rand_range(0.0, 4.0);
            let r = rand_range(6.0, 9.0);
            let birth = rand_range(0.0, 0.3);
            cloud_puffs.push(CloudPuff::new(start_x, start_y, end_x, end_y, r, birth, 0.0));
        }

        // Middle layer - medium spread
        for i in 0..10 {
            let angle = (i as f32 / 10.0) * std::f32::consts::TAU + rand_range(-0.3, 0.3);
            let start_dist = rand_range(1.0, 4.0);
            let end_dist = rand_range(12.0, 20.0);
            let start_x = center_x + angle.cos() * start_dist;
            let end_x = center_x + angle.cos() * end_dist;
            let end_y = height * 0.08 + rand_range(2.0, 6.0);
            let r = rand_range(5.0, 8.0);
            let birth = rand_range(0.2, 0.5);
            cloud_puffs.push(CloudPuff::new(start_x, start_y, end_x, end_y, r, birth, 0.4));
        }

        // Front/top layer (bright) - stays more central
        for _ in 0..6 {
            let start_x = center_x + rand_range(-3.0, 3.0);
            let end_x = center_x + rand_range(-8.0, 8.0);
            let end_y = height * 0.05 + rand_range(0.0, 3.0);
            let r = rand_range(7.0, 10.0);
            let birth = rand_range(0.1, 0.4);
            cloud_puffs.push(CloudPuff::new(start_x, start_y, end_x, end_y, r, birth, 1.0));
        }

        // Bottom bulges (dark underbelly)
        for i in 0..6 {
            let offset = (i as f32 - 2.5) * 4.0;
            let start_x = center_x + offset * 0.3;
            let end_x = center_x + offset + rand_range(-2.0, 2.0);
            let end_y = height * 0.12 + rand_range(4.0, 8.0);
            let r = rand_range(4.0, 6.0);
            let birth = rand_range(0.5, 0.8);
            cloud_puffs.push(CloudPuff::new(start_x, start_y + 2.0, end_x, end_y, r, birth, 0.2));
        }

        // Create raindrops
        let raindrops: Vec<Raindrop> = (0..100).map(|_| Raindrop::new(width)).collect();

        Self {
            cloud_puffs,
            raindrops,
            lightning: None,
            time: 0.0,
            flash: 0.0,
            cloud_scale: 0.0,  // Start with no cloud
            center_x,
            width,
            height,
        }
    }

    fn grow(&mut self, amount: f32) {
        self.cloud_scale = (self.cloud_scale + amount).clamp(0.0, 1.0);
    }

    fn strike(&mut self) {
        // Pick a random spot in the cloud (centered)
        let x = self.width * 0.5 + rand_range(-15.0, 15.0);
        let cloud_bottom = self.height * 0.20;
        let ground = self.height * 0.95;

        self.lightning = Some(LightningBolt::new(x, cloud_bottom, ground));
        self.flash = 1.0;
    }

    fn update(&mut self, dt: f32) {
        self.time += dt;

        // Update cloud
        for puff in &mut self.cloud_puffs {
            puff.update(self.time, self.cloud_scale, self.center_x);
        }

        // Update rain
        for drop in &mut self.raindrops {
            drop.update(dt, self.width, self.height);
        }

        // Update lightning
        if let Some(ref mut bolt) = self.lightning {
            if !bolt.update(dt) {
                self.lightning = None;
            }
        }

        // Fade flash
        self.flash = (self.flash - dt * 5.0).max(0.0);
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    fn render(&self, buf: &mut Buffer, area: Rect) {
        let width = area.width as f32;
        let height = area.height as f32;

        if width < 1.0 || height < 1.0 {
            return;
        }

        // Background flash from lightning
        let bg_color = if self.flash > 0.5 {
            Color::Rgb(40, 40, 50)
        } else if self.flash > 0.0 {
            Color::Rgb(15, 15, 25)
        } else {
            Color::Rgb(5, 5, 15)
        };

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_bg(bg_color).set_char(' ');
            }
        }

        // Render cloud using density sampling with depth-based shading
        for sy in 0..area.height {
            for sx in 0..area.width {
                let px = sx as f32;
                let py = sy as f32;

                // Sample cloud density and average depth
                let mut density = 0.0;
                let mut weighted_depth = 0.0;
                for puff in &self.cloud_puffs {
                    let dx = px - puff.x;
                    let dy = (py - puff.y) * 2.0; // Stretch vertically for terminal chars
                    let dist = (dx * dx + dy * dy).sqrt();
                    let r = puff.current_radius(self.time);
                    if dist < r * 2.0 && r > 0.1 {
                        let contribution = (1.0 - dist / (r * 2.0)).max(0.0).powf(2.0);
                        density += contribution;
                        weighted_depth += contribution * puff.depth;
                    }
                }

                if density > 0.15 {
                    // Average depth weighted by contribution
                    let avg_depth = if density > 0.0 { weighted_depth / density } else { 0.5 };

                    // Base brightness from depth (0=dark, 1=bright)
                    let base_gray = 35.0 + avg_depth * 50.0;
                    // Density adds some brightness
                    let density_boost = density.min(1.5) * 25.0;
                    let gray = (base_gray + density_boost) as u8;

                    // Slight blue tint for storm clouds, darker at bottom
                    let r = (gray as f32 * 0.85) as u8;
                    let g = (gray as f32 * 0.9) as u8;
                    let b = gray + 15;

                    let ch = match density {
                        d if d > 1.0 => '█',
                        d if d > 0.7 => '▓',
                        d if d > 0.4 => '▒',
                        _ => '░',
                    };
                    buf[(area.x + sx, area.y + sy)]
                        .set_char(ch)
                        .set_fg(Color::Rgb(r, g, b));
                }
            }
        }

        // Render rain - intensity increases as cloud forms
        // Rain intensity matches cloud scale
        let visible_drops = (self.raindrops.len() as f32 * self.cloud_scale) as usize;
        let rain_color = Color::Rgb(100, 150, 255);
        for drop in self.raindrops.iter().take(visible_drops) {
            let x = drop.x as u16;
            let y = drop.y as u16;
            if x < area.width && y < area.height && y > (self.height * 0.2) as u16 {
                buf[(area.x + x, area.y + y)]
                    .set_char('│')
                    .set_fg(rain_color);
                // Trail
                if y > 0 && drop.length > 1.5 {
                    buf[(area.x + x, area.y + y.saturating_sub(1))]
                        .set_char('|')
                        .set_fg(Color::Rgb(60, 90, 150));
                }
            }
        }

        // Render lightning
        if let Some(ref bolt) = self.lightning {
            let bright = (bolt.brightness * 255.0) as u8;
            let color = Color::Rgb(255, 255, bright);
            let dim_color = Color::Rgb(200, 200, bright / 2);

            for (x1, y1, x2, y2) in &bolt.segments {
                // Draw line using Bresenham-ish
                let steps = ((x2 - x1).abs().max((y2 - y1).abs()) as i32).max(1);
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let x = (x1 + (x2 - x1) * t) as u16;
                    let y = (y1 + (y2 - y1) * t) as u16;
                    if x < area.width && y < area.height {
                        let ch = if rand() < 0.5 { '╲' } else { '╱' };
                        buf[(area.x + x, area.y + y)]
                            .set_char(if bolt.brightness > 0.7 { '█' } else { ch })
                            .set_fg(if bolt.brightness > 0.5 { color } else { dim_color });
                    }
                }
            }

            // Glow around main bolt (first segment's x)
            if bolt.brightness > 0.5 && !bolt.segments.is_empty() {
                let main_x = bolt.segments[0].0 as u16;
                for y in 0..area.height {
                    for dx in [-1i16, 1] {
                        let x = (main_x as i16 + dx) as u16;
                        if x < area.width {
                            let cell = &mut buf[(area.x + x, area.y + y)];
                            if cell.symbol() == " " {
                                cell.set_fg(Color::Rgb(80, 80, 120));
                            }
                        }
                    }
                }
            }
        }

        // Ground
        let ground_y = (self.height * 0.95) as u16;
        if ground_y < area.height {
            for x in 0..area.width {
                buf[(area.x + x, area.y + ground_y)]
                    .set_char('▀')
                    .set_fg(Color::Rgb(30, 50, 30));
            }
        }
    }
}

struct App {
    storm: Storm,
    last_frame: Instant,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        Self {
            storm: Storm::new(width as f32, height as f32),
            last_frame: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.storm.update(dt);
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.storm.strike();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.storm.grow(0.05);  // Grow cloud
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.storm.grow(-0.05); // Shrink cloud
            }
            _ => {}
        }
        false
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.storm.resize(width as f32, height as f32);
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let terminal = ratatui::init();
    let result = run(terminal);

    ratatui::restore();
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height.saturating_sub(1));

    loop {
        let frame_start = Instant::now();

        if event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key.code) {
                        break;
                    }
                }
                Event::Resize(w, h) => {
                    app.resize(w, h.saturating_sub(1));
                }
                _ => {}
            }
        }

        app.update();

        terminal.draw(|frame| {
            let area = frame.area();
            let storm_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));

            app.storm.render(frame.buffer_mut(), storm_area);

            // Status bar with cloud scale indicator
            let scale_pct = (app.storm.cloud_scale * 100.0) as u8;
            let status = format!(" [↑/↓] Cloud: {}% | [SPACE] Lightning! | [Q]uit ", scale_pct);
            let status_widget = Paragraph::new(status)
                .style(Style::default().fg(Color::DarkGray).bg(Color::Black));
            frame.render_widget(
                status_widget,
                Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            );
        })?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}
