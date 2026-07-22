import XCTest
@testable import ConnectivityCore

final class IosConnectivityMapperTests: XCTestCase {
   func testWifiTakesPriorityOverEverything() {
      XCTAssertEqual(
         IosConnectivityMapper.connectionType(
            hasWifi: true,
            hasEthernet: true,
            hasCellular: true
         ),
         .wifi
      )
   }

   func testEthernetWhenNoWifi() {
      XCTAssertEqual(
         IosConnectivityMapper.connectionType(
            hasWifi: false,
            hasEthernet: true,
            hasCellular: true
         ),
         .ethernet
      )
   }

   func testCellularWhenNoWifiOrEthernet() {
      XCTAssertEqual(
         IosConnectivityMapper.connectionType(
            hasWifi: false,
            hasEthernet: false,
            hasCellular: true
         ),
         .cellular
      )
   }

   func testUnknownWhenNoKnownInterfaceIsPresent() {
      XCTAssertEqual(
         IosConnectivityMapper.connectionType(
            hasWifi: false,
            hasEthernet: false,
            hasCellular: false
         ),
         .unknown
      )
   }

   func testSerializedRawValuesMatchGuestContract() {
      XCTAssertEqual(ConnectionType.wifi.rawValue, "wifi")
      XCTAssertEqual(ConnectionType.ethernet.rawValue, "ethernet")
      XCTAssertEqual(ConnectionType.cellular.rawValue, "cellular")
      XCTAssertEqual(ConnectionType.unknown.rawValue, "unknown")
   }

   func testSupportedConnectionTypesListsAllTransportsInStableOrder() {
      XCTAssertEqual(
         IosConnectivityMapper.supportedConnectionTypes(
            hasWifi: true,
            hasEthernet: true,
            hasCellular: true
         ),
         [.wifi, .ethernet, .cellular]
      )
   }

   func testSupportedConnectionTypesOmitsAbsentTransports() {
      XCTAssertEqual(
         IosConnectivityMapper.supportedConnectionTypes(
            hasWifi: true,
            hasEthernet: false,
            hasCellular: true
         ),
         [.wifi, .cellular]
      )
   }

   func testSupportedConnectionTypesIsEmptyWhenNoKnownInterfaceIsPresent() {
      XCTAssertEqual(
         IosConnectivityMapper.supportedConnectionTypes(
            hasWifi: false,
            hasEthernet: false,
            hasCellular: false
         ),
         []
      )
   }
}
