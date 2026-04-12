# Android Provisioning App MVP

> Historical note: this file keeps the first APK MVP scope. The current APK status lives in [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md), and the current board/app provisioning contract lives in [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md).

This is the first Android-only app target for Lumelo Wi-Fi provisioning.

## Scope

The MVP app only does provisioning:

- scan for `Lumelo T4` over classic Bluetooth
- connect to the provisioning service over classic Bluetooth
- send Wi-Fi SSID and password
- show progress and final result
- show the Lumelo WebUI URL after the device reports an IP address

It does not implement music browsing or playback control. After Wi-Fi succeeds,
the user should open the normal WebUI.

## Screens

1. `Find Lumelo`
- Shows a single scan button
- Lists classic Bluetooth devices whose name starts with `Lumelo`
- Shows signal strength if available

2. `Connect`
- Shows the chosen device name
- Connects/pairs over classic Bluetooth
- Reads the device info JSON

3. `Wi-Fi`
- Text input for SSID
- Password input for WPA-PSK
- `Use Current Wi-Fi` shortcut to prefill the phone's current SSID when available
- Send button
- Manual `Read Status` button for bring-up retries
- Manual `Disconnect` button so the user can restart BLE pairing without force-closing the app

4. `Result`
- Shows `Applying`, `Connected`, or `Failed`
- If connected, shows `http://<device-ip>:18080/`
- If connected, automatically opens the Lumelo main interface inside the APK
- Provides open buttons for:
  - WebUI root
  - `/library`
  - `/provisioning`
  - `/logs`
  - `/healthz`
- Keeps a small on-screen debug log for scan / connect / GATT status transitions
- Allows clearing the on-screen debug log between retry attempts
- Starts a temporary automatic status polling loop after sending credentials so
  the user does not need to manually tap `Read Status` on every retry

## Android Permissions

Target Android 12+ first:

- `BLUETOOTH_SCAN`
- `BLUETOOTH_CONNECT`
- `ACCESS_FINE_LOCATION`

For older Android versions, the app may also need classic Bluetooth/location
permission fallbacks. Keep those out of the first implementation unless testing
requires them.

## Main Transport Contract

The app should use the Lumelo provisioning protocol defined in
[Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md).

Main transport:

- classic Bluetooth `RFCOMM / SPP`

Payloads remain JSON UTF-8 strings in the first version.

Client commands:

- `{"type":"device_info"}`
- `{"type":"wifi_credentials_encrypted","payload":{...}}`
- `{"type":"apply"}`
- `{"type":"status"}`

Current secure transport behavior:

- the app should inspect `hello.security`
- when `hello.security.scheme = dh-hmac-sha256-stream-v1`, the app should send
  `wifi_credentials_encrypted`
- the secure payload should carry the same logical content as plaintext
  credentials, but the Wi-Fi password must no longer appear as plaintext
  over the Bluetooth application protocol
- if the board does not advertise secure credential transport, the app should
  stop and ask for a board-side update instead of sending plaintext Wi-Fi
  credentials

Example status payload:

```json
{"type":"status","payload":{"state":"connected","ip":"192.168.1.42","web_url":"http://192.168.1.42:18080/"}}
```

The app should also tolerate richer payloads such as:

```json
{"type":"status","payload":{"state":"waiting_for_ip","message":"credentials applied; waiting for DHCP","ssid":"Home WiFi","wifi_interface":"wlan0","status_path":"/run/lumelo/provisioning-status.json"}}
```

## BLE Diagnostic Mode

`Raw BLE Scan` remains in scope as a diagnostics tool.

It is no longer the main provisioning transport, but it should continue to:

- scan nearby BLE advertisements
- list local name / UUID / manufacturer data
- help us judge whether the board is emitting any BLE signal at all

The first in-app main interface can stay thin:

- an Android `WebView`
- loads `http://<device-ip>:18080/`
- exposes quick buttons for:
  - Home
  - Library
  - Provisioning
  - Logs
  - open in external browser
  - back to setup

## Implementation Preference

Use a simple native Android project first:

- Kotlin
- Jetpack Compose
- one Activity
- no backend account
- no analytics
- no cloud dependency

The first APK can be debug-signed. Release signing and app-store polish are out
of scope for the bring-up phase.

## Validation

The APK MVP is considered usable when:

- it discovers the T4 over classic Bluetooth
- it connects without needing Linux login credentials
- it writes SSID/password
- the T4 reports either `connected` with an IP or `failed` with a readable error
- the operator can manually re-read status and disconnect without relaunching the app
- the operator can prefill the current Wi-Fi SSID from the phone when Android exposes it
- after `connected`, the APK can enter the in-app main interface without kicking the user to an external browser
- the user can open `http://<device-ip>:18080/logs` from the phone browser
