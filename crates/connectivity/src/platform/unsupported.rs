use crate::error::{Error, Result};
use crate::types::{ConnectionStatus, ConnectionType};
use tracing::warn;

/// Returns [`Error::Unsupported`] until a platform-specific implementation is added.
pub(crate) fn connection_status() -> Result<ConnectionStatus> {
   warn!("connection status detection is not supported on this platform");
   Err(Error::Unsupported)
}

/// Returns [`Error::SupportedConnectionTypesUnsupported`] until a
/// platform-specific implementation is added.
pub(crate) fn supported_connection_types() -> Result<Vec<ConnectionType>> {
   warn!("supported connection type detection is not supported on this platform");
   Err(Error::SupportedConnectionTypesUnsupported)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
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
