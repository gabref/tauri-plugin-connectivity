use tracing::{debug, warn};
use windows::Networking::Connectivity::{
   ConnectionCost, ConnectionProfile, NetworkConnectivityLevel, NetworkCostType, NetworkInformation,
};
use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_NO_DATA, NO_ERROR, S_OK};
use windows::Win32::NetworkManagement::IpHelper::{
   GAA_FLAG_INCLUDE_ALL_INTERFACES, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
   GAA_FLAG_SKIP_MULTICAST, GAA_FLAG_SKIP_UNICAST, GetAdaptersAddresses, GetIfEntry2,
   IP_ADAPTER_ADDRESSES_LH, MIB_IF_ROW2,
};
use windows::Win32::Networking::WinSock::AF_UNSPEC;

use crate::error::{Error, Result};
use crate::types::{ConnectionStatus, ConnectionType, ConnectionTypes};

/// [`IanaInterfaceType`](https://www.iana.org/assignments/ianaiftype-mib/ianaiftype-mib) values.
/// IANA interface type for Ethernet-like interfaces (`ethernetCsmacd`).
const IANA_ETHERNET_CSMACD: u32 = 6;
/// IANA interface type for IEEE 802.11 wireless LAN.
const IANA_IEEE80211: u32 = 71;
/// IANA interface types for WWAN mobile broadband transports.
const IANA_WWANPP: u32 = 243;
const IANA_WWANPP2: u32 = 244;

const INITIAL_ADAPTER_BUFFER_SIZE: u32 = 15 * 1024;
const MAX_ADAPTER_QUERY_ATTEMPTS: usize = 3;

// `MIB_IF_ROW2.InterfaceAndOperStatusFlags` stores `HardwareInterface` in bit 0
// and `ConnectorPresent` in bit 2. Requiring both follows the structure's
// semantics for a hardware-backed interface with a physical connector, which
// excludes hardware-backed virtual miniports that do not expose one.
// https://learn.microsoft.com/en-us/windows/win32/api/netioapi/ns-netioapi-mib_if_row2
const HARDWARE_INTERFACE_FLAG: u8 = 1;
const CONNECTOR_PRESENT_FLAG: u8 = 1 << 2;

fn is_physical_interface(flags: u8) -> bool {
   flags & HARDWARE_INTERFACE_FLAG != 0 && flags & CONNECTOR_PRESENT_FLAG != 0
}

fn classify_enumeration_result(
   result: u32,
   success: u32,
   no_data: u32,
) -> std::result::Result<bool, u32> {
   if result == success {
      Ok(true)
   } else if result == no_data {
      Ok(false)
   } else {
      Err(result)
   }
}

fn collect_successful_items<T, E>(
   results: impl IntoIterator<Item = std::result::Result<Option<T>, E>>,
) -> std::result::Result<Vec<T>, E> {
   let mut items = Vec::new();
   let mut first_error = None;

   for result in results {
      match result {
         Ok(Some(item)) => items.push(item),
         Ok(None) => {}
         Err(error) => {
            first_error.get_or_insert(error);
         }
      }
   }

   if items.is_empty()
      && let Some(error) = first_error
   {
      return Err(error);
   }

   Ok(items)
}

/// Returns the current network connection status using WinRT
/// [`NetworkInformation`](https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.networkinformation?view=winrt-28000).
///
/// Windows exposes a "preferred" internet profile rather than a single canonical
/// device-wide network. We query that profile and derive connectivity, cost, and
/// transport information from the resulting
/// [`ConnectionProfile`](https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.connectionprofile?view=winrt-28000).
pub(crate) fn connection_status() -> Result<ConnectionStatus> {
   debug!("querying Windows internet connection profile");

   let profile = match NetworkInformation::GetInternetConnectionProfile() {
      Ok(profile) => profile,
      Err(error) if is_missing_profile_error(&error) => {
         debug!("Windows did not return an internet connection profile");
         return Ok(ConnectionStatus::disconnected());
      }
      Err(error) => {
         warn!(%error, "failed to query Windows internet connection profile");
         return Err(error.into());
      }
   };

   let connectivity_level = profile
      .GetNetworkConnectivityLevel()
      .inspect_err(|error| warn!(%error, "failed to query Windows connectivity level"))?;

   debug!(
      connectivity_level = ?connectivity_level,
      "queried Windows connectivity level"
   );

   if !has_network_connectivity(connectivity_level) {
      debug!(
         connectivity_level = ?connectivity_level,
         "connectivity level does not indicate internet or constrained access"
      );
      return Ok(ConnectionStatus::disconnected());
   }

   let connection_cost = profile
      .GetConnectionCost()
      .inspect_err(|error| warn!(%error, "failed to query Windows connection cost"))?;

   let cost_type = connection_cost
      .NetworkCostType()
      .inspect_err(|error| warn!(%error, "failed to query Windows network cost type"))?;

   let constrained = is_constrained_connectivity(connectivity_level)
      || is_constrained_cost(&connection_cost).inspect_err(
         |error| warn!(%error, "failed to query Windows constrained connection flags"),
      )?;

   let connection_type = resolve_connection_type(&profile)
      .inspect_err(|error| warn!(%error, "failed to resolve Windows connection type"))?;

   let status = ConnectionStatus {
      connected: true,
      metered: metered_status(cost_type),
      constrained: Some(constrained),
      connection_type,
   };

   debug!(
      ?cost_type,
      constrained = ?status.constrained,
      connection_type = ?status.connection_type,
      metered = ?status.metered,
      "resolved Windows connection status"
   );

   Ok(status)
}

/// Returns the supported physical connection transport classes.
pub(crate) fn supported_connection_types() -> Result<Vec<ConnectionType>> {
   debug!("querying Windows supported connection types");

   // Use Win32 adapter enumeration rather than WinRT connection profiles:
   // `GetAdaptersAddresses` returns adapters present on the local computer,
   // while `NetworkInformation::GetConnectionProfiles()` can include saved
   // profiles that are not current hardware. The API and buffer contract are
   // documented here:
   // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
   Ok(collect_supported_connection_types_from_adapters(
      adapter_interface_types()?,
   ))
}

/// The WinRT binding can return a success-coded error when the API succeeds but
/// does not provide a preferred internet connection profile. Treat only `S_OK` as
/// this missing-profile case; other success HRESULTs are informational results
/// that should not be silently swallowed.
fn is_missing_profile_error(error: &windows::core::Error) -> bool {
   error.code() == S_OK
}

/// Treat full or constrained internet access as connected.
///
/// The returned [`ConnectionProfile`](https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.connectionprofile?view=winrt-28000)
/// can still represent local-only or constrained access. The plugin reports
/// constrained internet access as connected but constrained so captive portal
/// and similar limited-internet cases do not look fully offline.
///
/// Microsoft also notes that connectivity level is only a hint and apps should
/// re-check at the decision point rather than assume earlier results.
fn has_network_connectivity(connectivity_level: NetworkConnectivityLevel) -> bool {
   matches!(
      connectivity_level,
      NetworkConnectivityLevel::InternetAccess
         | NetworkConnectivityLevel::ConstrainedInternetAccess
   )
}

/// Windows reports metering through
/// [`ConnectionCost`](https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.connectioncost?view=winrt-28000).
/// Fixed-cost and variable-cost plans are metered, unrestricted plans are not,
/// and `Unknown` means Windows did not provide enough cost information.
fn metered_status(cost_type: NetworkCostType) -> Option<bool> {
   match cost_type {
      NetworkCostType::Unrestricted => Some(false),
      NetworkCostType::Fixed | NetworkCostType::Variable => Some(true),
      _ => None,
   }
}

/// Windows exposes several cost-related flags. We treat approaching/over-limit
/// roaming, and background data restrictions as constrained because callers use
/// this field for conservative network policy decisions. The relevant flags all
/// come from
/// [`ConnectionCost`](https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.connectioncost?view=winrt-28000).
fn is_constrained_cost(connection_cost: &ConnectionCost) -> Result<bool> {
   Ok(connection_cost.ApproachingDataLimit()?
      || connection_cost.OverDataLimit()?
      || connection_cost.Roaming()?
      || connection_cost.BackgroundDataUsageRestricted()?)
}

/// Windows reports captive portal and similar limited-internet cases through
/// [`NetworkConnectivityLevel::ConstrainedInternetAccess`].
fn is_constrained_connectivity(connectivity_level: NetworkConnectivityLevel) -> bool {
   connectivity_level == NetworkConnectivityLevel::ConstrainedInternetAccess
}

/// Prefer the explicit WLAN/WWAN profile checks first.
///
/// Windows already exposes higher-level transport classification on
/// [`ConnectionProfile`](https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.connectionprofile?view=winrt-28000).
/// The adapter interface type is only a fallback when those profile-level checks
/// do not classify the transport.
fn resolve_connection_type(profile: &ConnectionProfile) -> Result<ConnectionType> {
   if profile.IsWlanConnectionProfile()? {
      debug!("Windows classified the preferred profile as WLAN");
      return Ok(ConnectionType::Wifi);
   }

   if profile.IsWwanConnectionProfile()? {
      debug!("Windows classified the preferred profile as WWAN");
      return Ok(ConnectionType::Cellular);
   }

   let iana_interface_type = profile.NetworkAdapter()?.IanaInterfaceType()?;

   debug!(
      iana_interface_type,
      "falling back to IANA interface type for connection classification"
   );

   Ok(map_iana_interface_type(iana_interface_type))
}

/// Maps the adapter's IANA interface type to a plugin-level transport.
///
/// IANA assigns standard numeric identifiers for network interface categories.
/// Windows surfaces those identifiers on the adapter, which lets us recognize
/// common transports when the higher-level profile checks do not classify the
/// transport.
///
/// References:
/// - IANA interface type registry: <https://www.iana.org/assignments/ianaiftype-mib/ianaiftype-mib>
/// - Windows `NetworkAdapter`: <https://learn.microsoft.com/en-us/uwp/api/windows.networking.connectivity.networkadapter?view=winrt-28000>
fn map_iana_interface_type(iana_interface_type: u32) -> ConnectionType {
   match iana_interface_type {
      IANA_ETHERNET_CSMACD => ConnectionType::Ethernet,
      IANA_IEEE80211 => ConnectionType::Wifi,
      IANA_WWANPP | IANA_WWANPP2 => ConnectionType::Cellular,
      _ => ConnectionType::Unknown,
   }
}

fn is_supported_interface_type(iana_interface_type: u32) -> bool {
   map_iana_interface_type(iana_interface_type) != ConnectionType::Unknown
}

fn collect_supported_connection_types(
   iana_interface_types: impl IntoIterator<Item = u32>,
) -> Vec<ConnectionType> {
   let mut connection_types = ConnectionTypes::new();

   for iana_interface_type in iana_interface_types {
      connection_types.insert(map_iana_interface_type(iana_interface_type));
   }

   connection_types.into_vec()
}

fn collect_supported_connection_types_from_adapters(
   adapters: impl IntoIterator<Item = (u32, bool)>,
) -> Vec<ConnectionType> {
   collect_supported_connection_types(
      adapters
         .into_iter()
         .filter_map(|(iana_interface_type, is_physical)| {
            is_physical.then_some(iana_interface_type)
         }),
   )
}

fn adapter_interface_types() -> Result<Vec<(u32, bool)>> {
   // Microsoft recommends a 15 KB initial buffer to avoid repeated allocation
   // for typical adapter lists. If the buffer is still too small, retry with
   // the required size up to the three attempts used in Microsoft's example.
   // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
   let (buffer, result) = query_adapter_buffer(|buffer, size| unsafe {
      // `GAA_FLAG_INCLUDE_ALL_INTERFACES` includes adapters regardless of
      // operational state, matching the supported-hardware contract. The skip
      // flags avoid populating address lists we do not inspect.
      // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
      GetAdaptersAddresses(
         AF_UNSPEC.0.into(),
         GAA_FLAG_INCLUDE_ALL_INTERFACES
            | GAA_FLAG_SKIP_UNICAST
            | GAA_FLAG_SKIP_ANYCAST
            | GAA_FLAG_SKIP_MULTICAST
            | GAA_FLAG_SKIP_DNS_SERVER,
         None,
         Some(buffer),
         size,
      )
   });

   match classify_enumeration_result(result, NO_ERROR.0, ERROR_NO_DATA.0) {
      Ok(true) => {}
      Ok(false) => return Ok(Vec::new()),
      Err(result) => {
         return Err(Error::SupportedConnectionTypesDetectionFailed {
            message: String::from("GetAdaptersAddresses failed"),
            code: Some(result as i32),
         });
      }
   }

   let mut adapter_results = Vec::new();
   let mut adapter = buffer.as_ptr();

   while !adapter.is_null() {
      let adapter_ref = unsafe { &*adapter };

      // Unknown interface types cannot contribute a supported transport. Skip
      // them before querying physical status so a failure on an irrelevant
      // loopback, tunnel, or other adapter cannot fail detection.
      if !is_supported_interface_type(adapter_ref.IfType) {
         debug!(
            iana_interface_type = adapter_ref.IfType,
            "skipping unsupported Windows network adapter type"
         );
         adapter = adapter_ref.Next;
         continue;
      }

      // IANA interface types alone do not distinguish a physical Ethernet
      // adapter from Ethernet-like virtual adapters. Query the interface row
      // by LUID and require both `HardwareInterface` and `ConnectorPresent`.
      // `ConnectorPresent` distinguishes physical adapters from virtual
      // miniports that can still report themselves as hardware-backed:
      // https://learn.microsoft.com/en-us/windows/win32/api/netioapi/ns-netioapi-mib_if_row2
      let mut interface = MIB_IF_ROW2 {
         InterfaceLuid: adapter_ref.Luid,
         ..MIB_IF_ROW2::default()
      };
      let interface_result = unsafe { GetIfEntry2(&mut interface) };

      if interface_result != NO_ERROR {
         warn!(
            code = interface_result.0,
            iana_interface_type = adapter_ref.IfType,
            "failed to query Windows adapter physical status; skipping adapter"
         );
         adapter_results.push(Err(interface_result.0));
         adapter = adapter_ref.Next;
         continue;
      }

      let is_physical = is_physical_interface(interface.InterfaceAndOperStatusFlags._bitfield);

      if !is_physical {
         debug!(
            iana_interface_type = adapter_ref.IfType,
            "skipping virtual Windows network adapter"
         );
      }

      adapter_results.push(Ok(is_physical.then_some((adapter_ref.IfType, is_physical))));
      adapter = adapter_ref.Next;
   }

   collect_successful_items(adapter_results).map_err(|result| {
      Error::SupportedConnectionTypesDetectionFailed {
         message: String::from("GetIfEntry2 failed for every supported adapter"),
         code: Some(result as i32),
      }
   })
}

fn query_adapter_buffer(
   mut query: impl FnMut(*mut IP_ADAPTER_ADDRESSES_LH, &mut u32) -> u32,
) -> (Vec<IP_ADAPTER_ADDRESSES_LH>, u32) {
   let mut size = INITIAL_ADAPTER_BUFFER_SIZE;

   for attempt in 0..MAX_ADAPTER_QUERY_ATTEMPTS {
      let mut buffer = adapter_buffer(size);
      let result = query(buffer.as_mut_ptr(), &mut size);

      if result != ERROR_BUFFER_OVERFLOW.0 || attempt + 1 == MAX_ADAPTER_QUERY_ATTEMPTS {
         return (buffer, result);
      }
   }

   unreachable!("adapter query attempt limit is nonzero")
}

fn adapter_buffer(size_in_bytes: u32) -> Vec<IP_ADAPTER_ADDRESSES_LH> {
   let adapter_count =
      (size_in_bytes as usize).div_ceil(std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>());

   vec![IP_ADAPTER_ADDRESSES_LH::default(); adapter_count.max(1)]
}

#[cfg(test)]
mod tests {
   use super::*;
   use rstest::rstest;
   use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, S_FALSE};
   use windows::core::{Error, HRESULT};

   #[rstest]
   #[case::internet_access(NetworkConnectivityLevel::InternetAccess, true)]
   #[case::constrained_internet_access(NetworkConnectivityLevel::ConstrainedInternetAccess, true)]
   #[case::none(NetworkConnectivityLevel::None, false)]
   #[case::local_access(NetworkConnectivityLevel::LocalAccess, false)]
   fn detects_internet_access_for_full_and_constrained_levels(
      #[case] connectivity_level: NetworkConnectivityLevel,
      #[case] expected: bool,
   ) {
      assert_eq!(has_network_connectivity(connectivity_level), expected);
   }

   #[rstest]
   #[case::constrained_internet_access(NetworkConnectivityLevel::ConstrainedInternetAccess, true)]
   #[case::internet_access(NetworkConnectivityLevel::InternetAccess, false)]
   fn treats_constrained_internet_access_as_constrained(
      #[case] connectivity_level: NetworkConnectivityLevel,
      #[case] expected: bool,
   ) {
      assert_eq!(is_constrained_connectivity(connectivity_level), expected);
   }

   #[rstest]
   #[case::unknown(NetworkCostType::Unknown, None)]
   #[case::unrestricted(NetworkCostType::Unrestricted, Some(false))]
   #[case::fixed(NetworkCostType::Fixed, Some(true))]
   #[case::variable(NetworkCostType::Variable, Some(true))]
   fn maps_metered_cost_types_without_collapsing_unknown(
      #[case] cost_type: NetworkCostType,
      #[case] expected: Option<bool>,
   ) {
      assert_eq!(metered_status(cost_type), expected);
   }

   #[rstest]
   #[case::empty_windows_error_is_missing_profile(Error::empty(), true)]
   #[case::success_false_hresult_is_not_missing_profile(Error::from_hresult(S_FALSE), false)]
   #[case::failure_hresult_is_not_missing_profile(Error::from_hresult(HRESULT(-1)), false)]
   fn detects_missing_profile_error(#[case] error: Error, #[case] expected: bool) {
      assert_eq!(is_missing_profile_error(&error), expected);
   }

   #[rstest]
   #[case::ethernet(IANA_ETHERNET_CSMACD, ConnectionType::Ethernet)]
   #[case::wifi(IANA_IEEE80211, ConnectionType::Wifi)]
   #[case::wwanpp(IANA_WWANPP, ConnectionType::Cellular)]
   #[case::wwanpp2(IANA_WWANPP2, ConnectionType::Cellular)]
   #[case::unrecognized(999, ConnectionType::Unknown)]
   fn maps_iana_interface_types(
      #[case] iana_interface_type: u32,
      #[case] expected: ConnectionType,
   ) {
      assert_eq!(map_iana_interface_type(iana_interface_type), expected);
   }

   #[rstest]
   #[case::ethernet(IANA_ETHERNET_CSMACD, true)]
   #[case::wifi(IANA_IEEE80211, true)]
   #[case::wwanpp(IANA_WWANPP, true)]
   #[case::unrecognized(999, false)]
   fn queries_physical_status_only_for_supported_interface_types(
      #[case] iana_interface_type: u32,
      #[case] expected: bool,
   ) {
      assert_eq!(is_supported_interface_type(iana_interface_type), expected);
   }

   #[test]
   fn collects_supported_connection_types_from_adapter_interface_types() {
      assert_eq!(
         collect_supported_connection_types([
            IANA_WWANPP,
            999,
            IANA_IEEE80211,
            IANA_WWANPP2,
            IANA_ETHERNET_CSMACD,
         ]),
         vec![
            ConnectionType::Wifi,
            ConnectionType::Ethernet,
            ConnectionType::Cellular,
         ]
      );
   }

   #[test]
   fn excludes_virtual_adapters_from_supported_connection_types() {
      assert_eq!(
         collect_supported_connection_types_from_adapters([
            (IANA_IEEE80211, true),
            (IANA_ETHERNET_CSMACD, false),
         ]),
         vec![ConnectionType::Wifi]
      );
   }

   #[rstest]
   #[case::no_flags(0, false)]
   #[case::hardware_only(HARDWARE_INTERFACE_FLAG, false)]
   #[case::connector_only(CONNECTOR_PRESENT_FLAG, false)]
   #[case::hardware_and_connector(HARDWARE_INTERFACE_FLAG | CONNECTOR_PRESENT_FLAG, true)]
   fn identifies_physical_interfaces_from_status_flags(#[case] flags: u8, #[case] expected: bool) {
      assert_eq!(is_physical_interface(flags), expected);
   }

   #[rstest]
   #[case::no_data_is_empty_enumeration(ERROR_NO_DATA.0, Ok(false))]
   #[case::success_is_populated_enumeration(NO_ERROR.0, Ok(true))]
   #[case::failure_is_preserved(ERROR_INVALID_PARAMETER.0, Err(ERROR_INVALID_PARAMETER.0))]
   fn classifies_native_enumeration_results(
      #[case] result: u32,
      #[case] expected: std::result::Result<bool, u32>,
   ) {
      assert_eq!(
         classify_enumeration_result(result, NO_ERROR.0, ERROR_NO_DATA.0),
         expected
      );
   }

   #[rstest]
   #[case::every_item_query_fails_reports_failure(&[Err(1168), Err(1168)], Err(1168))]
   #[case::success_without_usable_items_reports_failure(&[Err(1168), Ok(None)], Err(1168))]
   #[case::all_succeed_without_items_returns_empty(&[Ok(None), Ok(None)], Ok(Vec::new()))]
   #[case::one_item_query_succeeds_keeps_partial_results(
      &[Err(1168), Ok(Some((71, true)))],
      Ok(vec![(71, true)])
   )]
   fn collects_successful_items(
      #[case] results: &[std::result::Result<Option<(u32, bool)>, u32>],
      #[case] expected: std::result::Result<Vec<(u32, bool)>, u32>,
   ) {
      assert_eq!(collect_successful_items(results.iter().copied()), expected);
   }

   #[test]
   fn retries_adapter_query_until_the_third_attempt_succeeds() {
      let mut attempts = 0;

      let (_, result) = query_adapter_buffer(|_, size| {
         attempts += 1;

         if attempts < 3 {
            *size += 1024;
            ERROR_BUFFER_OVERFLOW.0
         } else {
            NO_ERROR.0
         }
      });

      assert_eq!(result, NO_ERROR.0);
      assert_eq!(attempts, 3);
   }

   #[test]
   fn stops_retrying_adapter_query_after_three_overflows() {
      let mut attempts = 0;

      let (_, result) = query_adapter_buffer(|_, size| {
         attempts += 1;
         *size += 1024;
         ERROR_BUFFER_OVERFLOW.0
      });

      assert_eq!(result, ERROR_BUFFER_OVERFLOW.0);
      assert_eq!(attempts, 3);
   }
}
