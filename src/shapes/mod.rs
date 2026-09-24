pub mod countdown;
pub mod cube;
#[cfg(feature = "gif")]
pub mod gif;
pub mod mesh;
pub mod particles;
pub mod sdf;
pub mod sphere;
pub mod torus;

pub use countdown::{Countdown, CountdownState};
pub use cube::Cube;
#[cfg(feature = "gif")]
pub use gif::AnimatedGif;
pub use mesh::{Mesh, Triangle};
pub use particles::{ParticlePreset, ParticleSystem};
pub use sdf::{SdfPreset, SdfScene};
pub use sphere::Sphere;
pub use torus::Torus;

use std::time::Duration;

/// Convert a frame delta to a `Duration`, treating negative, NaN, or
/// out-of-range values as zero instead of panicking.
pub(crate) fn dt_to_duration(dt: f32) -> Duration {
    Duration::try_from_secs_f32(dt).unwrap_or(Duration::ZERO)
}
