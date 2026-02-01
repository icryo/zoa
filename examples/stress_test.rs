//! Stress test: 8 oddly-shaped panels to test rendering edge cases
//!
//! Tests extreme aspect ratios, tiny sizes, and mixed content types.
//!
//! Run with: cargo run --example stress_test

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

use zoa::{CharStyle, ColorPalette, ZoaWidget, Shape};

const TARGET_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

struct Panel {
    widget: ZoaWidget,
    label: &'static str,
}

impl Panel {
    fn shape(label: &'static str, shape: Shape, style: CharStyle, palette: ColorPalette) -> Self {
        let mut widget = ZoaWidget::default();
        widget.set_shape(shape);
        widget.set_char_style(style);
        widget.set_palette(palette);
        Self { widget, label }
    }

    fn countdown(label: &'static str, duration: &str, style: CharStyle, palette: ColorPalette) -> Self {
        let mut widget = ZoaWidget::default();
        widget.set_char_style(style);
        widget.set_palette(palette);
        let _ = widget.start_countdown_from_str(duration);
        Self { widget, label }
    }

    fn gif(label: &'static str, style: CharStyle, palette: ColorPalette) -> Self {
        let mut widget = ZoaWidget::default();
        widget.set_char_style(style);
        widget.set_palette(palette);
        // Try to load test GIF
        let _ = widget.load_gif(Path::new("samples/test.gif"));
        Self { widget, label }
    }
}

struct App {
    panels: Vec<Panel>,
    last_update: Instant,
    layout_mode: usize,
}

impl App {
    fn new() -> Self {
        let panels = vec![
            // Various shapes with different styles
            Panel::shape("Torus/Tiny", Shape::Torus, CharStyle::Ascii, ColorPalette::Cyan),
            Panel::shape("Cube/Wide", Shape::Cube, CharStyle::Braille, ColorPalette::Fire),
            Panel::shape("Sphere/Tall", Shape::Sphere, CharStyle::Blocks, ColorPalette::Matrix),
            Panel::countdown("Timer/Slim", "2:30", CharStyle::Hatching, ColorPalette::Purple),
            Panel::gif("GIF/Square", CharStyle::Dots, ColorPalette::Rainbow),
            Panel::shape("Torus/Mini", Shape::Torus, CharStyle::Dense, ColorPalette::Grayscale),
            Panel::countdown("Timer/Bar", "1:00", CharStyle::Minimal, ColorPalette::Cyan),
            Panel::shape("Cube/Odd", Shape::Cube, CharStyle::Stars, ColorPalette::Fire),
        ];

        Self {
            panels,
            last_update: Instant::now(),
            layout_mode: 0,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32();
        self.last_update = now;

        for panel in &mut self.panels {
            panel.widget.update(dt);
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Tab | KeyCode::Char(' ') => {
                self.layout_mode = (self.layout_mode + 1) % 4;
            }
            KeyCode::Char('r') => {
                // Reset all countdowns
                for panel in &mut self.panels {
                    panel.widget.reset_countdown();
                }
            }
            _ => {}
        }
        false
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

        terminal.draw(|frame| draw(frame, &mut app))?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Reserve bottom for help
    let main_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));

    match app.layout_mode {
        0 => draw_chaotic_layout(frame, main_area, app),
        1 => draw_extreme_horizontal(frame, main_area, app),
        2 => draw_extreme_vertical(frame, main_area, app),
        _ => draw_mixed_extreme(frame, main_area, app),
    }

    // Help bar
    let layout_name = match app.layout_mode {
        0 => "Chaotic",
        1 => "Horizontal",
        2 => "Vertical",
        _ => "Mixed",
    };
    let help = format!(
        " [Tab/Space] Layout: {} | [R] Reset timers | [Q] Quit ",
        layout_name
    );
    let help_widget = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .centered();

    let bar = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    frame.render_widget(help_widget, bar);
}

/// Chaotic layout with various odd sizes
fn draw_chaotic_layout(frame: &mut Frame, area: Rect, app: &mut App) {
    // Split into 3 rows with uneven heights
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),      // Tiny row
            Constraint::Percentage(50), // Big row
            Constraint::Min(8),         // Rest
        ])
        .split(area);

    // Top row: 3 tiny panels
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(15),     // Very narrow
            Constraint::Percentage(60), // Wide
            Constraint::Min(10),        // Rest
        ])
        .split(rows[0]);

    // Middle row: 2 panels, one very wide
    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75), // Very wide
            Constraint::Min(8),         // Narrow
        ])
        .split(rows[1]);

    // Bottom row: 3 panels with odd sizes
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Percentage(40),
            Constraint::Min(15),
        ])
        .split(rows[2]);

    let areas = [
        top[0], top[1], top[2],
        middle[0], middle[1],
        bottom[0], bottom[1], bottom[2],
    ];

    for (i, panel_area) in areas.iter().enumerate() {
        if i < app.panels.len() {
            draw_panel(frame, *panel_area, &mut app.panels[i]);
        }
    }
}

/// Extreme horizontal: 8 very tall, narrow panels
fn draw_extreme_horizontal(frame: &mut Frame, area: Rect, app: &mut App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Percentage(15),
            Constraint::Length(10),
            Constraint::Percentage(20),
            Constraint::Length(14),
            Constraint::Min(5),
        ])
        .split(area);

    for (i, panel_area) in cols.iter().enumerate() {
        if i < app.panels.len() {
            draw_panel(frame, *panel_area, &mut app.panels[i]);
        }
    }
}

/// Extreme vertical: 8 very wide, short panels
fn draw_extreme_vertical(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(2),
            Constraint::Percentage(15),
            Constraint::Length(5),
            Constraint::Percentage(20),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(area);

    for (i, panel_area) in rows.iter().enumerate() {
        if i < app.panels.len() {
            draw_panel(frame, *panel_area, &mut app.panels[i]);
        }
    }
}

/// Mixed extreme: combination of very small and normal
fn draw_mixed_extreme(frame: &mut Frame, area: Rect, app: &mut App) {
    // Create a complex nested layout
    let main_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Left side: stack of tiny panels
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Micro
            Constraint::Length(4),  // Tiny
            Constraint::Length(5),  // Small
            Constraint::Min(3),     // Rest
        ])
        .split(main_split[0]);

    // Right side: mix of sizes
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_split[1]);

    let right_top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(20)])
        .split(right_rows[0]);

    let right_bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Min(8)])
        .split(right_rows[1]);

    let areas = [
        left[0], left[1], left[2], left[3],
        right_top[0], right_top[1],
        right_bottom[0], right_bottom[1],
    ];

    for (i, panel_area) in areas.iter().enumerate() {
        if i < app.panels.len() {
            draw_panel(frame, *panel_area, &mut app.panels[i]);
        }
    }
}

fn draw_panel(frame: &mut Frame, area: Rect, panel: &mut Panel) {
    let size_info = format!("{}x{}", area.width, area.height);

    let block = Block::default()
        .title(format!(" {} [{}] ", panel.label, size_info))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Only render if we have some space
    if inner.width > 0 && inner.height > 0 {
        frame.render_widget(&mut panel.widget, inner);
    }
}
