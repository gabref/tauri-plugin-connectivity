use tauri::{AppHandle, Runtime, command};
use tracing::{debug, warn};

use crate::ConnectivityExt;
use crate::error::Result;
use crate::types::ConnectionStatus;

/// Returns the current network connection status.
///
/// On platforms without an implementation, this returns [`Error::Unsupported`].
#[command]
pub(crate) async fn connection_status<R: Runtime>(app: AppHandle<R>) -> Result<ConnectionStatus> {
   debug!("received frontend request for connection_status");

   match app.connectivity().connection_status() {
      Ok(status) => {
         debug!(?status, "returning connection status to frontend");
         Ok(status)
      }
      Err(error) => {
         warn!(%error, "failed to resolve connection status");
         Err(error)
      }
   }
}
