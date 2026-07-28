# Connectivity

Tauri-independent network connectivity detection for desktop Rust applications.

The crate provides synchronous APIs for Windows, Linux, and macOS:

   * `connection_status()` returns reachability, metered/constrained flags, and
     the primary connection type.
   * `supported_connection_types()` returns the transport classes reported by
     the current platform backend.

## Installation

```toml
[dependencies]
connectivity = { git = "https://github.com/silvermine/tauri-plugin-connectivity" }
```

## Usage

```rust,no_run
fn main() -> connectivity::Result<()> {
   let status = connectivity::connection_status()?;

   if status.connected && !status.metered && !status.constrained {
      println!("network is suitable for unrestricted work");
   }

   let supported_types = connectivity::supported_connection_types()?;
   println!("supported transports: {supported_types:?}");

   Ok(())
}
```

Both detection functions are synchronous because they call platform APIs
directly. Applications using an async runtime should run them on a blocking
worker thread.

Android and iOS detection remain part of the Tauri plugin's native mobile
bridge and are not implemented by this crate.

## License

MIT
