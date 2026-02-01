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
    widgets::{Paragraph, Widget},
    DefaultTerminal, Frame,
};

use zoa::{AsciiBuffer, CharStyle, ColorPalette, ParticlePreset, ParticleSystem};

const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

struct App {
    particles: ParticleSystem,
    buffer: AsciiBuffer,
    char_style: CharStyle,
    palette: ColorPalette,
    last_frame: Instant,
}

impl App {
    fn new() -> Self {
        Self {
            particles: ParticleSystem::with_preset(ParticlePreset::Fire),
            buffer: AsciiBuffer::new(80, 24),
            char_style: CharStyle::Blocks,
            palette: ColorPalette::Fire,
            last_frame: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.particles.update(dt);
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char(' ') | KeyCode::Tab => {
                self.particles.cycle_preset();
                // Match palette to preset
                self.palette = match self.particles.preset() {
                    ParticlePreset::Fire => ColorPalette::Fire,
                    ParticlePreset::Smoke => ColorPalette::Grayscale,
                    ParticlePreset::Rain => ColorPalette::Cyan,
                    ParticlePreset::Snow => ColorPalette::Grayscale,
                    ParticlePreset::Sparks => ColorPalette::Fire,
                    ParticlePreset::Fountain => ColorPalette::Cyan,
                    ParticlePreset::Explosion => ColorPalette::Fire,
                    ParticlePreset::Matrix => ColorPalette::Matrix,
                };
            }
            KeyCode::Char('b') => {
                self.particles.burst(50);
            }
            KeyCode::Char('s') => {
                self.char_style = self.char_style.next();
            }
            KeyCode::Char('c') => {
                self.palette = self.palette.next();
            }
            _ => {}
        }
        false
    }
}

struct AsciiWidget<'a> {
    buffer: &'a AsciiBuffer,
    palette: ColorPalette,
    char_style: CharStyle,
}

impl<'a> Widget for AsciiWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in 0..area.height.min(self.buffer.height) {
            for x in 0..area.width.min(self.buffer.width) {
                if let Some(fragment) = self.buffer.get(x, y) {
                    let ch = self.char_style.to_char(fragment.luminance);
                    let color = self.palette.to_color(fragment.luminance);
                    buf[(area.x + x, area.y + y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(color));
                }
            }
        }
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
    let mut app = App::new();

    loop {
        let frame_start = Instant::now();

        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.handle_key(key.code) {
                    break;
                }
            }
        }

        app.update();

        terminal.draw(|frame| {
            let area = frame.area();

            // Render particles to buffer
            app.buffer.resize(area.width, area.height.saturating_sub(2));
            app.buffer.clear();
            app.particles.render(&mut app.buffer);

            // Draw particles
            let particle_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
            let widget = AsciiWidget {
                buffer: &app.buffer,
                palette: app.palette,
                char_style: app.char_style,
            };
            frame.render_widget(widget, particle_area);

            // Draw status
            draw_status(frame, &app);
        })?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}

fn draw_status(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.height < 2 {
        return;
    }

    let status = format!(
        " {} | {} particles | [Space] Preset | [B]urst | [S]tyle: {} | [C]olor: {} | [Q]uit ",
        app.particles.preset().name(),
        app.particles.particle_count(),
        app.char_style.name(),
        app.palette.name(),
    );

    let widget = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(widget, Rect::new(area.x, area.y + area.height - 1, area.width, 1));
}
