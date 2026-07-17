use crate::error::{Error, Result};
#[cfg(not(target_os = "macos"))]
use crate::types::ConnectionStatus;
use crate::types::ConnectionType;
use tracing::warn;

/// Returns [`Error::Unsupported`] until a platform-specific implementation is added.
#[cfg(not(target_os = "macos"))]
pub fn connection_status() -> Result<ConnectionStatus> {
   warn!("connection status detection is not supported on this platform");
   Err(Error::Unsupported)
}

/// Returns [`Error::SupportedConnectionTypesUnsupported`] until a
/// platform-specific implementation is added.
pub fn supported_connection_types() -> Result<Vec<ConnectionType>> {
   warn!("supported connection type detection is not supported on this platform");
   Err(Error::SupportedConnectionTypesUnsupported)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   #[cfg(not(target_os = "macos"))]
   fn connection_status_returns_unsupported() {
      assert!(matches!(connection_status(), Err(Error::Unsupported)));
   }

   #[test]
   fn supported_connection_types_returns_unsupported() {
      assert!(matches!(
         supported_connection_types(),
         Err(Error::SupportedConnectionTypesUnsupported)
      ));
   }
}
