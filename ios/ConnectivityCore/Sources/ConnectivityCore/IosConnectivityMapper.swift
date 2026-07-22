public enum ConnectionType: String, Encodable {
   case wifi
   case ethernet
   case cellular
   case unknown
}

/// Priority logic:
/// (wifi → ethernet → cellular → unknown)
public enum IosConnectivityMapper {
   public static func connectionType(
      hasWifi: Bool,
      hasEthernet: Bool,
      hasCellular: Bool
   ) -> ConnectionType {
      supportedConnectionTypes(
         hasWifi: hasWifi,
         hasEthernet: hasEthernet,
         hasCellular: hasCellular
      ).first ?? .unknown
   }

   /// The supported transports of the current path, deduplicated in stable API
   /// order. A satisfied path that exposes none of these interfaces (for
   /// example one using only `.other` or `.loopback`) yields an empty list.
   public static func supportedConnectionTypes(
      hasWifi: Bool,
      hasEthernet: Bool,
      hasCellular: Bool
   ) -> [ConnectionType] {
      var connectionTypes: [ConnectionType] = []

      if hasWifi {
         connectionTypes.append(.wifi)
      }

      if hasEthernet {
         connectionTypes.append(.ethernet)
      }

      if hasCellular {
         connectionTypes.append(.cellular)
      }

      return connectionTypes
   }
}
