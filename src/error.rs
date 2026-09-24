//! Error type for loading content and parsing input.

use std::fmt;

/// Errors from loading meshes and GIFs or parsing countdown durations.
#[derive(Debug)]
pub enum Error {
    /// Reading a file failed
    Io(std::io::Error),
    /// The file extension isn't one zoa can load
    UnsupportedFormat(String),
    /// The file was read but its contents are invalid (or contain nothing to draw)
    InvalidData(String),
    /// A countdown duration string couldn't be parsed
    InvalidDuration(String),
    /// Decoding a GIF failed
    #[cfg(feature = "gif")]
    Image(image::ImageError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::UnsupportedFormat(ext) => write!(f, "unsupported file format: .{ext}"),
            Self::InvalidData(msg) => write!(f, "invalid file: {msg}"),
            Self::InvalidDuration(msg) => write!(f, "invalid duration: {msg}"),
            #[cfg(feature = "gif")]
            Self::Image(e) => write!(f, "failed to decode image: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            #[cfg(feature = "gif")]
            Self::Image(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(feature = "gif")]
impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Self::Image(e)
    }
}
