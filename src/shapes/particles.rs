use crate::renderer::AsciiBuffer;

/// A single particle with position, velocity, and lifetime
#[derive(Clone)]
struct Particle {
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    life: f32,     // 0.0 to 1.0, dies at 0
    max_life: f32, // starting life
}

impl Particle {
    fn new(x: f32, y: f32, z: f32, vx: f32, vy: f32, vz: f32, life: f32) -> Self {
        Self {
            x, y, z, vx, vy, vz,
            life,
            max_life: life,
        }
    }

    fn update(&mut self, dt: f32, gravity: f32, drag: f32) {
        self.vy -= gravity * dt;
        self.vx *= 1.0 - drag * dt;
        self.vy *= 1.0 - drag * dt;
        self.vz *= 1.0 - drag * dt;

        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.z += self.vz * dt;

        self.life -= dt;
    }

    fn alive(&self) -> bool {
        self.life > 0.0
    }

    fn intensity(&self) -> f32 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

/// Emitter shape for spawning particles
#[derive(Clone, Copy)]
pub enum EmitterShape {
    Point,
    Line { length: f32 },
    Circle { radius: f32 },
    Sphere { radius: f32 },
}

/// Preset particle effects
#[derive(Clone, Copy, Default)]
pub enum ParticlePreset {
    #[default]
    Fire,
    Smoke,
    Rain,
    Snow,
    Sparks,
    Fountain,
    Explosion,
    Matrix,
}

impl ParticlePreset {
    pub fn next(self) -> Self {
        match self {
            Self::Fire => Self::Smoke,
            Self::Smoke => Self::Rain,
            Self::Rain => Self::Snow,
            Self::Snow => Self::Sparks,
            Self::Sparks => Self::Fountain,
            Self::Fountain => Self::Explosion,
            Self::Explosion => Self::Matrix,
            Self::Matrix => Self::Fire,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Fire => "Fire",
            Self::Smoke => "Smoke",
            Self::Rain => "Rain",
            Self::Snow => "Snow",
            Self::Sparks => "Sparks",
            Self::Fountain => "Fountain",
            Self::Explosion => "Explosion",
            Self::Matrix => "Matrix",
        }
    }
}

/// Particle system with emitter and physics
pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,

    // Emitter properties
    emitter_x: f32,
    emitter_y: f32,
    emitter_z: f32,
    emitter_shape: EmitterShape,
    spawn_rate: f32, // particles per second
    spawn_accum: f32,

    // Particle properties
    speed_min: f32,
    speed_max: f32,
    life_min: f32,
    life_max: f32,
    spread: f32,      // cone angle in radians
    direction: (f32, f32, f32), // normalized emission direction

    // Physics
    gravity: f32,
    drag: f32,

    // State
    preset: ParticlePreset,
    time: f32,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        let mut sys = Self {
            particles: Vec::with_capacity(1000),
            max_particles: 1000,
            emitter_x: 0.0,
            emitter_y: -2.0,
            emitter_z: 0.0,
            emitter_shape: EmitterShape::Point,
            spawn_rate: 50.0,
            spawn_accum: 0.0,
            speed_min: 2.0,
            speed_max: 4.0,
            life_min: 1.0,
            life_max: 2.0,
            spread: 0.3,
            direction: (0.0, 1.0, 0.0),
            gravity: 0.0,
            drag: 0.0,
            preset: ParticlePreset::Fire,
            time: 0.0,
        };
        sys.apply_preset(ParticlePreset::Fire);
        sys
    }
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_preset(preset: ParticlePreset) -> Self {
        let mut sys = Self::default();
        sys.apply_preset(preset);
        sys
    }

    pub fn set_preset(&mut self, preset: ParticlePreset) {
        self.apply_preset(preset);
    }

    pub fn preset(&self) -> ParticlePreset {
        self.preset
    }

    pub fn cycle_preset(&mut self) {
        self.apply_preset(self.preset.next());
    }

    fn apply_preset(&mut self, preset: ParticlePreset) {
        self.preset = preset;
        self.particles.clear();

        match preset {
            ParticlePreset::Fire => {
                self.emitter_y = -2.5;
                self.emitter_shape = EmitterShape::Line { length: 2.0 };
                self.spawn_rate = 80.0;
                self.speed_min = 1.5;
                self.speed_max = 3.0;
                self.life_min = 0.8;
                self.life_max = 1.5;
                self.spread = 0.4;
                self.direction = (0.0, 1.0, 0.0);
                self.gravity = -0.5; // negative = upward
                self.drag = 1.0;
            }
            ParticlePreset::Smoke => {
                self.emitter_y = -2.5;
                self.emitter_shape = EmitterShape::Circle { radius: 0.5 };
                self.spawn_rate = 30.0;
                self.speed_min = 0.5;
                self.speed_max = 1.5;
                self.life_min = 2.0;
                self.life_max = 4.0;
                self.spread = 0.6;
                self.direction = (0.0, 1.0, 0.0);
                self.gravity = -0.2;
                self.drag = 0.5;
            }
            ParticlePreset::Rain => {
                self.emitter_y = 3.0;
                self.emitter_shape = EmitterShape::Line { length: 8.0 };
                self.spawn_rate = 150.0;
                self.speed_min = 8.0;
                self.speed_max = 12.0;
                self.life_min = 0.8;
                self.life_max = 1.2;
                self.spread = 0.05;
                self.direction = (0.0, -1.0, 0.0);
                self.gravity = 5.0;
                self.drag = 0.0;
            }
            ParticlePreset::Snow => {
                self.emitter_y = 3.0;
                self.emitter_shape = EmitterShape::Line { length: 8.0 };
                self.spawn_rate = 40.0;
                self.speed_min = 0.5;
                self.speed_max = 1.5;
                self.life_min = 3.0;
                self.life_max = 5.0;
                self.spread = 0.8;
                self.direction = (0.0, -1.0, 0.0);
                self.gravity = 0.3;
                self.drag = 2.0;
            }
            ParticlePreset::Sparks => {
                self.emitter_y = 0.0;
                self.emitter_shape = EmitterShape::Point;
                self.spawn_rate = 100.0;
                self.speed_min = 3.0;
                self.speed_max = 6.0;
                self.life_min = 0.3;
                self.life_max = 0.8;
                self.spread = std::f32::consts::PI; // full sphere
                self.direction = (0.0, 1.0, 0.0);
                self.gravity = 3.0;
                self.drag = 0.5;
            }
            ParticlePreset::Fountain => {
                self.emitter_y = -2.5;
                self.emitter_shape = EmitterShape::Point;
                self.spawn_rate = 60.0;
                self.speed_min = 4.0;
                self.speed_max = 5.0;
                self.life_min = 1.5;
                self.life_max = 2.5;
                self.spread = 0.3;
                self.direction = (0.0, 1.0, 0.0);
                self.gravity = 4.0;
                self.drag = 0.1;
            }
            ParticlePreset::Explosion => {
                self.emitter_y = 0.0;
                self.emitter_shape = EmitterShape::Sphere { radius: 0.2 };
                self.spawn_rate = 500.0; // burst
                self.speed_min = 2.0;
                self.speed_max = 5.0;
                self.life_min = 0.5;
                self.life_max = 1.5;
                self.spread = std::f32::consts::PI;
                self.direction = (0.0, 1.0, 0.0);
                self.gravity = 1.0;
                self.drag = 2.0;
            }
            ParticlePreset::Matrix => {
                self.emitter_y = 3.0;
                self.emitter_shape = EmitterShape::Line { length: 10.0 };
                self.spawn_rate = 25.0;
                self.speed_min = 2.0;
                self.speed_max = 4.0;
                self.life_min = 2.0;
                self.life_max = 4.0;
                self.spread = 0.0;
                self.direction = (0.0, -1.0, 0.0);
                self.gravity = 0.0;
                self.drag = 0.0;
            }
        }
    }

    /// Trigger a burst of particles (for explosion-like effects)
    pub fn burst(&mut self, count: usize) {
        for _ in 0..count {
            if self.particles.len() < self.max_particles {
                self.spawn_particle();
            }
        }
    }

    fn spawn_particle(&mut self) {
        let (mut x, mut y, mut z) = (self.emitter_x, self.emitter_y, self.emitter_z);

        // Offset based on emitter shape
        match self.emitter_shape {
            EmitterShape::Point => {}
            EmitterShape::Line { length } => {
                x += (fastrand() - 0.5) * length;
            }
            EmitterShape::Circle { radius } => {
                let angle = fastrand() * std::f32::consts::TAU;
                let r = fastrand().sqrt() * radius;
                x += angle.cos() * r;
                z += angle.sin() * r;
            }
            EmitterShape::Sphere { radius } => {
                let theta = fastrand() * std::f32::consts::TAU;
                let phi = (fastrand() * 2.0 - 1.0).acos();
                let r = fastrand().cbrt() * radius;
                x += phi.sin() * theta.cos() * r;
                y += phi.cos() * r;
                z += phi.sin() * theta.sin() * r;
            }
        }

        // Calculate velocity with spread
        let speed = self.speed_min + fastrand() * (self.speed_max - self.speed_min);
        let (dx, dy, dz) = self.direction;

        // Add random spread
        let spread_x = (fastrand() - 0.5) * self.spread;
        let spread_z = (fastrand() - 0.5) * self.spread;

        let vx = dx * speed + spread_x * speed;
        let vy = dy * speed;
        let vz = dz * speed + spread_z * speed;

        let life = self.life_min + fastrand() * (self.life_max - self.life_min);

        self.particles.push(Particle::new(x, y, z, vx, vy, vz, life));
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        // Spawn new particles
        self.spawn_accum += self.spawn_rate * dt;
        while self.spawn_accum >= 1.0 && self.particles.len() < self.max_particles {
            self.spawn_particle();
            self.spawn_accum -= 1.0;
        }

        // Update existing particles
        for particle in &mut self.particles {
            particle.update(dt, self.gravity, self.drag);
        }

        // Remove dead particles
        self.particles.retain(|p| p.alive());
    }

    pub fn render(&self, buffer: &mut AsciiBuffer) {
        let buf_width = buffer.width as f32;
        let buf_height = buffer.height as f32;

        if buf_width < 1.0 || buf_height < 1.0 {
            return;
        }

        let scale = buf_height.min(buf_width) * 0.1;
        let center_x = buf_width / 2.0;
        let center_y = buf_height / 2.0;

        for particle in &self.particles {
            // Simple orthographic projection
            let screen_x = center_x + particle.x * scale;
            let screen_y = center_y - particle.y * scale; // flip Y

            if screen_x >= 0.0 && screen_x < buf_width && screen_y >= 0.0 && screen_y < buf_height {
                let depth = 1.0 / (5.0 + particle.z); // pseudo-depth
                let luminance = particle.intensity();
                buffer.plot(screen_x as u16, screen_y as u16, depth, luminance);
            }
        }
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

/// Simple fast random (xorshift)
fn fastrand() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = Cell::new(0xDEADBEEF);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_lifecycle() {
        let mut sys = ParticleSystem::new();
        assert_eq!(sys.particle_count(), 0);

        // Update should spawn particles
        sys.update(0.1);
        assert!(sys.particle_count() > 0);

        // Particles should eventually die
        for _ in 0..100 {
            sys.update(0.1);
        }
    }

    #[test]
    fn test_presets() {
        for preset in [
            ParticlePreset::Fire,
            ParticlePreset::Smoke,
            ParticlePreset::Rain,
            ParticlePreset::Snow,
            ParticlePreset::Sparks,
            ParticlePreset::Fountain,
            ParticlePreset::Explosion,
            ParticlePreset::Matrix,
        ] {
            let mut sys = ParticleSystem::with_preset(preset);
            sys.update(0.1);
            // Should not crash
        }
    }

    #[test]
    fn test_burst() {
        let mut sys = ParticleSystem::new();
        sys.burst(100);
        assert!(sys.particle_count() >= 100);
    }
}
