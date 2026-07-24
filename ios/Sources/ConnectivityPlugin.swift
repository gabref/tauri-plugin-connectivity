import ConnectivityCore
import Foundation
import Network
import Tauri

private struct ConnectionStatusPayload: Encodable {
   let connected: Bool
   let metered: Bool
   let constrained: Bool
   let connectionType: ConnectionType
}

private struct SupportedConnectionTypesPayload: Encodable {
   let value: [ConnectionType]
}

class ConnectivityPlugin: Plugin {
   private let monitor = NWPathMonitor()
   private let monitorQueue = DispatchQueue(label: "tauri.plugin.connectivity.path")
   private let stateQueue = DispatchQueue(label: "tauri.plugin.connectivity.state")
   private var latestPath: NWPath?
   private let firstPathGroup = DispatchGroup()
   private var hasReceivedFirstPath = false

   // Upper bound on how long an early command waits for the
   // initial NWPathMonitor update before falling back to monitor.currentPath.
   private static let firstPathTimeout: DispatchTimeInterval = .milliseconds(200)

   override init() {
      super.init()
      firstPathGroup.enter()
      monitor.pathUpdateHandler = { [weak self] path in
         guard let self else { return }
         self.stateQueue.async {
            self.latestPath = path
            if !self.hasReceivedFirstPath {
               self.hasReceivedFirstPath = true
               self.firstPathGroup.leave()
            }
         }
      }
      monitor.start(queue: monitorQueue)
   }

   deinit {
      monitor.cancel()
   }

   /// Returns the current network connection status.
   @objc public func connectionStatus(_ invoke: Invoke) throws {
      let path = resolveCurrentPath()
      let connectionType = Self.resolveConnectionType(path)

      invoke.resolve(ConnectionStatusPayload(
         connected: path.status == .satisfied,
         metered: path.isExpensive,
         constrained: path.isConstrained,
         connectionType: connectionType
      ))
   }

   /// Returns the transport classes available to the current network path.
   @objc public func supportedConnectionTypes(_ invoke: Invoke) throws {
      let path = resolveCurrentPath()
      let availableInterfaces = path.availableInterfaces
      let supportedTypes = path.status == .satisfied
         ? IosConnectivityMapper.supportedConnectionTypes(
            hasWifi: availableInterfaces.contains { $0.type == .wifi },
            hasEthernet: availableInterfaces.contains { $0.type == .wiredEthernet },
            hasCellular: availableInterfaces.contains { $0.type == .cellular }
         )
         : []

      invoke.resolve(SupportedConnectionTypesPayload(value: supportedTypes))
   }

   // The first pathUpdateHandler callback is delivered asynchronously after
   // start(), so on an early call latestPath may still be nil. Briefly wait
   // for that first update rather than immediately falling back to
   // `monitor.currentPath`, which may report `.requiresConnection` in that
   // window and under-report connectivity. The wait is bounded so the
   // calling thread never blocks indefinitely.
   private func resolveCurrentPath() -> NWPath {
      if stateQueue.sync(execute: { latestPath }) == nil {
         _ = firstPathGroup.wait(timeout: .now() + Self.firstPathTimeout)
      }
      return stateQueue.sync { latestPath } ?? monitor.currentPath
   }

   // Adapter over `IosConnectivityMapper`
   // A satisfied path that uses only `.other` or
   // `.loopback` interfaces matches none of these and maps to `.unknown`.
   private static func resolveConnectionType(_ path: NWPath) -> ConnectionType {
      IosConnectivityMapper.connectionType(
         hasWifi: path.usesInterfaceType(.wifi),
         hasEthernet: path.usesInterfaceType(.wiredEthernet),
         hasCellular: path.usesInterfaceType(.cellular)
      )
   }
}

@_cdecl("init_plugin_connectivity")
func initPlugin() -> Plugin {
   return ConnectivityPlugin()
}
