//! Multi-layout demo: 3 shell outputs + 1 rotating orb with switchable layouts
//!
//! Run with: cargo run --example quad_demo

use std::io;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

use zoa::{CharStyle, ColorPalette, ZoaWidget, Shape};

const TARGET_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

/// Available layout modes
#[derive(Clone, Copy, Default)]
enum LayoutMode {
    #[default]
    Quad,       // 2x2 grid
    Sidebar,    // Orb left, shells stacked right
    LargeOrb,   // Large orb top, shells bottom row
    Fullscreen, // Orb only
}

impl LayoutMode {
    fn next(self) -> Self {
        match self {
            Self::Quad => Self::Sidebar,
            Self::Sidebar => Self::LargeOrb,
            Self::LargeOrb => Self::Fullscreen,
            Self::Fullscreen => Self::Quad,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Quad => "Quad",
            Self::Sidebar => "Sidebar",
            Self::LargeOrb => "Large",
            Self::Fullscreen => "Full",
        }
    }
}

struct App {
    orb: ZoaWidget,
    panels: [PanelContent; 3],
    layout: LayoutMode,
    last_update: Instant,
}

struct PanelContent {
    title: String,
    command: String,
    output: String,
}

impl PanelContent {
    fn new(title: &str, command: &str) -> Self {
        Self {
            title: title.to_string(),
            command: command.to_string(),
            output: String::new(),
        }
    }

    fn run(&mut self) {
        self.output = format!("$ {}\n", self.command);

        let result = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &self.command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        } else {
            Command::new("sh")
                .args(["-c", &self.command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        };

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.output.push_str(&stdout);
                if !stderr.is_empty() {
                    self.output.push_str(&stderr);
                }
            }
            Err(e) => {
                self.output.push_str(&format!("Error: {}\n", e));
            }
        }
    }
}

impl App {
    fn new() -> Self {
        let mut orb = ZoaWidget::default();
        orb.set_palette(ColorPalette::Cyan);
        orb.set_char_style(CharStyle::Ascii);

        let mut panels = [
            PanelContent::new("System", "uname -a"),
            PanelContent::new("Directory", "ls -la | head -10"),
            PanelContent::new("Git", "git log --oneline -5 2>/dev/null || echo 'Not a git repo'"),
        ];

        for panel in &mut panels {
            panel.run();
        }

        Self {
            orb,
            panels,
            layout: LayoutMode::default(),
            last_update: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32();
        self.last_update = now;
        self.orb.update(dt);
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,

            // Layout switching
            KeyCode::Tab => self.layout = self.layout.next(),

            // Shape controls
            KeyCode::Char(' ') => {
                let shape = self.orb.config().shape.next();
                self.orb.set_shape(shape);
            }

            // Style controls
            KeyCode::Char('s') => {
                let style = self.orb.config().char_style.next();
                self.orb.set_char_style(style);
            }

            // Color controls
            KeyCode::Char('c') => {
                let palette = self.orb.config().palette.next();
                self.orb.set_palette(palette);
            }

            // Wireframe toggle
            KeyCode::Char('w') => {
                let mode = self.orb.config().render_mode.next();
                self.orb.set_render_mode(mode);
            }

            // Zoom
            KeyCode::Up => {
                let zoom = (self.orb.config().zoom + 0.1).min(3.0);
                self.orb.set_zoom(zoom);
            }
            KeyCode::Down => {
                let zoom = (self.orb.config().zoom - 0.1).max(0.3);
                self.orb.set_zoom(zoom);
            }

            // Refresh panels
            KeyCode::Char('r') => {
                for panel in &mut self.panels {
                    panel.run();
                }
            }

            // Manual rotation
            KeyCode::Char('i') => self.orb.rotate(-0.15, 0.0),
            KeyCode::Char('k') => self.orb.rotate(0.15, 0.0),
            KeyCode::Char('j') => self.orb.rotate(0.0, -0.15),
            KeyCode::Char('l') => self.orb.rotate(0.0, 0.15),

            // Toggle auto-rotate
            KeyCode::Char('a') => {
                let auto = !self.orb.config().auto_rotate;
                self.orb.set_auto_rotate(auto);
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

    // Reserve bottom row for help
    let main_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));

    match app.layout {
        LayoutMode::Quad => draw_quad_layout(frame, main_area, app),
        LayoutMode::Sidebar => draw_sidebar_layout(frame, main_area, app),
        LayoutMode::LargeOrb => draw_large_orb_layout(frame, main_area, app),
        LayoutMode::Fullscreen => draw_fullscreen_layout(frame, main_area, app),
    }

    // Help bar
    draw_help_bar(frame, area, app);
}

fn draw_quad_layout(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    draw_shell_panel(frame, top[0], &app.panels[0]);
    draw_orb_panel(frame, top[1], &mut app.orb);
    draw_shell_panel(frame, bottom[0], &app.panels[1]);
    draw_shell_panel(frame, bottom[1], &app.panels[2]);
}

fn draw_sidebar_layout(frame: &mut Frame, area: Rect, app: &mut App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Orb on left
    draw_orb_panel(frame, cols[0], &mut app.orb);

    // Shells stacked on right
    let shells = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(cols[1]);

    for (i, shell_area) in shells.iter().enumerate() {
        draw_shell_panel(frame, *shell_area, &app.panels[i]);
    }
}

fn draw_large_orb_layout(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Large orb on top
    draw_orb_panel(frame, rows[0], &mut app.orb);

    // Shells in a row at bottom
    let shells = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(rows[1]);

    for (i, shell_area) in shells.iter().enumerate() {
        draw_shell_panel(frame, *shell_area, &app.panels[i]);
    }
}

fn draw_fullscreen_layout(frame: &mut Frame, area: Rect, app: &mut App) {
    draw_orb_panel(frame, area, &mut app.orb);
}

fn draw_shell_panel(frame: &mut Frame, area: Rect, panel: &PanelContent) {
    let block = Block::default()
        .title(format!(" {} ", panel.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = panel
        .output
        .lines()
        .take(inner.height as usize)
        .map(|s| Line::from(s.to_string()))
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, inner);
}

fn draw_orb_panel(frame: &mut Frame, area: Rect, orb: &mut ZoaWidget) {
    let config = orb.config();
    let shape_name = match config.shape {
        Shape::Torus => "Torus",
        Shape::Cube => "Cube",
        Shape::Sphere => "Sphere",
    };
    let auto = if config.auto_rotate { "Auto" } else { "Manual" };

    let title = format!(
        " {} | {} | {} | {} | {:.1}x | {} ",
        shape_name,
        config.char_style.name(),
        config.palette.name(),
        config.render_mode.name(),
        config.zoom,
        auto,
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(orb, inner);
}

fn draw_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let help = format!(
        " [Tab] Layout: {} | [Space] Shape | [S] Style | [C] Color | [W] Wire | [↑↓] Zoom | [IJKL] Rotate | [A] Auto | [R] Refresh | [Q] Quit ",
        app.layout.name()
    );

    let widget = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .centered();

    if area.height > 0 {
        let bar = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        frame.render_widget(widget, bar);
    }
}
