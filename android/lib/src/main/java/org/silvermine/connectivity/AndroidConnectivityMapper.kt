package org.silvermine.connectivity

enum class ConnectionType(val serializedName: String) {
   WIFI("wifi"),
   ETHERNET("ethernet"),
   CELLULAR("cellular"),
   UNKNOWN("unknown")
}

object AndroidConnectivityMapper {
   fun isConnected(hasInternet: Boolean): Boolean {
      return hasInternet
   }

   fun isMetered(hasNotMetered: Boolean, hasTemporarilyNotMetered: Boolean): Boolean {
      return !hasNotMetered && !hasTemporarilyNotMetered
   }

   fun isConstrained(
      isValidated: Boolean,
      isBackgroundRestricted: Boolean,
      isMetered: Boolean
   ): Boolean {
      // Unvalidated networks include captive portals and other limited paths.
      // Data Saver restricts background data only on metered networks.
      return !isValidated || (isBackgroundRestricted && isMetered)
   }

   fun connectionType(
      hasWifi: Boolean,
      hasEthernet: Boolean,
      hasCellular: Boolean
   ): ConnectionType {
      if (hasWifi) {
         return ConnectionType.WIFI
      }

      if (hasEthernet) {
         return ConnectionType.ETHERNET
      }

      if (hasCellular) {
         return ConnectionType.CELLULAR
      }

      return ConnectionType.UNKNOWN
   }

   fun supportedConnectionTypes(
      hasWifi: Boolean,
      hasEthernet: Boolean,
      hasCellular: Boolean,
      activeTransportTypes: List<ConnectionType>
   ): List<ConnectionType> {
      // Keep the API order stable across platforms and filter UNKNOWN here so
      // callers can use the result directly for policy-setting UI.
      val connectionTypes = linkedSetOf<ConnectionType>()

      if (hasWifi) {
         connectionTypes.add(ConnectionType.WIFI)
      }
      if (hasEthernet) {
         connectionTypes.add(ConnectionType.ETHERNET)
      }
      if (hasCellular) {
         connectionTypes.add(ConnectionType.CELLULAR)
      }

      activeTransportTypes
         .filter { connectionType -> connectionType != ConnectionType.UNKNOWN }
         .forEach { connectionType -> connectionTypes.add(connectionType) }

      return listOf(ConnectionType.WIFI, ConnectionType.ETHERNET, ConnectionType.CELLULAR)
         .filter { connectionType -> connectionTypes.contains(connectionType) }
   }
}
