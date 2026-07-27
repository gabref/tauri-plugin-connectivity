package org.silvermine.plugin.connectivity

import android.content.Context
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import org.silvermine.connectivity.AndroidConnectivityMapper
import org.silvermine.connectivity.ConnectionType

class Connectivity(context: Context) {
   private val connectivityManager =
      context.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
   private val packageManager = context.packageManager

   fun connectionStatus(): JSObject {
      val manager = connectivityManager ?: return NativeConnectionStatus.disconnected().toJSObject()
      val activeNetwork = manager.activeNetwork ?: return NativeConnectionStatus.disconnected().toJSObject()
      val capabilities = manager.getNetworkCapabilities(activeNetwork)
         ?: return NativeConnectionStatus.disconnected().toJSObject()

      val hasInternet = capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
      val isValidated = capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)

      if (!AndroidConnectivityMapper.isConnected(hasInternet)) {
         return NativeConnectionStatus.disconnected().toJSObject()
      }

      // `TEMPORARILY_NOT_METERED` means the active network should be treated as
      // effectively unmetered while Android exposes that capability.
      val hasTemporarilyNotMetered = hasTemporarilyNotMetered(capabilities)
      val metered = AndroidConnectivityMapper.isMetered(
         capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED),
         hasTemporarilyNotMetered
      )
      val status = NativeConnectionStatus(
         connected = true,
         metered = metered,
         constrained = AndroidConnectivityMapper.isConstrained(
            isValidated,
            isBackgroundRestricted(manager),
            metered
         ),
         connectionType = AndroidConnectivityMapper.connectionType(
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI),
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET),
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)
         )
      )

      return status.toJSObject()
   }

   fun supportedConnectionTypes(): JSObject {
      val manager = requireNotNull(connectivityManager) {
         "ConnectivityManager is unavailable"
      }

      // Android does not expose a complete public inventory of inactive
      // removable network hardware. Combine PackageManager's declared system
      // features with currently tracked ConnectivityManager networks:
      // https://developer.android.com/reference/android/content/pm/PackageManager
      // https://developer.android.com/reference/android/net/ConnectivityManager#getAllNetworks()
      val activeTransportTypes = manager.allNetworks
         .mapNotNull { network -> manager.getNetworkCapabilities(network) }
         .map { capabilities ->
            AndroidConnectivityMapper.connectionType(
               capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI),
               capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET),
               capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)
            )
         }
      val connectionTypes = AndroidConnectivityMapper.supportedConnectionTypes(
         hasWifi = packageManager.hasSystemFeature(PackageManager.FEATURE_WIFI),
         hasEthernet = packageManager.hasSystemFeature(PackageManager.FEATURE_ETHERNET),
         hasCellular = packageManager.hasSystemFeature(PackageManager.FEATURE_TELEPHONY),
         activeTransportTypes = activeTransportTypes
      )
      val result = JSObject()
      val serializedTypes = JSArray()

      connectionTypes.forEach { connectionType ->
         serializedTypes.put(connectionType.serializedName)
      }
      result.put("value", serializedTypes)

      return result
   }

   private fun isBackgroundRestricted(manager: ConnectivityManager): Boolean {
      // Data Saver's restrict-background status was added after API 23.
      if (Build.VERSION.SDK_INT < Build.VERSION_CODES.N) {
         return false
      }

      return manager.restrictBackgroundStatus == ConnectivityManager.RESTRICT_BACKGROUND_STATUS_ENABLED
   }

   private fun hasTemporarilyNotMetered(capabilities: NetworkCapabilities): Boolean {
      // Guard the API 30 capability so the plugin still supports API 23+.
      if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
         return false
      }

      return capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_TEMPORARILY_NOT_METERED)
   }
}

data class NativeConnectionStatus(
   val connected: Boolean,
   val metered: Boolean,
   val constrained: Boolean,
   val connectionType: ConnectionType
) {
   fun toJSObject(): JSObject {
      val status = JSObject()

      status.put("connected", connected)
      status.put("metered", metered)
      status.put("constrained", constrained)
      status.put("connectionType", connectionType.serializedName)

      return status
   }

   companion object {
      fun disconnected(): NativeConnectionStatus {
         return NativeConnectionStatus(
            connected = false,
            metered = false,
            constrained = false,
            connectionType = ConnectionType.UNKNOWN
         )
      }
   }
}
