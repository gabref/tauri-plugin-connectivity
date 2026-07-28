//! Rust-native desktop connectivity backends.
//!
//! Mobile platforms are intentionally outside this crate's detection scope.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::connection_status;
#[cfg(target_os = "linux")]
pub use linux::supported_connection_types;
#[cfg(target_os = "macos")]
pub use macos::connection_status;
#[cfg(target_os = "macos")]
pub use macos::supported_connection_types;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use unsupported::connection_status;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use unsupported::supported_connection_types;
#[cfg(target_os = "windows")]
pub use windows::connection_status;
#[cfg(target_os = "windows")]
pub use windows::supported_connection_types;
