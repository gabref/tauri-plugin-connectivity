use serde::{Serialize, ser::Serializer};

/// Errors that can occur when detecting connection status.
#[derive(Debug, thiserror::Error)]
pub enum Error {
   /// The current platform does not support connection status detection.
   #[error("connection status detection is not supported on this platform")]
   Unsupported,

   /// The platform-specific backend failed while detecting connection status.
   #[error("connection status detection failed: {0}")]
   DetectionFailed(String),
}

impl Serialize for Error {
   fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
   where
      S: Serializer,
   {
      serializer.serialize_str(self.to_string().as_ref())
   }
}

/// A specialized [`Result`] type for connectivity operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for Error {
   fn from(value: windows::core::Error) -> Self {
      Self::DetectionFailed(value.to_string())
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn unsupported_error_displays_message() {
      let err = Error::Unsupported;

      assert_eq!(
         err.to_string(),
         "connection status detection is not supported on this platform"
      );
   }

   #[test]
   fn error_serializes_to_string() {
      let err = Error::Unsupported;
      let json = serde_json::to_value(&err).unwrap();

      assert_eq!(
         json,
         "connection status detection is not supported on this platform"
      );
   }

   #[test]
   fn detection_failed_error_displays_message() {
      let err = Error::DetectionFailed(String::from("backend unavailable"));

      assert_eq!(
         err.to_string(),
         "connection status detection failed: backend unavailable"
      );
   }
}
