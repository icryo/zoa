use std::env;
use std::io;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
    DefaultTerminal, Frame,
};
use tachyonfx::{fx, Effect, Interpolation, Shader};

use zoa::{AsciiBuffer, AnimatedGif, CharStyle, ColorPalette, Countdown, Cube, Mesh, PixelMode, RenderMode, Renderer, Sphere, Torus, Vec3, CHAR_ASPECT};

const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

const DEFAULT_DENSITY: usize = 8;
const MIN_DENSITY: usize = 2;
const MAX_DENSITY: usize = 20;

const DEFAULT_SPEED: f32 = 1.0;
const MIN_SPEED: f32 = 0.1;
const MAX_SPEED: f32 = 5.0;

#[derive(Clone, Copy, PartialEq)]
enum ShapeType {
    Torus,
    Cube,
    Sphere,
    CustomMesh,
    AnimatedGif,
    Countdown,
}

impl ShapeType {
    fn next(self, has_custom: bool, has_gif: bool, has_countdown: bool) -> Self {
        match self {
            ShapeType::Torus => ShapeType::Cube,
            ShapeType::Cube => ShapeType::Sphere,
            ShapeType::Sphere => {
                if has_custom {
                    ShapeType::CustomMesh
                } else if has_gif {
                    ShapeType::AnimatedGif
                } else if has_countdown {
                    ShapeType::Countdown
                } else {
                    ShapeType::Torus
                }
            }
            ShapeType::CustomMesh => {
                if has_gif {
                    ShapeType::AnimatedGif
                } else if has_countdown {
                    ShapeType::Countdown
                } else {
                    ShapeType::Torus
                }
            }
            ShapeType::AnimatedGif => {
                if has_countdown {
                    ShapeType::Countdown
                } else {
                    ShapeType::Torus
                }
            }
            ShapeType::Countdown => ShapeType::Torus,
        }
    }
}

/// Rotation mode - controls how shapes rotate
#[derive(Clone, Copy, PartialEq, Default)]
enum RotationMode {
    #[default]
    Horizontal, // Y-axis only (like a turntable)
    Full360,    // Full 3D rotation (X, Y, Z axes)
    Manual,     // No auto-rotation, user controls with IJKL
}

impl RotationMode {
    fn next(self) -> Self {
        match self {
            Self::Horizontal => Self::Full360,
            Self::Full360 => Self::Manual,
            Self::Manual => Self::Horizontal,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Horizontal => "Y-Rot",
            Self::Full360 => "360°",
            Self::Manual => "Manual",
        }
    }

    fn is_auto(self) -> bool {
        !matches!(self, Self::Manual)
    }
}

/// Different transition effect styles
#[derive(Clone, Copy, PartialEq, Default)]
enum TransitionStyle {
    #[default]
    Dissolve,
    FadeOut,
    Coalesce,
}

impl TransitionStyle {
    fn next(self) -> Self {
        match self {
            Self::Dissolve => Self::FadeOut,
            Self::FadeOut => Self::Coalesce,
            Self::Coalesce => Self::Dissolve,
        }
    }

    fn create_effect(self) -> Effect {
        let duration = 800;
        match self {
            Self::Dissolve => fx::dissolve((duration, Interpolation::QuadOut)),
            Self::FadeOut => fx::fade_to_fg(Color::Black, (duration, Interpolation::SineOut)),
            Self::Coalesce => fx::coalesce((duration, Interpolation::CubicOut)),
        }
    }
}

struct App {
    torus: Torus,
    cube: Cube,
    sphere: Sphere,
    custom_mesh: Option<Mesh>,
    custom_mesh_name: String,
    animated_gif: Option<AnimatedGif>,
    animated_gif_name: String,
    countdown: Option<Countdown>,
    renderer: Renderer,
    ascii_buffer: AsciiBuffer,
    current_shape: ShapeType,
    last_frame: Instant,
    transition_effect: Option<Effect>,
    transition_style: TransitionStyle,
    startup_effect: Option<Effect>,
    show_help: bool,
    palette: ColorPalette,
    char_style: CharStyle,
    render_mode: RenderMode,
    pixel_mode: PixelMode,
    dither: bool,
    rotation_mode: RotationMode,
    speed: f32,
    detail: usize,
    zoom: f32,
    frame_count: u64,
    frame_delta: Duration,
}

impl App {
    fn new() -> Self {
        // Create startup effect
        let startup = fx::coalesce((1000, Interpolation::CubicOut));

        Self {
            torus: Torus::default(),
            cube: Cube::default(),
            sphere: Sphere::default(),
            custom_mesh: None,
            custom_mesh_name: String::new(),
            animated_gif: None,
            animated_gif_name: String::new(),
            countdown: None,
            renderer: Renderer::default(),
            ascii_buffer: AsciiBuffer::new(80, 24),
            current_shape: ShapeType::Torus,
            last_frame: Instant::now(),
            transition_effect: None,
            transition_style: TransitionStyle::default(),
            startup_effect: Some(startup),
            show_help: true,
            palette: ColorPalette::default(),
            char_style: CharStyle::default(),
            render_mode: RenderMode::default(),
            pixel_mode: PixelMode::default(),
            dither: false,
            rotation_mode: RotationMode::default(),
            speed: DEFAULT_SPEED,
            detail: DEFAULT_DENSITY,
            zoom: 1.0,
            frame_count: 0,
            frame_delta: FRAME_DURATION,
        }
    }

    fn load_file(&mut self, path: PathBuf) -> Result<(), String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("gif") => {
                let gif = AnimatedGif::from_file(&path).map_err(|e| e.to_string())?;
                let method = if gif.used_chafa { "chafa" } else { "native" };
                self.animated_gif_name =
                    format!("{} ({} frames, {})", gif.name, gif.frame_count(), method);
                self.animated_gif = Some(gif);
                self.current_shape = ShapeType::AnimatedGif;
                Ok(())
            }
            Some("obj") | Some("stl") => {
                let mesh = Mesh::from_file(&path).map_err(|e| e.to_string())?;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Custom Model")
                    .to_string();

                self.custom_mesh_name = format!("{} ({} tris)", name, mesh.triangle_count());
                self.custom_mesh = Some(mesh);
                self.current_shape = ShapeType::CustomMesh;
                self.apply_detail();
                Ok(())
            }
            _ => Err("Unsupported file format. Supported: .obj, .stl, .gif".to_string()),
        }
    }

    fn shape_name(&self) -> String {
        match self.current_shape {
            ShapeType::Torus => "Torus".to_string(),
            ShapeType::Cube => "Cube".to_string(),
            ShapeType::Sphere => "Sphere".to_string(),
            ShapeType::CustomMesh => self.custom_mesh_name.clone(),
            ShapeType::AnimatedGif => self.animated_gif_name.clone(),
            ShapeType::Countdown => {
                if let Some(ref c) = self.countdown {
                    if let Some(ref title) = c.title {
                        format!("Countdown: {}", title)
                    } else {
                        "Countdown".to_string()
                    }
                } else {
                    "Countdown".to_string()
                }
            }
        }
    }

    fn has_custom_mesh(&self) -> bool {
        self.custom_mesh.is_some()
    }

    fn has_gif(&self) -> bool {
        self.animated_gif.is_some()
    }

    fn has_countdown(&self) -> bool {
        self.countdown.is_some()
    }

    fn apply_detail(&mut self) {
        self.torus.set_detail(
            (50.0 * (self.detail as f32 / DEFAULT_DENSITY as f32)) as usize,
            (25.0 * (self.detail as f32 / DEFAULT_DENSITY as f32)) as usize,
        );
        self.sphere.set_detail(
            (40.0 * (self.detail as f32 / DEFAULT_DENSITY as f32)) as usize,
            (20.0 * (self.detail as f32 / DEFAULT_DENSITY as f32)) as usize,
        );
    }

    fn apply_speed(&mut self) {
        self.torus.set_speed_multiplier(self.speed);
        self.cube.set_speed_multiplier(self.speed);
        self.sphere.set_speed_multiplier(self.speed);
        if let Some(ref mut mesh) = self.custom_mesh {
            mesh.set_speed_multiplier(self.speed);
        }
    }

    fn apply_zoom(&mut self) {
        self.renderer.camera.scale = self.zoom;
        self.renderer.camera.distance = 5.0 / self.zoom;
        // Also apply to GIF if loaded
        if let Some(ref mut gif) = self.animated_gif {
            gif.set_scale(self.zoom);
        }
        // Also apply to countdown if loaded
        if let Some(ref mut countdown) = self.countdown {
            countdown.set_scale(self.zoom);
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        self.frame_delta = now - self.last_frame;
        let dt = self.frame_delta.as_secs_f32();
        self.last_frame = now;
        self.frame_count += 1;

        // GIF animation always updates (frame-based, not rotation-based)
        if self.current_shape == ShapeType::AnimatedGif {
            if let Some(ref mut gif) = self.animated_gif {
                gif.update(dt);
            }
            return;
        }

        // Countdown timer always updates
        if self.current_shape == ShapeType::Countdown {
            if let Some(ref mut countdown) = self.countdown {
                countdown.update(dt);
            }
            return;
        }

        if self.rotation_mode.is_auto() {
            self.update_rotation(dt);
        }
    }

    /// Update shape rotation based on rotation mode
    fn update_rotation(&mut self, dt: f32) {
        let y_speed = self.speed * 0.5; // Y-axis rotation speed

        match self.rotation_mode {
            RotationMode::Horizontal => {
                // Y-axis only (turntable style)
                match self.current_shape {
                    ShapeType::Torus => self.torus.rotation.y += y_speed * dt,
                    ShapeType::Cube => self.cube.rotation.y += y_speed * dt,
                    ShapeType::Sphere => self.sphere.rotation.y += y_speed * dt,
                    ShapeType::CustomMesh => {
                        if let Some(ref mut mesh) = self.custom_mesh {
                            mesh.rotation.y += y_speed * dt;
                        }
                    }
                    ShapeType::AnimatedGif => {}
                    ShapeType::Countdown => {}
                }
            }
            RotationMode::Full360 => {
                // Full 3D rotation (original behavior)
                match self.current_shape {
                    ShapeType::Torus => self.torus.update(dt),
                    ShapeType::Cube => self.cube.update(dt),
                    ShapeType::Sphere => self.sphere.update(dt),
                    ShapeType::CustomMesh => {
                        if let Some(ref mut mesh) = self.custom_mesh {
                            mesh.update(dt);
                        }
                    }
                    ShapeType::AnimatedGif => {}
                    ShapeType::Countdown => {}
                }
            }
            RotationMode::Manual => {
                // No auto-rotation in manual mode
            }
        }
    }

    fn reset_rotation(&mut self) {
        self.torus.rotation = Vec3::default();
        self.cube.rotation = Vec3::default();
        self.sphere.rotation = Vec3::default();
        if let Some(ref mut mesh) = self.custom_mesh {
            mesh.rotation = Vec3::default();
        }
    }

    fn manual_rotate(&mut self, dx: f32, dy: f32) {
        match self.current_shape {
            ShapeType::Torus => {
                self.torus.rotation.x += dx;
                self.torus.rotation.y += dy;
            }
            ShapeType::Cube => {
                self.cube.rotation.x += dx;
                self.cube.rotation.y += dy;
            }
            ShapeType::Sphere => {
                self.sphere.rotation.x += dx;
                self.sphere.rotation.y += dy;
            }
            ShapeType::CustomMesh => {
                if let Some(ref mut mesh) = self.custom_mesh {
                    mesh.rotation.x += dx;
                    mesh.rotation.y += dy;
                }
            }
            ShapeType::AnimatedGif => {
                // GIFs don't support rotation
            }
            ShapeType::Countdown => {
                // Countdown doesn't support rotation
            }
        }
    }

    /// Pixel mode for the current shape (the countdown is drawn from text,
    /// so it always uses one pixel per cell)
    fn effective_pixel_mode(&self) -> PixelMode {
        if self.current_shape == ShapeType::Countdown {
            PixelMode::Cell
        } else {
            self.pixel_mode
        }
    }

    fn render_3d(&mut self, area: Rect) {
        let mode = self.effective_pixel_mode();
        let (sx, sy) = mode.subdivisions();
        self.ascii_buffer.resize(area.width.saturating_mul(sx), area.height.saturating_mul(sy));
        self.ascii_buffer.pixel_aspect = mode.pixel_aspect(CHAR_ASPECT);
        self.ascii_buffer.clear();
        self.renderer.mode = self.render_mode;

        match self.current_shape {
            ShapeType::Torus => {
                self.torus
                    .render(&self.renderer, &mut self.ascii_buffer)
            }
            ShapeType::Cube => {
                self.cube
                    .render(&self.renderer, &mut self.ascii_buffer)
            }
            ShapeType::Sphere => {
                self.sphere
                    .render(&self.renderer, &mut self.ascii_buffer)
            }
            ShapeType::CustomMesh => {
                if let Some(ref mesh) = self.custom_mesh {
                    mesh.render(&self.renderer, &mut self.ascii_buffer);
                }
            }
            ShapeType::AnimatedGif => {
                if let Some(ref gif) = self.animated_gif {
                    gif.render(&mut self.ascii_buffer);
                }
            }
            ShapeType::Countdown => {
                if let Some(ref countdown) = self.countdown {
                    countdown.render(&mut self.ascii_buffer);
                }
            }
        }
    }

    fn switch_shape(&mut self) {
        self.current_shape = self.current_shape.next(self.has_custom_mesh(), self.has_gif(), self.has_countdown());
        self.transition_effect = Some(self.transition_style.create_effect());
    }

    fn cycle_transition(&mut self) {
        self.transition_style = self.transition_style.next();
        self.transition_effect = Some(self.transition_style.create_effect());
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char(' ') => {
                // Space toggles pause for countdown, switches shape otherwise
                if self.current_shape == ShapeType::Countdown {
                    if let Some(ref mut countdown) = self.countdown {
                        countdown.toggle_pause();
                    }
                } else {
                    self.switch_shape();
                }
            }
            KeyCode::Tab => self.switch_shape(),
            KeyCode::Char('h') => self.show_help = !self.show_help,

            // Transition style cycling
            KeyCode::Char('t') => self.cycle_transition(),

            // Palette cycling with fade effect
            KeyCode::Char('c') => {
                self.palette = self.palette.next();
                // Quick color flash effect
                self.transition_effect = Some(fx::fade_from_fg(
                    Color::White,
                    (400, Interpolation::QuadOut),
                ));
            }

            // Character style cycling with dissolve
            KeyCode::Char('s') => {
                self.char_style = self.char_style.next();
                self.transition_effect = Some(fx::coalesce((500, Interpolation::CubicOut)));
            }

            // Wireframe toggle
            // Cycle pixel mode: Cell → HalfBlock → Quadrant → ... → Braille
            KeyCode::Char('p') => {
                self.pixel_mode = self.pixel_mode.next();
            }

            // Toggle ordered dithering (Cell mode)
            KeyCode::Char('d') => {
                self.dither = !self.dither;
            }

            KeyCode::Char('w') => {
                self.render_mode = self.render_mode.next();
                self.transition_effect = Some(fx::dissolve((300, Interpolation::QuadOut)));
            }

            // Cycle rotation mode: Y-Rot → 360° → Manual
            KeyCode::Char('m') => {
                self.rotation_mode = self.rotation_mode.next();
                // Reset to default orientation when entering Y-Rot mode
                if self.rotation_mode == RotationMode::Horizontal {
                    self.reset_rotation();
                }
            }

            // Arrow keys: always zoom and speed
            KeyCode::Up => {
                self.zoom = (self.zoom + 0.1).min(3.0);
                self.apply_zoom();
            }
            KeyCode::Down => {
                self.zoom = (self.zoom - 0.1).max(0.3);
                self.apply_zoom();
            }
            KeyCode::Left => {
                self.speed = (self.speed - 0.2).max(MIN_SPEED);
                self.apply_speed();
            }
            KeyCode::Right => {
                self.speed = (self.speed + 0.2).min(MAX_SPEED);
                self.apply_speed();
            }

            // IJKL for manual rotation (always works)
            KeyCode::Char('i') => self.manual_rotate(-0.15, 0.0),
            KeyCode::Char('k') => self.manual_rotate(0.15, 0.0),
            KeyCode::Char('l') => self.manual_rotate(0.0, 0.15),
            KeyCode::Char('j') => self.manual_rotate(0.0, -0.15),

            // Detail level: +/- or =/- or [/]
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => {
                self.detail = (self.detail + 1).min(MAX_DENSITY);
                self.apply_detail();
            }
            KeyCode::Char('-') | KeyCode::Char('[') => {
                self.detail = (self.detail.saturating_sub(1)).max(MIN_DENSITY);
                self.apply_detail();
            }

            // Reset with explosion effect (also resets countdown)
            KeyCode::Char('r') => {
                self.speed = DEFAULT_SPEED;
                self.detail = DEFAULT_DENSITY;
                self.zoom = 1.0;
                self.palette = ColorPalette::default();
                self.char_style = CharStyle::default();
                self.transition_style = TransitionStyle::default();
                self.render_mode = RenderMode::default();
                self.rotation_mode = RotationMode::default();
                self.apply_speed();
                self.apply_detail();
                self.apply_zoom();
                // Reset countdown if present
                if let Some(ref mut countdown) = self.countdown {
                    countdown.reset();
                }
                // Flash effect on reset
                self.transition_effect = Some(fx::parallel(&[
                    fx::fade_from_fg(Color::Cyan, (600, Interpolation::QuadOut)),
                    fx::dissolve((400, Interpolation::CubicOut)),
                ]));
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
    pixel_mode: PixelMode,
    dither: bool,
}

impl<'a> Widget for AsciiWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.buffer
            .draw_pixels(area, buf, self.pixel_mode, self.char_style, self.palette, self.dither);
    }
}

fn print_usage() {
    eprintln!("zoa - 3D ASCII renderer for terminals");
    eprintln!();
    eprintln!("Usage: zoa [OPTIONS] [FILE]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --run <CMD>      Run command and show animation until it completes");
    eprintln!("  --model <FILE>   Use specific 3D model for --run mode");
    eprintln!("  --countdown, -c <TIME>  Start a countdown timer");
    eprintln!("  --timer          Fullscreen timer (10 min default, R to reset)");
    eprintln!("  -h, --help       Show this help");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  FILE    Optional path to OBJ, STL, or GIF file to load (interactive mode)");
    eprintln!();
    eprintln!("Countdown formats:");
    eprintln!("  5m, 1h30m, 90s   Duration format");
    eprintln!("  1:30, 1:30:00    MM:SS or HH:MM:SS format");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  zoa                          # Interactive demo");
    eprintln!("  zoa model.stl                # View a 3D model");
    eprintln!("  zoa --countdown 5m           # 5 minute countdown");
    eprintln!("  zoa -c 1:30                  # 1 minute 30 second countdown");
    eprintln!("  zoa --timer                  # 10 minute timer");
    eprintln!("  zoa --timer -c 15m           # 15 minute timer");
    eprintln!("  zoa --run 'cargo build'      # Show animation while building");
    eprintln!("  zoa --run 'npm install' --model bunny.stl");
    eprintln!();
    eprintln!("Controls (interactive mode):");
    eprintln!("  Space     Pause/resume countdown, or switch shapes");
    eprintln!("  Tab       Switch between shapes");
    eprintln!("  M         Cycle rotation mode (Y-axis → 360° → Manual)");
    eprintln!("  I/J/K/L   Manual rotation");
    eprintln!("  ↑/↓       Zoom in/out");
    eprintln!("  ←/→       Speed down/up");
    eprintln!("  +/-       Detail level");
    eprintln!("  S         Cycle character style");
    eprintln!("  C         Cycle color palette");
    eprintln!("  W         Toggle wireframe/solid mode");
    eprintln!("  T         Cycle transition effect");
    eprintln!("  R         Reset (also resets countdown)");
    eprintln!("  H         Toggle help overlay");
    eprintln!("  Q/Esc     Quit");
    eprintln!();
    eprintln!("Character styles: ASCII, Blocks, Braille, Dense, Minimal, Hatching, Dots, Stars");
    eprintln!("Color palettes: Cyan, Fire, Matrix, Purple, Grayscale, Rainbow");
    eprintln!("Transitions: Dissolve, Fade, Coalesce");
    eprintln!("Render modes: Solid, Wireframe (toggle with W)");
    eprintln!();
    eprintln!("Supported formats: .obj, .stl (ASCII or binary), .gif (animated)");
}

/// Parsed command line arguments
struct Args {
    run_command: Option<String>,
    model_path: Option<PathBuf>,
    countdown: Option<String>,
    timer: bool,
    show_help: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut parsed = Args {
        run_command: None,
        model_path: None,
        countdown: None,
        timer: false,
        show_help: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => parsed.show_help = true,
            "--run" => {
                if i + 1 < args.len() {
                    parsed.run_command = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--model" => {
                if i + 1 < args.len() {
                    parsed.model_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--countdown" | "-c" => {
                if i + 1 < args.len() {
                    parsed.countdown = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--timer" => parsed.timer = true,
            arg if !arg.starts_with('-') => {
                // Positional argument - treat as model path
                if parsed.model_path.is_none() {
                    parsed.model_path = Some(PathBuf::from(arg));
                }
            }
            _ => {}
        }
        i += 1;
    }

    parsed
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = parse_args();

    if args.show_help {
        print_usage();
        return Ok(());
    }

    let mut app = App::new();

    // Load model if specified
    if let Some(path) = &args.model_path {
        if !path.exists() {
            eprintln!("Error: File not found: {}", path.display());
            return Ok(());
        }
        match app.load_file(path.clone()) {
            Ok(()) => {
                if args.run_command.is_none() && args.countdown.is_none() {
                    eprintln!("Loaded: {}", app.shape_name());
                }
            }
            Err(e) => {
                eprintln!("Error loading file: {}", e);
                return Ok(());
            }
        }
    }

    // Load countdown if specified
    if let Some(duration_str) = &args.countdown {
        match Countdown::parse(duration_str) {
            Ok(mut countdown) => {
                countdown.start();
                app.countdown = Some(countdown);
                app.current_shape = ShapeType::Countdown;
                app.char_style = CharStyle::Hatching; // Hatching looks great for countdown
                if args.run_command.is_none() && !args.timer {
                    eprintln!("Countdown: {}", duration_str);
                }
            }
            Err(e) => {
                eprintln!("Error parsing countdown: {}", e);
                eprintln!("Use formats like: 5m, 1h30m, 90s, 1:30, 1:30:00");
                return Ok(());
            }
        }
    }

    // Set up timer mode (10 minute countdown by default, load sample GIF)
    if args.timer {
        // Default to 10 minute countdown if none specified
        if app.countdown.is_none() {
            let mut countdown = Countdown::parse("10m").unwrap();
            countdown.start();
            app.countdown = Some(countdown);
        }
        // Load sample GIF if available
        let gif_path = PathBuf::from("samples/test.gif");
        if gif_path.exists() {
            if let Ok(gif) = AnimatedGif::from_file(&gif_path) {
                app.animated_gif = Some(gif);
            }
        }
        app.char_style = CharStyle::Hatching;
        app.show_help = false;
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let terminal = ratatui::init();

    let result = if args.timer {
        run_timer_mode(terminal, app)
    } else if let Some(cmd) = args.run_command {
        run_monitor(terminal, app, &cmd)
    } else {
        run_interactive(terminal, app)
    };

    ratatui::restore();
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

/// Run in monitor mode - show animation until command completes
fn run_monitor(mut terminal: DefaultTerminal, mut app: App, command: &str) -> Result<()> {
    // Spawn the command
    let mut child: Child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new("sh")
            .args(["-c", command])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }.map_err(|e| color_eyre::eyre::eyre!("Failed to spawn command: {}", e))?;

    // Hide help in monitor mode for cleaner display
    app.show_help = false;

    let start_time = Instant::now();

    loop {
        let frame_start = Instant::now();

        // Check if child process has exited
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished
                let elapsed = start_time.elapsed();
                ratatui::restore();
                disable_raw_mode()?;
                io::stdout().execute(LeaveAlternateScreen)?;

                if status.success() {
                    eprintln!("✓ Command completed successfully in {:.1}s", elapsed.as_secs_f32());
                } else {
                    let code = status.code().unwrap_or(-1);
                    eprintln!("✗ Command failed with exit code {} in {:.1}s", code, elapsed.as_secs_f32());
                }
                return Ok(());
            }
            Ok(None) => {
                // Still running, continue animation
            }
            Err(e) => {
                return Err(color_eyre::eyre::eyre!("Error checking process: {}", e));
            }
        }

        // Allow user to cancel with 'q' or Escape
        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        // Kill the child process
                        let _ = child.kill();
                        let _ = child.wait();
                        ratatui::restore();
                        disable_raw_mode()?;
                        io::stdout().execute(LeaveAlternateScreen)?;
                        eprintln!("Cancelled");
                        return Ok(());
                    }
            }
        }

        app.update();

        terminal.draw(|frame| {
            let area = frame.area();
            app.render_3d(area);

            let widget = AsciiWidget {
                buffer: &app.ascii_buffer,
                palette: app.palette,
                char_style: app.char_style,
                pixel_mode: app.effective_pixel_mode(),
                dither: app.dither,
            };
            frame.render_widget(widget, area);

            // Apply effects
            if let Some(ref mut effect) = app.startup_effect {
                effect.process(app.frame_delta.into(), frame.buffer_mut(), area);
                if effect.done() {
                    app.startup_effect = None;
                }
            }

            if let Some(ref mut effect) = app.transition_effect {
                effect.process(app.frame_delta.into(), frame.buffer_mut(), area);
                if effect.done() {
                    app.transition_effect = None;
                }
            }

            // Show minimal status
            draw_monitor_status(frame, command, start_time.elapsed());
        })?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }
}

/// Draw minimal status bar for monitor mode
fn draw_monitor_status(frame: &mut Frame, command: &str, elapsed: Duration) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Paragraph;

    let area = frame.area();
    let status = format!(" {} | {:.1}s | [Q] Cancel ", command, elapsed.as_secs_f32());
    let widget = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    if area.height > 0 {
        frame.render_widget(widget, Rect::new(area.x, area.y + area.height - 1, area.width, 1));
    }
}

/// Run in interactive mode with full controls
fn run_interactive(mut terminal: DefaultTerminal, mut app: App) -> Result<()> {
    loop {
        let frame_start = Instant::now();

        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && app.handle_key(key.code) {
                        break;
                    }
            }
        }

        app.update();

        terminal.draw(|frame| {
            let area = frame.area();

            app.render_3d(area);

            let widget = AsciiWidget {
                buffer: &app.ascii_buffer,
                palette: app.palette,
                char_style: app.char_style,
                pixel_mode: app.effective_pixel_mode(),
                dither: app.dither,
            };
            frame.render_widget(widget, area);

            // Apply startup effect first
            if let Some(ref mut effect) = app.startup_effect {
                effect.process(app.frame_delta.into(), frame.buffer_mut(), area);

                if effect.done() {
                    app.startup_effect = None;
                }
            }

            // Then apply transition effects
            if let Some(ref mut effect) = app.transition_effect {
                effect.process(app.frame_delta.into(), frame.buffer_mut(), area);

                if effect.done() {
                    app.transition_effect = None;
                }
            }

            if app.show_help {
                draw_help(frame, &app);
            }
        })?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}

/// Run in timer mode - large countdown with small GIF, minimal UI
fn run_timer_mode(mut terminal: DefaultTerminal, mut app: App) -> Result<()> {
    use ratatui::layout::{Constraint, Direction, Layout};

    // Secondary buffer for GIF
    let mut gif_buffer = AsciiBuffer::new(30, 10);

    loop {
        let frame_start = Instant::now();

        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('r') => {
                            // Reset countdown
                            if let Some(ref mut countdown) = app.countdown {
                                countdown.reset();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update countdown
        let now = Instant::now();
        let dt = now.duration_since(app.last_frame).as_secs_f32();
        app.last_frame = now;
        app.frame_delta = Duration::from_secs_f32(dt);

        if let Some(ref mut countdown) = app.countdown {
            countdown.update(dt);
        }
        if let Some(ref mut gif) = app.animated_gif {
            gif.update(dt);
        }

        // Check if countdown is finished for color change
        let is_finished = app.countdown.as_ref().map(|c| c.is_finished()).unwrap_or(false);
        let palette = if is_finished { ColorPalette::Fire } else { app.palette };

        terminal.draw(|frame| {
            let area = frame.area();

            // Layout: main countdown area with small GIF in corner
            let has_gif = app.animated_gif.is_some();

            if has_gif && area.width > 40 && area.height > 15 {
                // Split layout: countdown on left/center, GIF on bottom-right
                let main_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(20),
                        Constraint::Length(32),
                    ])
                    .split(area);

                let countdown_area = main_layout[0];
                let gif_column = main_layout[1];

                // GIF at bottom of right column
                let gif_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),
                        Constraint::Length(12),
                    ])
                    .split(gif_column);

                let gif_area = gif_layout[1];

                // Render countdown
                app.ascii_buffer.resize(countdown_area.width, countdown_area.height);
                app.ascii_buffer.clear();
                if let Some(ref countdown) = app.countdown {
                    countdown.render(&mut app.ascii_buffer);
                }

                let countdown_widget = AsciiWidget {
                    buffer: &app.ascii_buffer,
                    palette,
                    char_style: app.char_style,
                    pixel_mode: PixelMode::Cell,
                    dither: false,
                };
                frame.render_widget(countdown_widget, countdown_area);

                // Render GIF
                gif_buffer.resize(gif_area.width, gif_area.height);
                gif_buffer.clear();
                if let Some(ref gif) = app.animated_gif {
                    gif.render(&mut gif_buffer);
                }

                let gif_widget = AsciiWidget {
                    buffer: &gif_buffer,
                    palette: ColorPalette::Cyan,
                    char_style: CharStyle::Braille,
                    pixel_mode: PixelMode::Cell,
                    dither: false,
                };
                frame.render_widget(gif_widget, gif_area);
            } else {
                // No GIF or too small - just countdown fullscreen
                app.ascii_buffer.resize(area.width, area.height);
                app.ascii_buffer.clear();
                if let Some(ref countdown) = app.countdown {
                    countdown.render(&mut app.ascii_buffer);
                }

                let widget = AsciiWidget {
                    buffer: &app.ascii_buffer,
                    palette,
                    char_style: app.char_style,
                    pixel_mode: PixelMode::Cell,
                    dither: false,
                };
                frame.render_widget(widget, area);
            }

            // Apply effects
            if let Some(ref mut effect) = app.startup_effect {
                effect.process(app.frame_delta.into(), frame.buffer_mut(), area);
                if effect.done() {
                    app.startup_effect = None;
                }
            }
        })?;

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    Ok(())
}

fn draw_help(frame: &mut Frame, app: &App) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Paragraph;

    let area = frame.area();

    // Title bar - simplified
    let title = format!(
        " {} | {} | {} | {} | {} | {} ",
        app.shape_name(),
        app.effective_pixel_mode().name(),
        app.char_style.name(),
        app.palette.name(),
        app.render_mode.name(),
        app.rotation_mode.name(),
    );
    let title_widget = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    if area.height > 0 {
        frame.render_widget(title_widget, Rect::new(area.x, area.y, area.width, 1));
    }

    // Help text - compact
    let help = " [Space] Shape | [P]ixels | [S]tyle | [D]ither | [C]olor | [W]ire | [M]ode | [IJKL] Rotate | [↑↓] Zoom | [←→] Speed | [+/-] Detail | [R]eset | [H]ide | [Q]uit ";
    let help_widget = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    if area.height > 1 {
        frame.render_widget(
            help_widget,
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );
    }
}
