//! GIF widget demo - shows GIF at different sizes
//!
//! Run with: cargo run --example gif_demo -- samples/test.gif

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

use zoa::ZoaWidget;

fn main() -> color_eyre::Result<()> {
    let args: Vec<String> = env::args().collect();
    let gif_path = args.get(1).map(|s| s.as_str()).unwrap_or("samples/test.gif");

    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let terminal = ratatui::init();
    let result = run(terminal, gif_path);

    ratatui::restore();
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(mut terminal: DefaultTerminal, gif_path: &str) -> color_eyre::Result<()> {
    let mut widgets: Vec<ZoaWidget> = (0..4).map(|_| ZoaWidget::default()).collect();
    
    // Load GIF into all widgets
    let path = Path::new(gif_path);
    for widget in &mut widgets {
        if let Err(e) = widget.load_gif(path) {
            eprintln!("Failed to load GIF: {}", e);
            return Ok(());
        }
    }

    let mut last_update = Instant::now();

    loop {
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }

        let now = Instant::now();
        let dt = (now - last_update).as_secs_f32();
        last_update = now;

        for widget in &mut widgets {
            widget.update(dt);
        }

        terminal.draw(|frame| draw(frame, &mut widgets))?;
    }

    Ok(())
}

fn draw(frame: &mut Frame, widgets: &mut [ZoaWidget]) {
    let area = frame.area();

    // 2x2 grid with different sized panels
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(rows[1]);

    let panels = [top[0], top[1], bottom[0], bottom[1]];
    let labels = ["Large", "Medium", "Small", "Wide"];

    for (i, (panel, label)) in panels.iter().zip(labels.iter()).enumerate() {
        let block = Block::default()
            .title(format!(" {} ({}x{}) ", label, panel.width, panel.height))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(*panel);
        frame.render_widget(block, *panel);
        frame.render_widget(&mut widgets[i], inner);
    }

    // Help at bottom
    let help = Paragraph::new(" [Q] Quit - GIF scales to each panel size ")
        .style(Style::default().fg(Color::DarkGray))
        .centered();

    let help_area = Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1);
    frame.render_widget(help, help_area);
}
