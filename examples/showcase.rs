//! Showcase demo: demonstrates all orb features
//!
//! Shows 3D shapes, GIF animation, countdown timer, and various styles
//! in a multi-panel layout.
//!
//! Run with: cargo run --example showcase
//!
//! Optional: cargo run --example showcase -- path/to/animation.gif

use std::env;
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

/// Content type for each panel
#[derive(Clone, Copy, PartialEq)]
enum PanelContent {
    Shape3D,
    Countdown,
    Gif,
}

impl PanelContent {
    fn name(self) -> &'static str {
        match self {
            Self::Shape3D => "3D Shape",
            Self::Countdown => "Countdown",
            Self::Gif => "GIF",
        }
    }
}

struct Panel {
    widget: ZoaWidget,
    content: PanelContent,
    style_index: usize,
    palette_index: usize,
}

impl Panel {
    fn new_shape(shape: Shape, style: CharStyle, palette: ColorPalette) -> Self {
        let mut widget = ZoaWidget::default();
        widget.set_shape(shape);
        widget.set_char_style(style);
        widget.set_palette(palette);
        Self {
            widget,
            content: PanelContent::Shape3D,
            style_index: 0,
            palette_index: 0,
        }
    }

    fn new_countdown(duration: &str, style: CharStyle, palette: ColorPalette) -> Self {
        let mut widget = ZoaWidget::default();
        widget.set_char_style(style);
        widget.set_palette(palette);
        let _ = widget.start_countdown_from_str(duration);
        Self {
            widget,
            content: PanelContent::Countdown,
            style_index: 0,
            palette_index: 0,
        }
    }

    fn new_gif(path: &Path, style: CharStyle, palette: ColorPalette) -> Option<Self> {
        let mut widget = ZoaWidget::default();
        widget.set_char_style(style);
        widget.set_palette(palette);
        widget.load_gif(path).ok()?;
        Some(Self {
            widget,
            content: PanelContent::Gif,
            style_index: 0,
            palette_index: 0,
        })
    }

    fn cycle_style(&mut self) {
        let styles = [
            CharStyle::Ascii,
            CharStyle::Blocks,
            CharStyle::Braille,
            CharStyle::Hatching,
            CharStyle::Dots,
        ];
        self.style_index = (self.style_index + 1) % styles.len();
        self.widget.set_char_style(styles[self.style_index]);
    }

    fn cycle_palette(&mut self) {
        let palettes = [
            ColorPalette::Cyan,
            ColorPalette::Fire,
            ColorPalette::Matrix,
            ColorPalette::Purple,
            ColorPalette::Rainbow,
        ];
        self.palette_index = (self.palette_index + 1) % palettes.len();
        self.widget.set_palette(palettes[self.palette_index]);
    }
}

struct App {
    panels: Vec<Panel>,
    selected: usize,
    last_update: Instant,
    has_gif: bool,
}

impl App {
    fn new(gif_path: Option<&Path>) -> Self {
        let mut panels = vec![
            // Top-left: Torus with Cyan/ASCII
            Panel::new_shape(Shape::Torus, CharStyle::Ascii, ColorPalette::Cyan),
            // Top-right: Countdown with Hatching/Fire
            Panel::new_countdown("5:00", CharStyle::Hatching, ColorPalette::Fire),
            // Bottom-left: Cube with Braille/Matrix
            Panel::new_shape(Shape::Cube, CharStyle::Braille, ColorPalette::Matrix),
        ];

        // Bottom-right: GIF if provided, otherwise Sphere
        let has_gif = if let Some(path) = gif_path {
            if let Some(gif_panel) = Panel::new_gif(path, CharStyle::Dots, ColorPalette::Purple) {
                panels.push(gif_panel);
                true
            } else {
                panels.push(Panel::new_shape(
                    Shape::Sphere,
                    CharStyle::Dots,
                    ColorPalette::Purple,
                ));
                false
            }
        } else {
            // Try default GIF path
            let default_gif = Path::new("samples/test.gif");
            if let Some(gif_panel) =
                Panel::new_gif(default_gif, CharStyle::Dots, ColorPalette::Purple)
            {
                panels.push(gif_panel);
                true
            } else {
                panels.push(Panel::new_shape(
                    Shape::Sphere,
                    CharStyle::Dots,
                    ColorPalette::Purple,
                ));
                false
            }
        };

        Self {
            panels,
            selected: 0,
            last_update: Instant::now(),
            has_gif,
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

            // Panel selection (1-4)
            KeyCode::Char('1') => self.selected = 0,
            KeyCode::Char('2') => self.selected = 1.min(self.panels.len() - 1),
            KeyCode::Char('3') => self.selected = 2.min(self.panels.len() - 1),
            KeyCode::Char('4') => self.selected = 3.min(self.panels.len() - 1),

            // Tab to cycle selection
            KeyCode::Tab => self.selected = (self.selected + 1) % self.panels.len(),

            // Space: shape cycle or countdown pause
            KeyCode::Char(' ') => {
                let panel = &mut self.panels[self.selected];
                match panel.content {
                    PanelContent::Shape3D => {
                        let shape = panel.widget.config().shape.next();
                        panel.widget.set_shape(shape);
                    }
                    PanelContent::Countdown => {
                        panel.widget.toggle_countdown_pause();
                    }
                    PanelContent::Gif => {}
                }
            }

            // Style cycle for selected panel
            KeyCode::Char('s') => {
                self.panels[self.selected].cycle_style();
            }

            // Color cycle for selected panel
            KeyCode::Char('c') => {
                self.panels[self.selected].cycle_palette();
            }

            // Zoom selected panel
            KeyCode::Up => {
                let zoom = (self.panels[self.selected].widget.config().zoom + 0.1).min(3.0);
                self.panels[self.selected].widget.set_zoom(zoom);
            }
            KeyCode::Down => {
                let zoom = (self.panels[self.selected].widget.config().zoom - 0.1).max(0.3);
                self.panels[self.selected].widget.set_zoom(zoom);
            }

            // Reset countdown
            KeyCode::Char('r') => {
                let panel = &mut self.panels[self.selected];
                if panel.content == PanelContent::Countdown {
                    panel.widget.reset_countdown();
                }
            }

            // Wireframe toggle for 3D shapes
            KeyCode::Char('w') => {
                let panel = &mut self.panels[self.selected];
                if panel.content == PanelContent::Shape3D {
                    let mode = panel.widget.config().render_mode.next();
                    panel.widget.set_render_mode(mode);
                }
            }

            _ => {}
        }
        false
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = env::args().collect();
    let gif_path = args.get(1).map(|s| Path::new(s.as_str()));

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let terminal = ratatui::init();
    let result = run(terminal, gif_path);

    ratatui::restore();
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(mut terminal: DefaultTerminal, gif_path: Option<&Path>) -> color_eyre::Result<()> {
    let mut app = App::new(gif_path);

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
    let main_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));

    // 2x2 grid
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let areas = [top[0], top[1], bottom[0], bottom[1]];

    for (i, panel_area) in areas.iter().enumerate() {
        if i < app.panels.len() {
            draw_panel(frame, *panel_area, &mut app.panels[i], i == app.selected, i);
        }
    }

    // Help bar
    draw_help_bar(frame, area, app);
}

fn draw_panel(frame: &mut Frame, area: Rect, panel: &mut Panel, selected: bool, index: usize) {
    let config = panel.widget.config();

    let content_info = match panel.content {
        PanelContent::Shape3D => {
            let shape = match config.shape {
                Shape::Torus => "Torus",
                Shape::Cube => "Cube",
                Shape::Sphere => "Sphere",
            };
            format!("{} | {}", shape, config.render_mode.name())
        }
        PanelContent::Countdown => {
            if panel.widget.has_countdown() {
                "Timer".to_string()
            } else {
                "Countdown".to_string()
            }
        }
        PanelContent::Gif => "Animation".to_string(),
    };

    let title = format!(
        " [{}] {} | {} | {} | {:.1}x ",
        index + 1,
        content_info,
        config.char_style.name(),
        config.palette.name(),
        config.zoom,
    );

    let border_color = if selected {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(&mut panel.widget, inner);
}

fn draw_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let panel = &app.panels[app.selected];
    let panel_type = panel.content.name();

    let context_help = match panel.content {
        PanelContent::Shape3D => "[Space] Shape [W] Wire",
        PanelContent::Countdown => "[Space] Pause [R] Reset",
        PanelContent::Gif => "[S] Style [C] Color",
    };

    let gif_note = if app.has_gif { "" } else { " (no GIF loaded)" };

    let help = format!(
        " [1-4/Tab] Select | {} | [S] Style | [C] Color | [{}{}] Zoom | [Q] Quit{}",
        context_help,
        '\u{2191}',
        '\u{2193}',
        gif_note,
    );

    let status = format!(" Selected: [{}] {} ", app.selected + 1, panel_type);

    // Two-line help
    let help_widget = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .centered();

    let status_widget = Paragraph::new(status)
        .style(Style::default().fg(Color::Cyan))
        .centered();

    if area.height > 1 {
        let bar1 = Rect::new(area.x, area.y + area.height - 2, area.width, 1);
        let bar2 = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        frame.render_widget(status_widget, bar1);
        frame.render_widget(help_widget, bar2);
    }
}
