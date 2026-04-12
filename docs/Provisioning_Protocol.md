# Provisioning Protocol

This document defines the current Bluetooth / Wi-Fi provisioning protocol between
the T4 and the Android provisioning app.

Historical product-scope notes for the first APK MVP are archived in
[archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md).

## Current T4-Side Foundation

The next rebuilt image is expected to include:

- `bluez`
- `wpasupplicant`
- `iw`
- `rfkill`
- `wireless-regdb`
- `/usr/bin/lumelo-bluetooth-provisioning-mode`
- `/usr/bin/lumelo-wifi-apply`
- `/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond`
- `/etc/systemd/network/30-wireless-dhcp.network`

This foundation is enough to validate whether the NanoPC-T4 exposes a usable
Bluetooth controller and Wi-Fi interface under the Lumelo rootfs.

## Provisioning User Flow

1. T4 boots without known Wi-Fi credentials.
2. T4 starts Bluetooth provisioning mode.
3. Phone app discovers `Lumelo T4`.
4. User pairs or connects over classic Bluetooth.
5. Phone app sends SSID and password to the T4.
6. T4 writes credentials and restarts Wi-Fi.
7. Phone app shows success once the T4 has an IP address.
8. User opens the normal Lumelo WebUI on Wi-Fi.

## Transport Decision

当前 bring-up 已经确认：

- 经典蓝牙在接好天线后可被手机系统蓝牙设置页发现
- BLE 广播在当前 `T4` 板上仍不稳定

因此配网主通道调整为：

- 经典蓝牙 `RFCOMM / SPP` 作为主传输层
- `Raw BLE Scan` 保留为诊断工具，不再作为主配网发现路径

## Provisioning Message Shape

经典蓝牙主通道不再依赖 GATT characteristic。

但上层业务语义继续沿用当前定义：

- `device_info`: JSON with hostname, build id, and current IP state
- `wifi_credentials_encrypted`: encrypted credential payload carrying the same `ssid/password` semantics
- `apply`: trigger that asks the T4 to apply the last credentials
- `status`: JSON with `advertising`, `credentials_ready`, `applying`, `waiting_for_ip`, `connected`, or `failed`

首版经典蓝牙协议采用逐行 JSON：

- `{"type":"device_info"}`
- `{"type":"wifi_credentials_encrypted","payload":{...}}`
- `{"type":"apply"}`
- `{"type":"status"}`

当前协议已经扩展为协商式安全传输：

- `hello` 现在会额外携带 `security` 字段
- 当前实现中：
  - App 只发送 `wifi_credentials_encrypted`
- 当前实现采用：
  - `scheme = dh-hmac-sha256-stream-v1`
  - `dh_group = modp14-sha256`
- 板端会在 `hello.security` 中提供：
  - `session_id`
  - `server_nonce`
  - `server_public_key`
- 手机端在发送加密凭据时提供：
  - `client_public_key`
  - `client_nonce`
  - `message_nonce`
  - `ciphertext`
  - `mac`

设计边界：

- 这一轮先解决“蓝牙传输链路不再明文暴露 Wi-Fi 密码”
- 板端“非明文持久化存储”不在当前改动范围内，后续在固件改造时一并处理
- 板端当前会拒绝旧的明文 `wifi_credentials` 命令，并返回：
  - `code = plaintext_credentials_disabled`

板端响应：

- `{"type":"device_info","payload":{...}}`
- `{"type":"status","payload":{...}}`
- `{"type":"ack","message":"..."}`
- `{"type":"error","message":"...","code":"..."}`

The first implementation should accept only WPA-PSK credentials. Open networks
and enterprise Wi-Fi can stay out of scope.

## T4 Implementation Notes

当前主实现调整为经典蓝牙 RFCOMM 服务：

- `/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond`

它负责：

- 让 `T4` 进入经典蓝牙 discoverable / pairable 模式
- 通过 `sdptool` 注册 `SPP` 服务
- 接受手机端 RFCOMM JSON 指令
- 调用 `/usr/bin/lumelo-wifi-apply`
- 持续输出 `/run/lumelo/provisioning-status.json`

The current bring-up iteration also writes the latest status snapshot to
`/run/lumelo/provisioning-status.json` so `controld`, SSH, and the T4 report
script can all inspect the same runtime state.

That snapshot should now also carry:

- `error_code`
- `apply_output`
- `diagnostic_hint`
- `wpa_unit`
- `ip_wait_seconds`

`/usr/bin/lumelo-bluetooth-provisioning-mode` 仍负责在服务启动前把控制器拉到
经典蓝牙可发现 / 可连接状态。

`/usr/bin/lumelo-wifi-apply` should no longer assume `wlan0`; it should prefer
`LUMELO_WIFI_IFACE`, then `WIFI_INTERFACE`, then auto-detect the first wireless
interface via `iw dev` or `/sys/class/net/*/wireless`.

## App Role

Start with Android only unless iOS becomes a hard requirement. The app should
only do:

- scan for `Lumelo T4` over classic Bluetooth
- connect/pair over classic Bluetooth
- send SSID/password
- prefill the current phone Wi-Fi SSID when available
- show connection result
- show the WebUI URL after success
- automatically enter the APK-hosted main interface after `connected`
- allow manual status refresh and disconnect during bring-up
- automatically poll status for a short window after apply
- expose the board-side `/provisioning`, `/logs`, and `/healthz` pages once an IP is known

`Raw BLE Scan` 作为诊断能力保留，用来判断板子是否还有 BLE 广播，但它不再
承担主配网职责。

The WebUI home page should also expose a compact provisioning summary so the
operator can see the latest Bluetooth / Wi-Fi state without leaving `/`.

The log page remains part of the WebUI, not the mobile provisioning app.

The first Android-only MVP scope is archived in
[archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md).

The current APK structure, status, and follow-up roadmap are maintained in
[Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md).
