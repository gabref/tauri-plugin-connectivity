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
pub use platform::{connection_status, supported_connection_types};
pub use types::{ConnectionStatus, ConnectionType};
