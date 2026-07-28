//! Tauri-independent network connectivity detection for desktop platforms.
//!
//! The crate exposes the same synchronous detection APIs on Windows, Linux,
//! and macOS. Callers running inside an async runtime should invoke them on a
//! blocking worker thread.
//!
//! ```no_run
//! let status = connectivity::connection_status()?;
//! println!("connected: {}", status.connected);
//!
//! let supported_types = connectivity::supported_connection_types()?;
//! println!("supported transports: {supported_types:?}");
//! # Ok::<(), connectivity::Error>(())
//! ```

mod error;
mod platform;
mod types;

pub use error::{Error, Result};
pub use types::{ConnectionStatus, ConnectionType};

/// Returns the current network connection status reported by the platform.
///
/// Windows and Linux query their platform backends when this function is called.
/// On macOS, the result comes from the latest asynchronous path-monitor update;
/// until the first update arrives, it reports a disconnected state.
///
/// # Errors
///
/// Returns [`Error::Unsupported`] when the target has no connectivity backend,
/// or [`Error::DetectionFailed`] when the platform cannot determine the status.
pub fn connection_status() -> Result<ConnectionStatus> {
   platform::connection_status()
}

/// Returns the connection transport classes reported by the platform.
///
/// Windows and Linux enumerate present adapters or devices, while macOS reports
/// only interfaces on the current satisfied network path. The result is
/// deduplicated, excludes [`ConnectionType::Unknown`], and can be a best-effort
/// partial inventory when at least one transport was recovered.
///
/// # Errors
///
/// Returns [`Error::SupportedConnectionTypesUnsupported`] when the target has no
/// inventory backend, or [`Error::SupportedConnectionTypesDetectionFailed`] when
/// no transport can be recovered after a platform failure.
pub fn supported_connection_types() -> Result<Vec<ConnectionType>> {
   platform::supported_connection_types()
}
