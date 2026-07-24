# macOS Connectivity Manual Testing

Manual macOS scenarios for exercising `src/platform/macos.rs`.

The macOS backend keeps a process-lifetime `NWPathMonitor` and caches the latest
path update. It reports connected only for a satisfied path, derives metered
and constrained flags from that path, and uses the first interface in Apple's
preference order as `connectionType`.

`supportedConnectionTypes()` enumerates the interfaces available to the current
satisfied path. It does not inventory inactive network hardware. Before the
first monitor update, and whenever the path is disconnected, it returns an
empty array.

## Automated Checks

Run the Rust checks before manual testing:

```sh
cargo test --workspace --lib
```

## Reference Links

| Item | Link |
| ---- | ---- |
| Tauri prerequisites | <https://v2.tauri.app/start/prerequisites/> |
| `NWPathMonitor` API | <https://developer.apple.com/documentation/network/nwpathmonitor> |
| `NWPath` API | <https://developer.apple.com/documentation/network/nwpath> |
| `nw_path_enumerate_interfaces` API | <https://developer.apple.com/documentation/network/nw_path_enumerate_interfaces(_:_:)> |

## Scenario Coverage

| Scenario | Status | Expected result |
| -------- | ------ | --------------- |
| Wi-Fi connected | Not tested | `connected: true`, `connectionType: "wifi"` |
| Ethernet connected | Not tested | `connected: true`, `connectionType: "ethernet"` |
| Disconnected | Not tested | disconnected status and empty supported types |
| Personal Hotspot | Not tested | `metered: true` when macOS marks the path expensive |
| Low Data Mode | Not tested | `constrained: true` |
| Wi-Fi and Ethernet available | Not tested | preferred interface is the connection type; both can appear in supported types |
| VPN over Wi-Fi | Not tested | transport can be `unknown` and supported types can be empty when only the tunnel is exposed |
| Query immediately on launch | Not tested | initial cache can briefly report disconnected until the first path update |

## Base Test Setup

Install the root and example dependencies:

```sh
npm install
cd examples/tauri-app
npm install
```

Run the desktop example app:

```sh
cd examples/tauri-app
npm run dev
```

For each scenario, press `Refresh status` in the example app and record the
`Raw response`.

## Manual Scenarios

### Wi-Fi Connected

1. Disconnect Ethernet.
2. Connect to a Wi-Fi network.
3. Run or refresh the example app.

Expected response:

```json
{
   "connected": true,
   "metered": false,
   "constrained": false,
   "connectionType": "wifi"
}
```

Expected `supportedConnectionTypes()` response:

```json
["wifi"]
```

### Ethernet Connected

1. Connect Ethernet.
2. Disable Wi-Fi.
3. Run or refresh the example app.

Expected response:

```json
{
   "connected": true,
   "connectionType": "ethernet"
}
```

Expected `supportedConnectionTypes()` response:

```json
["ethernet"]
```

### Disconnected

1. Disable Wi-Fi.
2. Disconnect Ethernet and any active VPN.
3. Run or refresh the example app.

Expected response:

```json
{
   "connected": false,
   "metered": false,
   "constrained": false,
   "connectionType": "unknown"
}
```

Expected `supportedConnectionTypes()` response:

```json
[]
```

### Wi-Fi And Ethernet Available

1. Connect both Wi-Fi and Ethernet.
2. Run or refresh the example app.

`connectionType` should identify the first interface in the path's preference
order. `supportedConnectionTypes()` can include both `"wifi"` and `"ethernet"`
because it enumerates every known interface available to the satisfied path.

### Personal Hotspot And Low Data Mode

1. Connect through a Personal Hotspot and check whether `metered` becomes
   `true`.
2. Enable Low Data Mode for the active network and check whether `constrained`
   becomes `true`.

These values reflect `nw_path_is_expensive` and `nw_path_is_constrained`; macOS
decides the flags for the current path.

### VPN Over Wi-Fi

1. Connect to Wi-Fi.
2. Enable a VPN that routes all traffic through its tunnel.
3. Run or refresh the example app.

The path can expose only a tunnel interface that maps to `unknown`, hiding the
underlying Wi-Fi interface. In that case the connection remains usable, but
`connectionType` is `"unknown"` and `supportedConnectionTypes()` returns `[]`.

### Query Immediately On Launch

1. Fully quit the example app.
2. Relaunch it and query the status immediately.
3. Refresh after the first `NWPathMonitor` update arrives.

The monitor callback is asynchronous. Until its first update populates the
cache, `connectionStatus()` reports disconnected and
`supportedConnectionTypes()` returns `[]`.
