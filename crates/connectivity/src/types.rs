use serde::{Deserialize, Serialize};

/// Describes the physical or logical transport used to connect to the network.
///
/// When multiple interfaces are active simultaneously (e.g. WiFi + Cellular),
/// this represents the preferred/primary transport as determined by the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionType {
   /// Connected via Wi-Fi.
   Wifi,

   /// Connected via Ethernet (wired).
   Ethernet,

   /// Connected via a cellular network (e.g. LTE, 5G).
   Cellular,

   /// The connection type could not be determined.
   Unknown,
}

/// Internal helper used by the Linux, macOS, and Windows backends to deduplicate
/// supported transport classes in stable API order. Android and iOS construct
/// this list in native Kotlin and Swift instead.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows", test))]
#[derive(Debug, Default)]
pub(crate) struct ConnectionTypes {
   wifi: bool,
   ethernet: bool,
   cellular: bool,
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows", test))]
impl ConnectionTypes {
   pub(crate) fn new() -> Self {
      Self::default()
   }

   pub(crate) fn insert(&mut self, connection_type: ConnectionType) {
      match connection_type {
         ConnectionType::Wifi => self.wifi = true,
         ConnectionType::Ethernet => self.ethernet = true,
         ConnectionType::Cellular => self.cellular = true,
         ConnectionType::Unknown => {}
      }
   }

   pub(crate) fn into_vec(self) -> Vec<ConnectionType> {
      let mut connection_types = Vec::with_capacity(3);

      if self.wifi {
         connection_types.push(ConnectionType::Wifi);
      }
      if self.ethernet {
         connection_types.push(ConnectionType::Ethernet);
      }
      if self.cellular {
         connection_types.push(ConnectionType::Cellular);
      }

      connection_types
   }
}

/// Information about the current network connection.
///
/// Combines reachability, cost/constraint flags, and the physical [`ConnectionType`]
/// to give callers enough context to make network policy decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
   /// Whether the device has an active network path.
   ///
   /// A connected path can still be limited or not fully usable. Check
   /// [`Self::constrained`] when the caller needs usable internet access.
   pub connected: bool,

   /// Whether data usage is billed or limited (e.g. mobile data plans, capped
   /// hotspots), or [`None`] when the backend cannot determine the cost.
   ///
   /// Platform mapping:
   /// - **Windows:** `NetworkCostType` is `Fixed` or `Variable`; `Unknown`
   ///   returns [`None`]
   /// - **Linux:** NetworkManager primary device `Metered` is `YES` or
   ///   `GUESS_YES`; `UNKNOWN`, unavailable device details, and passive fallback
   ///   return [`None`]
   /// - **iOS:** `NWPath.isExpensive`
   /// - **Android:** absence of `NET_CAPABILITY_NOT_METERED`
   pub metered: Option<bool>,

   /// Whether the connection is constrained -- approaching or over its data limit,
   /// roaming, or background data usage is restricted -- or [`None`] when the
   /// backend cannot determine the constraint state.
   ///
   /// Platform mapping:
   /// - **Windows:** `ConstrainedInternetAccess`, `ApproachingDataLimit`,
   ///   `OverDataLimit`, `Roaming`, or `BackgroundDataUsageRestricted`;
   ///   this is computed independently of `metered`, so unknown cost alone does
   ///   not make the constraint state unknown
   /// - **Linux:** NetworkManager `Connectivity` is `PORTAL` or `LIMITED`,
   ///   primary device is metered, or ModemManager reports cellular roaming;
   ///   unknown or unavailable source signals and passive fallback can return
   ///   [`None`]
   /// - **iOS:** `NWPath.isConstrained` (Low Data Mode)
   /// - **Android:** missing `NET_CAPABILITY_VALIDATED`, or Data Saver /
   ///   `RESTRICT_BACKGROUND_STATUS` on a metered active network
   pub constrained: Option<bool>,

   /// The physical or logical transport used to connect to the network.
   pub connection_type: ConnectionType,
}

impl ConnectionStatus {
   /// Returns a [`ConnectionStatus`] representing a disconnected state.
   pub fn disconnected() -> Self {
      Self {
         connected: false,
         metered: Some(false),
         constrained: Some(false),
         connection_type: ConnectionType::Unknown,
      }
   }
}

#[cfg(test)]
mod tests {
   use super::*;
   use rstest::rstest;

   #[test]
   fn serializes_connection_status() {
      let status = ConnectionStatus {
         connected: true,
         metered: Some(true),
         constrained: Some(false),
         connection_type: ConnectionType::Cellular,
      };
      let json = serde_json::to_value(&status).unwrap();

      assert_eq!(json["connected"], true);
      assert_eq!(json["metered"], true);
      assert_eq!(json["constrained"], false);
      assert_eq!(json["connectionType"], "cellular");
   }

   #[rstest]
   #[case::wifi(ConnectionType::Wifi, "wifi")]
   #[case::ethernet(ConnectionType::Ethernet, "ethernet")]
   #[case::cellular(ConnectionType::Cellular, "cellular")]
   #[case::unknown(ConnectionType::Unknown, "unknown")]
   fn serializes_all_connection_types(#[case] variant: ConnectionType, #[case] expected: &str) {
      let json = serde_json::to_value(variant).unwrap();
      assert_eq!(json, expected);
   }

   #[test]
   fn deserializes_connection_status() {
      let json =
         r#"{"connected":true,"metered":false,"constrained":false,"connectionType":"wifi"}"#;
      let status: ConnectionStatus = serde_json::from_str(json).unwrap();

      assert!(status.connected);
      assert_eq!(status.metered, Some(false));
      assert_eq!(status.constrained, Some(false));
      assert_eq!(status.connection_type, ConnectionType::Wifi);
   }

   #[rstest]
   #[case::wifi("\"wifi\"", ConnectionType::Wifi)]
   #[case::ethernet("\"ethernet\"", ConnectionType::Ethernet)]
   #[case::cellular("\"cellular\"", ConnectionType::Cellular)]
   #[case::unknown("\"unknown\"", ConnectionType::Unknown)]
   fn deserializes_all_connection_types(#[case] json: &str, #[case] expected: ConnectionType) {
      let result: ConnectionType = serde_json::from_str(json).unwrap();
      assert_eq!(result, expected);
   }

   #[test]
   fn disconnected_returns_expected_defaults() {
      let status = ConnectionStatus::disconnected();

      assert!(!status.connected);
      assert_eq!(status.metered, Some(false));
      assert_eq!(status.constrained, Some(false));
      assert_eq!(status.connection_type, ConnectionType::Unknown);
   }

   #[test]
   fn serializes_unknown_policy_flags_as_null() {
      let status = ConnectionStatus {
         connected: true,
         metered: None,
         constrained: None,
         connection_type: ConnectionType::Ethernet,
      };
      let json = serde_json::to_value(&status).unwrap();

      assert!(json["metered"].is_null());
      assert!(json["constrained"].is_null());
   }

   #[rstest]
   #[case::known_types_are_deduped_and_unknown_ignored(
      &[
         ConnectionType::Cellular,
         ConnectionType::Unknown,
         ConnectionType::Wifi,
         ConnectionType::Cellular,
         ConnectionType::Ethernet,
      ],
      &[ConnectionType::Wifi, ConnectionType::Ethernet, ConnectionType::Cellular]
   )]
   #[case::only_unknown_inserted_is_empty(&[ConnectionType::Unknown], &[])]
   fn connection_types_into_vec_returns_inserted_known_types(
      #[case] inserted: &[ConnectionType],
      #[case] expected: &[ConnectionType],
   ) {
      let mut connection_types = ConnectionTypes::new();

      for &connection_type in inserted {
         connection_types.insert(connection_type);
      }

      assert_eq!(connection_types.into_vec(), expected);
   }
}
