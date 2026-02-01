# zoa

A 3D ASCII renderer for terminals, built on [ratatui](https://github.com/ratatui/ratatui).

## Features

- **3D Shapes** - Torus, cube, sphere, and custom OBJ/STL mesh support
- **Particle Systems** - Fire, rain, snow, sparks, and more
- **SDF Ray Marching** - Metaballs, fractals, and procedural geometry
- **Animated GIFs** - Render GIFs as ASCII art
- **Countdown Timer** - Large ASCII digit display
- **Multiple Styles** - ASCII, blocks, braille, hatching characters
- **Color Palettes** - Cyan, fire, matrix, purple, rainbow, grayscale

## Installation

```toml
[dependencies]
zoa = "0.1"
```

## Quick Start

```rust
use zoa::ZoaWidget;
use ratatui::Frame;

let mut widget = ZoaWidget::default();

// In your render loop:
fn draw(frame: &mut Frame, widget: &mut ZoaWidget) {
    widget.update(0.016); // delta time in seconds
    frame.render_widget(widget, frame.area());
}
```

## Loading Content

```rust
// 3D mesh from OBJ file
widget.load_mesh(Path::new("model.obj"))?;

// Animated GIF
widget.load_gif(Path::new("animation.gif"))?;

// Countdown timer
widget.start_countdown_from_str("5m")?;   // 5 minutes
widget.start_countdown_from_str("1:30")?; // 1 min 30 sec
```

## Configuration

```rust
use zoa::{ZoaWidget, ZoaConfig, Shape, CharStyle, ColorPalette};

let config = ZoaConfig {
    shape: Shape::Torus,
    char_style: CharStyle::Braille,
    palette: ColorPalette::Fire,
    zoom: 1.5,
    speed: 1.0,
    ..Default::default()
};

let mut widget = ZoaWidget::new(config);
```

## CLI

```bash
# 3D shapes
zoa                      # default torus
zoa --shape cube
zoa --shape sphere

# Files
zoa model.obj
zoa animation.gif

# Countdown timer
zoa --countdown 5m
zoa -c 1:30:00

# Fullscreen timer
zoa --timer
```

### Controls

| Key | Action |
|-----|--------|
| `s` | Cycle shapes |
| `c` | Cycle character styles |
| `p` | Cycle color palettes |
| `m` | Toggle wireframe/solid |
| `+`/`-` | Zoom in/out |
| `[`/`]` | Adjust speed |
| `Space` | Pause |
| `r` | Reset |
| `q` | Quit |

## Examples

```bash
cargo run --example showcase        # Feature overview
cargo run --example particles_demo  # Particle effects
cargo run --example sdf_demo        # SDF ray marching
cargo run --example storm_demo      # Interactive storm cloud
```

## License

Apache-2.0
