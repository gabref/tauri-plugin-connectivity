use serde::Serialize;
use tauri::{AppHandle, Runtime, command};
use tracing::{debug, warn};

#[cfg(mobile)]
use crate::ConnectivityExt;
#[cfg(desktop)]
use crate::Error;
use crate::{ConnectionStatus, ConnectionType, Result};

/// Frontend compatibility response. The Rust API exposes unknown policy flags
/// as `None`, while the existing JavaScript API keeps its boolean fallback.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrontendConnectionStatus {
   connected: bool,
   metered: bool,
   constrained: bool,
   connection_type: ConnectionType,
}

impl From<ConnectionStatus> for FrontendConnectionStatus {
   fn from(status: ConnectionStatus) -> Self {
      Self {
         connected: status.connected,
         metered: status.metered.unwrap_or(false),
         constrained: status.constrained.unwrap_or(false),
         connection_type: status.connection_type,
      }
   }
}

/// Returns the current network connection status.
///
/// On platforms without an implementation, this returns [`crate::Error::Unsupported`].
#[command]
pub(crate) async fn connection_status<R: Runtime>(
   _app: AppHandle<R>,
) -> Result<FrontendConnectionStatus> {
   debug!("received frontend request for connection_status");

   #[cfg(mobile)]
   let result = _app.connectivity().connection_status();

   #[cfg(desktop)]
   let result = tauri::async_runtime::spawn_blocking(connectivity::connection_status)
      .await
      .map_err(|error| Error::DetectionFailed {
         message: format!("connection status worker failed: {error}"),
         code: None,
      })?;

   match result {
      Ok(status) => {
         debug!(?status, "returning connection status to frontend");
         Ok(status.into())
      }
      Err(error) => {
         warn!(%error, "failed to resolve connection status");
         Err(error)
      }
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn frontend_status_preserves_known_policy_flags() {
      let status = FrontendConnectionStatus::from(ConnectionStatus {
         connected: true,
         metered: Some(true),
         constrained: Some(false),
         connection_type: ConnectionType::Cellular,
      });

      assert!(status.connected);
      assert!(status.metered);
      assert!(!status.constrained);
      assert_eq!(status.connection_type, ConnectionType::Cellular);
   }

   #[test]
   fn frontend_status_maps_unknown_policy_flags_to_false() {
      let status = FrontendConnectionStatus::from(ConnectionStatus {
         connected: true,
         metered: None,
         constrained: None,
         connection_type: ConnectionType::Ethernet,
      });

      assert!(status.connected);
      assert!(!status.metered);
      assert!(!status.constrained);
      assert_eq!(status.connection_type, ConnectionType::Ethernet);
   }
}

/// Returns the connection transport classes reported by the platform backend.
///
/// An empty vector means detection succeeded but found no supported transports.
/// The meaning is platform-specific: Apple backends report interfaces available
/// to the current satisfied path, while other backends can report present
/// hardware or currently visible networks.
/// Backends that recover at least one transport can return a best-effort partial
/// result when another interface cannot be inspected. Detection failures that
/// prevent recovery return [`crate::Error::SupportedConnectionTypesDetectionFailed`].
/// On platforms without an implementation, this returns
/// [`crate::Error::SupportedConnectionTypesUnsupported`].
#[command]
pub(crate) async fn supported_connection_types<R: Runtime>(
   _app: AppHandle<R>,
) -> Result<Vec<ConnectionType>> {
   debug!("received frontend request for supported_connection_types");

   #[cfg(mobile)]
   let result = _app.connectivity().supported_connection_types();

   #[cfg(desktop)]
   let result = tauri::async_runtime::spawn_blocking(connectivity::supported_connection_types)
      .await
      .map_err(|error| Error::SupportedConnectionTypesDetectionFailed {
         message: format!("supported connection types worker failed: {error}"),
         code: None,
      })?;

   match result {
      Ok(connection_types) => {
         debug!(
            ?connection_types,
            "returning supported connection types to frontend"
         );
         Ok(connection_types)
      }
      Err(error) => {
         warn!(%error, "failed to resolve supported connection types");
         Err(error)
      }
   }
}
