# Lumelo 音频系统开发进度日志

## 1. 文档用途

本文件只记录当前阶段进展、最新验证事实、未闭环事项和下一步。

长期产品边界看：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)

环境、在线更新、出包规则看：

- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)

手机 APK 当前状态看：

- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)

配网协议和板端 classic Bluetooth 契约看：

- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)

## 2. 整理规则

2026-04-25 起，本日志已按用户要求清理：

- `v20` 之前的过程性记录已删除。
- 已形成长期结论的内容不再留在本日志里堆历史，改由对应权威文档维护。
- 本日志从 `v20` 之后的 WebUI / APK / T4 classic provisioning 进展继续记录。

当前新增协作规则：

- 默认继续在线修和 live 验证。
- 用户明确说“出包 / 出 image / 出 img / 打镜像”时，才生成新的 T4 `image/img`。
- 如果确实遇到必须出包才能解决的问题，先说明原因和风险，再建议用户下达出包命令。

## 3. 当前开发基线

- 当前目标版本：`V1`
- 当前工作模式：`Local Mode`
- 当前产品形态：`headless` 本地音频系统
- steady-state 主交互：Ethernet / Wi-Fi 下的 `WebUI`
- APK 定位：classic Bluetooth / Wi-Fi provisioning 工具、联网后的 WebView 壳、诊断入口

当前服务边界：

- `playbackd` 是播放状态、队列状态和切歌逻辑权威。
- `sessiond` 只负责 Quiet Mode 与系统环境切换。
- `controld` 负责 WebUI / API / 设置 / 认证，不承载播放队列语义。
- `media-indexd` 是按需索引 worker，不应成为播放期高活跃后台。

## 4. 当前未闭环事项

### 4.1 T4 / classic provisioning

已验证：

- live `T4 192.168.71.7` 上，classic provisioning 两层真根因已钉死并 runtime 修复：
  - `lumelo-bluetooth-provisioning-mode` 在 systemd/no-TTY 下裸跑 `btmgmt` 会挂住，已改为 pseudo-tty wrapper。
  - `bluetooth.service` 未以 `bluetoothd -C` 启动时，`sdptool` compat SDP 路径不可用，已新增 `30-lumelo-compat.conf`。
- runtime 修复后，手机 APK 的 `SCAN -> CONNECT -> device_info -> status` 已跑通。
- 2026-04-26 用 `PJZ110 / Android 16` 真实执行 `SCAN -> CONNECT -> USE CURRENT WI-FI -> SEND WI-FI CREDENTIALS`：
  - SSID：`iSee`
  - T4 `wlan0` 获得 `192.168.71.243/24`
  - APK 自动进入 `http://192.168.71.243:18080/`
  - 手机侧 `/healthz` 返回 `provisioning_state=connected`

还没验证：

- 把这些修复刷入新镜像后的 cold boot bring-up。
- 刷机后重新跑 `systemctl / journalctl / bluetoothctl show / sdptool browse local`。
- 刷机后手机 APK 再次 `SCAN -> CONNECT`。

### 4.2 APK

已验证：

- 当前源码已通过 `assembleDebug` 和 `lintDebug`。
- `COPY DIAGNOSTIC SUMMARY` 不再覆盖主状态。
- classic connect failure 后会探测 `Last known WebUI` 和当前 `/24`。
- live T4 修复后，APK 的 `SCAN -> CONNECT -> device_info -> status` 主链已跑通。
- live T4 修复后，APK 的 `SEND WI-FI CREDENTIALS` 到 WebView 自动跳转主链已跑通。
- live `T4 192.168.71.7` + `BKQ-AN90 / Android 16` 上，连接态入口已逐个点开：
  - `OPEN WEBUI` -> `/`
  - `OPEN PROVISIONING` -> `/provisioning`
  - `OPEN LOGS` -> `/logs`
  - `OPEN HEALTHZ` -> `/healthz`

还没验证：

- `ack timeout / write failed / reconnect` 的专门人为诱发回归。
- `MainInterfaceActivity` Android 16 back gesture 的手感。

### 4.3 WebUI / media

已验证：

- `player-first` mobile WebUI 已完成多轮 runtime update。
- 首页和曲库已经收成浅色 mobile-first 播放器语言。
- 当前 WebUI 仍遵守 steady-state 主交互定位。
- 首页播放列表区已改为 `播放历史`：
  - 只显示已到达过的曲目和当前播放曲目
  - 不提前暴露 shuffle / 伪随机后续队列顺序
  - 展示 title/path 时会用曲库 snapshot 补全历史记录里的 UID 占位
- 设置页已补 V1 当前解码器展示：
  - 未发现 USB Audio 解码器时显示 `未连接`
  - 发现 USB Audio 解码器时显示解码器名称

还没验证：

- 真实专辑 / artwork 数据态下的最终视觉。
- 当前最新镜像刷入后的手机 Safari 复验。
- 真实播放 smoke 的最新镜像回归。
- 真机连接 / 断开 USB DAC 后的设置页显示。

## 5. 进展记录

### 5.1 2026-04-19：`player-first` 浅色新版 WebUI runtime update

这轮只动 `controld` Web layer，没有改 `playbackd / sessiond / media-indexd`。

主要变化：

- 首页改为 `music player first`：
  - 大封面 / 当前曲目 / transport controls / 队列 / 最近专辑。
- 曲库改为 `album-first`：
  - 最近专辑、专辑详情、曲目列表优先。
- `Provisioning / Logs` 下沉为诊断入口。
- 文案和按钮中文化。

已验证：

- `go test ./internal/api ./internal/provisioningclient ./internal/settings`
- `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build ./cmd/controld`
- runtime update 到 live `T4 192.168.1.110`
- `/healthz`、`/provisioning-status`、`/logs.txt` 可达。

### 5.2 2026-04-21：`v21` image 收口 WebUI 阶段

这轮把 player-first WebUI 收成阶段性镜像：

- [lumelo-t4-rootfs-20260421-v21.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260421-v21.img)
- `sha256 = 9764a053a01152cff0d58ec49228f559f0b96d156372fd649dc305dd26c8ff7d`

已验证：

- `go test ./internal/api ./internal/provisioningclient ./internal/settings`
- `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build`
- `verify-t4-lumelo-rootfs-image.sh = 0 failure(s), 0 warning(s)`

未跑：

- `compare-t4-wireless-golden.sh`

原因：

- 这轮只改 WebUI / `controld`，没有动无线、蓝牙、firmware、SSH 或 boot 链。

### 5.3 2026-04-22：APK classic provisioning recovery

这轮收 APK recovery 和诊断：

- classic session reset / disconnect 后，不再清掉最后已知 `WebUI URL`。
- APK 内 WebView 顶栏中文化：
  - `首页 / 曲库 / 日志 / 设置 / 重试 / 浏览器 / 返回`
- `RFCOMM write failed` 改为关闭 socket 后走现有 reader/reconnect 判定。
- classic connect failure 后给出更明确提示：
  - `Lumelo was discovered, but the provisioning service did not answer.`

已验证：

- `./gradlew :app:assembleDebug`
- debug APK 重新安装到 `PJZ110`
- `scan -> connect -> in-app WebView` 最小 smoke 正常。
- 断开后仍保留 `Last known WebUI`，并能再次打开 WebUI。

尚未验证：

- 人为诱发 `ack timeout / write failed / reconnect` 的完整闭环。

### 5.4 2026-04-22：APK connect failure 后补 WebUI 探测

这轮继续增强现场诊断：

- 如果 classic connect failure 后存在 `Last known WebUI`，APK 会后台探测 `/healthz`。
- 如果旧地址不可达，APK 会继续扫当前 `/24` 的 `18080/healthz`。
- 如果没有命中，直接提示：
  - `No Lumelo WebUI responded on the current /24 subnet.`

已验证：

- 手机 `wlan0 = 192.168.71.6/24`
- 旧地址 `192.168.71.9:18080` 当时对手机不可达。
- APK 状态文案能准确区分：
  - classic scan 仍能看到设备
  - RFCOMM connect 失败
  - last known WebUI 不可达
  - 当前 `/24` 也没找到 Lumelo WebUI

### 5.5 2026-04-23：APK `Copy Diagnostic Summary`

这轮新增现场转发能力：

- 新增 `COPY DIAGNOSTIC SUMMARY`
- `Export Diagnostics` 头部新增 quick summary

summary 包含：

- 当前 `Status`
- 手机当前 `Wi-Fi / IPv4`
- `Last known T4 Wi-Fi`
- `Last known WebUI`
- 最近一次 WebUI probe
- 最近一次 `/24` scan
- classic session 摘要

已验证：

- `./gradlew :app:assembleDebug`
- 最新 debug APK 安装到 `PJZ110`
- 点击按钮后 debug log 出现：
  - `Copied current diagnostic summary to clipboard`

### 5.6 2026-04-24：APK hard bug sweep

这轮按 fail-fast 规则清 hard error：

- `COPY DIAGNOSTIC SUMMARY` 改成 `Toast + debug log`，不覆盖主状态。
- 权限链补 `BLUETOOTH_SCAN + coarse/fine location`。
- `BluetoothGatt.close()` 显式处理 `SecurityException`。
- `/24` subnet scan 不再静默吞异常。
- `MainInterfaceActivity` 补 `OnBackInvokedCallback`。
- `ProvisioningSecurity` 去掉 `BigInteger.TWO`，兼容 `minSdk 26`。

已验证：

- `./gradlew :app:assembleDebug`
- `./gradlew :app:lintDebug`
- `lintDebug = 0 errors`

### 5.7 2026-04-24：APK warning 中有行为风险的部分已收掉

这轮只收有运行时风险的 warning：

- `AndroidManifest.xml` 显式补 `dataExtractionRules / fullBackupContent`。
- 新增 `data_extraction_rules.xml` 和 `backup_rules.xml`。
- 3 处 UI thread 上的 `SharedPreferences commit()` 改为 `apply()`。
- `requestPreferredMtu()` 的 `SecurityException` 写进 debug log。

已验证：

- `./gradlew :app:assembleDebug`
- `./gradlew :app:lintDebug`
- 当前 lint：
  - `0 errors`
  - 剩余 warning 只剩 `MissingApplicationIcon / SetTextI18n`

### 5.8 2026-04-24：板端 classic provisioning lifecycle 修复

这轮回到 T4 板端，按 fail-fast 原则收 classic provisioning：

- `classic-bluetooth-wifi-provisiond`
  - 启动前清 stale SPP SDP record。
  - 退出时清本次注册的 SDP record。
  - 新增 `--cleanup-sdp`。
  - adapter / SDP / socket startup failure 拆成精确 error code。
  - 不再用泛化错误覆盖精确 failure。
- `lumelo-bluetooth-provisioning-mode`
  - 支持 `enable / disable`。
  - 校验 `powered / discoverable / pairable`。
- `lumelo-bluetooth-provisioning.service`
  - 新增 `PartOf=lumelo-wifi-provisiond.service`。
  - 新增 `ExecStop=/usr/bin/lumelo-bluetooth-provisioning-mode disable`。
- `lumelo-wifi-provisiond.service`
  - 新增 `BindsTo=lumelo-bluetooth-provisioning.service`。
  - 新增 `ExecStopPost=... --cleanup-sdp`。
- `controld`
  - `/healthz`、首页、`/provisioning` 透出 `bluetooth_address / rfcomm_channel / sdp_record_handles / error_code`。

已验证：

- `python3 -m py_compile classic-bluetooth-wifi-provisiond`
- `sh -n lumelo-bluetooth-provisioning-mode`
- `sh -n lumelo-t4-report`
- `go test ./internal/api ./internal/provisioningclient`
- 本地 SDP cleanup / failure-code mock 通过。

### 5.9 2026-04-24：`v22 image` 产出，但随后 live 发现仍缺最终修复

这轮曾产出：

- [lumelo-t4-rootfs-20260424-v22.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260424-v22.img)
- `sha256 = 2ee5d0ee1a973590b90a18618d628878f7fa36a2b2db35996bce514d5ab5b096`

已验证：

- `verify-t4-lumelo-rootfs-image.sh = 0 failure(s), 0 warning(s)`
- `compare-t4-wireless-golden.sh = 0 failure(s), 0 warning(s)`

后续 live 复验发现：

- `v22` 还不包含最终 runtime 已验证的两条关键修复：
  - systemd/no-TTY 下 `btmgmt` 需要 pseudo-tty wrapper。
  - `bluetooth.service` 需要 `bluetoothd -C` compat drop-in。

### 5.10 2026-04-25：live T4 钉死两层 classic provisioning 真根因

live board:

- `T4 192.168.71.7`
- 当前 `v22`

第一层真根因：

- 原 helper 中 `bluetoothctl` batch 不可靠。
- `system-alias Lumelo T4` 会触发参数错误。
- 裸跑 `btmgmt` 在 systemd/no-TTY 条件下会挂住。
- `script -q -c 'btmgmt ...' /dev/null` 能稳定工作。

第二层真根因：

- `bluetoothd` 没有以 `-C` 启动。
- `sdptool browse local / sdptool add SP` compat 路径不可用。

runtime 修复：

- 替换 `/usr/bin/lumelo-bluetooth-provisioning-mode`。
- 新增 `/etc/systemd/system/bluetooth.service.d/30-lumelo-compat.conf`：
  - `ExecStart=/usr/libexec/bluetooth/bluetoothd -C`

已验证：

- `bluetooth.service = active`
- `lumelo-bluetooth-provisioning.service = active`
- `lumelo-wifi-provisiond.service = active`
- `/healthz` 返回 `provisioning_available=true`
- `/run/lumelo/provisioning-status.json`：
  - `state=advertising`
  - `bluetooth_address=C0:84:7D:1F:37:C7`
  - `rfcomm_channel=1`
  - `sdp_record_handles=["0x10007"]`
- `sdptool browse local` 可列出 local records。

### 5.11 2026-04-25：手机 APK 复验 `SCAN -> CONNECT -> device_info -> status` 主链正常

这轮纠正了上一轮误判：

- 之前以为“选中设备后没有进入 connect，像 APK UI/state bug”。
- live 复验确认真实原因是：
  - `CONNECT` 按钮在列表下方，需要滚动到 `Selected:` 后面。

已验证：

- 手机能扫到：
  - `[LAST] [NAME] Lumelo T4 (C0:84:7D:1F:37:C7)`
- `CONNECT` 按钮存在且 `enabled=true`。
- 点击后：
  - `Classic session: connected=true`
  - `device=C0:84:7D:1F:37:C7`
- `Device info` 回读：
  - `Name: Lumelo T4`
  - `Hostname: lumelo`
  - `IP: 192.168.71.7`
  - `Wired IP: 192.168.71.7`
  - `Transport: classic_bluetooth`
  - `Web port: 18080`
- `READ STATUS` 回读：
  - `State: advertising`
  - `rfcomm_channel=1`
  - `sdp_record_handles=["0x10007"]`

### 5.12 2026-04-25：`v23 image` 已产出并过离线 gate

已产出：

- [lumelo-t4-rootfs-20260425-v23.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260425-v23.img)
- [lumelo-t4-rootfs-20260425-v23.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260425-v23.img.sha256)
- `sha256 = 33bd44deeb630544327aa1ef28544e65ecb44b41d63e3adaaf793c30e5618c4e`

包含修复：

- pseudo-tty `btmgmt` wrapper。
- `bluetoothd -C` compat drop-in。
- verify script 纳入对应检查。

已验证：

- `build-t4-lumelo-rootfs-image.sh`
- `verify-t4-lumelo-rootfs-image.sh = 0 failure(s), 0 warning(s)`
- `compare-t4-wireless-golden.sh = 0 failure(s), 0 warning(s)`

重要协作结论：

- `v23` 已存在，但以后不再默认频繁出新 image。
- 当前默认继续在线修。
- 后续只有用户明确下令“出包”时才出新 `image/img`。

尚未验证：

- `v23` 刷入后的 cold boot bring-up。
- 刷入后手机 APK 再次 `SCAN -> CONNECT`。

### 5.13 2026-04-25：APK 连接态入口复验通过

本轮继续沿用 live `T4 192.168.71.7`，未出新 `image/img`。

现场设备：

- `BKQ-AN90`
- Android 16

已验证：

- T4 Web 端点：
  - `/healthz = 200`
  - `/provisioning-status = 200`
  - `/ = 200`
  - `/provisioning = 200`
  - `/logs = 200`
  - `/logs.txt = 200`
- APK 重新跑通：
  - `SCAN FOR LUMELO`
  - 选中 `[LAST] [PAIRED] [NAME] Lumelo T4 (C0:84:7D:1F:37:C7)`
  - `CONNECT`
  - `device_info`
  - `status = advertising`
- APK 连接态入口逐个点开并进入 `MainInterfaceActivity`：
  - `OPEN WEBUI` -> `http://192.168.71.7:18080/`
  - `OPEN PROVISIONING` -> `http://192.168.71.7:18080/provisioning`
  - `OPEN LOGS` -> `http://192.168.71.7:18080/logs`
  - `OPEN HEALTHZ` -> `http://192.168.71.7:18080/healthz`

尚未验证：

- `ack timeout / write failed / auto reconnect` 的专门诱发回归。
- `MainInterfaceActivity` Android 16 back gesture 的手感。

### 5.14 2026-04-25：设置页补 V1 当前解码器展示

本轮只做 `controld` WebUI / API 小步改动，没有改 `playbackd` 输出选择语义。

已实现：

- 新增 `GET /api/v1/system/audio-output`。
- 新增 ALSA card 只读解析：
  - 数据源：`/proc/asound/cards`
  - V1 只把 `USB-Audio` card 视为解码器。
- 设置页新增 `当前解码器` 下拉框：
  - 未发现 USB Audio 解码器时显示 `未连接`
  - 发现 USB Audio 解码器时显示解码器名称
- 新增 [Audio_Output_Device_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Audio_Output_Device_Plan.md)，记录 V2 多解码器选择与 USB 事件监听计划。

已验证：

- `go test ./...`
- `go build ./cmd/controld`
- live `T4 192.168.71.7` runtime update：
  - `/usr/bin/controld` 已替换并重启 `controld.service`
  - 旧二进制备份到 `/usr/bin/controld.bak.20260425-161411`
  - `/healthz = 200`
  - `/api/v1/system/audio-output = 200`
  - 当前现场无 USB Audio 解码器，返回 `connected=false`
  - 设置页真实 HTML 显示 `当前解码器` 和 `未连接`

尚未验证：

- 真 USB DAC 插入 / 拔出后的页面显示。

### 5.15 2026-04-26：手机 APK 真实 Wi-Fi provisioning 跑通

本轮沿用 live `T4 192.168.71.7`，未出新 `image/img`。

现场设备：

- 手机：`PJZ110 / Android 16`
- 手机 Wi-Fi：`iSee`
- T4 wired IP：`192.168.71.7`
- T4 Wi-Fi IP：`192.168.71.243`

已验证：

- APK 真实操作链：
  - `SCAN FOR LUMELO`
  - 选中 `Lumelo T4 (C0:84:7D:1F:37:C7)`
  - `CONNECT`
  - `USE CURRENT WI-FI`
  - `SEND WI-FI CREDENTIALS`
- APK 自动进入 WebView：
  - `http://192.168.71.243:18080/`
- 手机侧验证：
  - `ping 192.168.71.243`：`0% packet loss`
  - `curl http://192.168.71.243:18080/healthz`：
    - `interface_mode=wifi`
    - `provisioning_state=connected`
    - `provisioning_message=wifi connected on wlan0`
- T4 本机验证：
  - `/run/lumelo/provisioning-status.json`：
    - `state=connected`
    - `ssid=iSee`
    - `wifi_ip=192.168.71.243`
    - `wired_ip=192.168.71.7`
  - `ip -4 addr show wlan0`：`192.168.71.243/24`
  - `iw dev wlan0 link`：已连接 `iSee`
  - `curl http://127.0.0.1:18080/healthz`：`provisioning_state=connected`

未验证 / 注意：

- 电脑侧直接访问 `http://192.168.71.243:18080/healthz` 超时，推测是电脑当前网络路径不在同一可达网段；手机侧和 T4 本机均已验证通过。
- `ack timeout / write failed / auto reconnect` 的专门诱发回归仍未做。

### 5.16 2026-04-26：首页队列区改为播放历史

本轮沿用 live `T4 192.168.71.243`，未出新 `image/img`。

已实现：

- `playbackd` 新增只读 `HISTORY_SNAPSHOT` IPC command。
- `controld` 新增 `/api/v1/playback/history`。
- 首页原 `接下来播放` 区改为 `播放历史`：
  - 只渲染已播放 / 当前曲目
  - 最近播放的曲目显示在最上方
  - 不再展示后续 shuffle 队列
  - 每条历史曲目提供 `播放` 按钮，走 `play_history`
  - `state=stopped` 时不把保留的 `current_track` 标成 `当前播放`
  - 历史记录 title/path 优先用曲库 snapshot enrich，避免把 track UID 当标题显示
- 首页 / 曲库 transport 中间按钮改为状态驱动：
  - `playing / quiet_active` 显示 `暂停`
  - `stopped / paused / 无活动播放` 显示 `播放`
  - `播放` action 会携带当前 / 建议 track id，避免 stopped 状态点击后报 `no_active_track`
- 首页 hero 和曲库 mini player 都保留 `停止` 按钮。
- 曲库专辑卡片补默认方形封面 placeholder，避免无封面专辑破坏网格格式。
- 底部 mini player 的曲名和路径类信息改为单行省略，避免长标题撑高悬浮播放器。

已验证：

- `cargo test -p ipc-proto -p playbackd`
- `go test ./internal/api ./internal/playbackclient`
- `go test ./...`
- `OrbStack / lumelo-dev` 原生 arm64 build `playbackd`
- live runtime update：
  - `/usr/bin/playbackd` 已替换并重启
  - `/usr/bin/controld` 已替换并重启
  - `/healthz = ok`
  - `/api/v1/playback/history` 返回 7 条现场历史
  - in-app browser 打开 `http://192.168.71.243:18080/`：
    - `播放历史` section row count = 28
    - history row 已出现 `播放` 按钮，form action 使用 `play_history`
    - 不包含 `接下来播放`
    - 不包含 `队列序号`
    - 不包含 `4a3fd89c83d706f4` hash title
    - 包含 enrich 后的 `C'est Si Bon`
  - in-app browser 打开 `http://192.168.71.243:18080/library?album_uid=7aa6bc469a99348a`：
    - 无封面专辑 placeholder count = 21
    - 长曲名底部 mini player 不再撑高，标题以省略号显示
    - 当前播放态下中间按钮为 `暂停 / value=pause`

未验证 / 注意：

- 本轮是 runtime update，尚未刷入新镜像验证 cold boot。
- 因重启 `playbackd`，现场 playback state 回到 `stopped`，未重新发起播放。

### 5.21 2026-04-26：固定 WebUI hostname 策略与 mDNS Quiet Mode 待开发项

结论：

- 默认入口：`http://lumelo.local/`
- 可靠入口：`http://<T4_IP>/`
- `http://lumelo/`：放弃，不开发、不承诺、不验收。

已同步到待开发：

- `REQ-P0-007`：`sessiond` 在正式 `Playback Quiet Mode active` 中关闭或抑制 mDNS/DNS-SD 广播，停止播放后恢复。
  - live T4 当前观测：mDNS 由已有 `systemd-resolved` 提供，`1 thread / RSS ~14 MB`，`avahi-daemon=inactive`。
  - 结论：开销很小但不是零；为符合极限纯净播放器目标，播放态应进一步降噪。
- `REQ-P0-008`：Android APK 在蓝牙配网完成后检测当前手机是否能访问 `http://lumelo.local/healthz`。
  - 成功：默认打开 `http://lumelo.local/`。
  - 失败：自动打开 provisioning status 返回的 `http://<T4_IP>/`。

已清理产品口径：

- 文档不再把 `http://lumelo/` 写成 best-effort 入口。
- 不为 `http://lumelo/` 打开 `LLMNR`、NetBIOS 或引入 router DNS 依赖。

### 5.20 2026-04-26：修复 live T4 runtime update 后 IPv4 DHCP 失效

背景：

- 用户通过手机 APK 重新发送 Wi-Fi 凭据后，T4 可以通过 Bluetooth 返回 `device_info`，但只汇报 IPv6：
  - `240e:38c:8469:b600:c284:7dff:fe1f:37c6`
- 手机侧可通过 IPv6 访问 WebUI，但 `/provisioning-status` 显示：
  - `dhcp_timeout`
  - `wifi_ip=""`

根因：

- live runtime update 时，`scripts/deploy-t4-runtime-update.sh` 继承了本机 overlay 文件 mode。
- 本机这轮相关 systemd 配置文件 mode 异常为 `0700`。
- 板端 `/etc/systemd/network/*.network` 被安装成 `0700` 后，`systemd-networkd` 无法读取配置：
  - `Failed to open /etc/systemd/network/20-wired-dhcp.network: Permission denied`
  - `Failed to open /etc/systemd/network/30-wireless-dhcp.network: Permission denied`
- 这不是镜像损坏，也不是 DHCP 被配置关闭；是 runtime deploy 文件权限问题。

已修复：

- 仓库本地权限修回：
  - `base/rootfs/overlay/etc/systemd/network/20-wired-dhcp.network = 0644`
  - `base/rootfs/overlay/etc/systemd/network/30-wireless-dhcp.network = 0644`
  - `base/rootfs/overlay/etc/systemd/system/controld.service = 0644`
- live T4 板端权限修回 `0644 root:root`。
- 重启 `systemd-networkd.service` 与 `wpa_supplicant@wlan0.service` 后，T4 重新拿到 IPv4：
  - `wlan0 = 192.168.71.243/24`
- 重启 `lumelo-wifi-provisiond.service` 刷新 provisioning runtime status。
- `scripts/deploy-t4-runtime-update.sh` 增加保护：
  - 部署 `/etc/systemd/network/*`
  - `/etc/systemd/system/*`
  - `/usr/lib/systemd/system/*`
  - `/etc/systemd/resolved.conf.d/*`
  - `/etc/systemd/dnssd/*`
  时强制使用 `0644`，避免后续 live update 再把 systemd 配置写成不可读。

已验证：

- 手机侧 `/provisioning-status`：
  - `state=connected`
  - `ip=192.168.71.243`
  - `wifi_ip=192.168.71.243`
  - `web_url=http://192.168.71.243/`
- Mac 侧：
  - `curl http://192.168.71.243/healthz` 返回 `status=ok`
  - `provisioning_state=connected`
  - `interface_mode=wifi`
- `dns-sd -G v4 lumelo.local` 可解析：
  - `lumelo.local. -> 192.168.71.243`
- `sh -n scripts/deploy-t4-runtime-update.sh`

尚未验证：

- `curl http://lumelo.local/healthz` 在 Mac 当前 resolver 路径下仍出现过一次解析超时；`dns-sd` 已确认 mDNS record 存在，后续可在手机浏览器或不同客户端继续验证短地址体验。

### 5.19 2026-04-26：WebUI 入口切到 `80/tcp`、启用 mDNS / DNS-SD

背景：

- 用户确认正式用户入口不应要求记住 `:18080`。
- 目标入口：
  - `http://lumelo.local/`
  - `http://<T4_IP>/`
- `http://lumelo/` 不作为产品入口，不开发、不承诺、不验收。

已修改：

- `controld.service` 默认监听 `0.0.0.0:80`。
- `systemd-networkd` wired / wireless 配置启用 `MulticastDNS=yes`。
- `systemd-resolved` 配置启用 `MulticastDNS=yes`，继续关闭 `LLMNR`。
- 新增 `/etc/systemd/dnssd/lumelo-http.dnssd`：
  - `Name=%H`
  - `Type=_http._tcp`
  - `Port=80`
- classic / legacy provisioning status 改为返回：
  - `web_url=http://<T4_IP>/`
  - `web_port=80`
- APK fallback WebUI URL 和 subnet scan 改为使用 port `80`。
- `18080` 兼容 redirect / 继续监听已放入 [Milestone_Progress_Document.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Milestone_Progress_Document.md) 待办。
- 未来 `controld` 非 root 化后的 `CAP_NET_BIND_SERVICE` 已放入待办。

已验证：

- `go test ./internal/api ./internal/provisioningclient`
- `python3 -m py_compile` for provisioning helpers
- `JAVA_HOME='/Applications/Android Studio.app/Contents/jbr/Contents/Home' ./gradlew :app:assembleDebug`
- `JAVA_HOME='/Applications/Android Studio.app/Contents/jbr/Contents/Home' ./gradlew :app:lintDebug`
- 新 APK 已安装到当前连接手机：
  - package: `com.lumelo.provisioning`
  - version: `0.1.0`
  - APK sha256: `2906907abf42f94fa41fdb77c8a3cc1a43167272f33e78c7b4257a72f5ace370`
  - `lastUpdateTime=2026-04-26 05:53:37`

T4 runtime 状态：

- runtime update 已把相关 rootfs overlay 文件写入 live `T4 192.168.71.243`。
- 执行 live network reload 后，`192.168.71.243` 暂时不可达。
- 需要断电重启 T4 后继续验证：
  - `http://<T4_IP>/healthz`
  - `http://<T4_IP>/`
  - `http://lumelo.local/`
  - `_http._tcp` DNS-SD service

### 5.19 2026-05-02：修复 `playbackd` 写死 ALSA output device

背景：

- live T4 在公司网络通过手机配网成功，WebUI 可访问，U 盘和 USB DAC 均已连接。
- 点击播放后 UI 直接回到 `stopped`。
- 板端 log 显示 `playbackd decoder stream error ... write pcm to aplay ... Broken pipe`。
- 真机 `/proc/asound/cards` 显示当前 USB DAC 为 `iBassoDC04Pro`，但 `playbackd.service` 固定设置：
  - `LUMELO_AUDIO_DEVICE=plughw:CARD=Audio,DEV=0`

已修复：

- `playbackd.service` 去掉固定 `LUMELO_AUDIO_DEVICE`。
- `playbackd` 每次开始播放前读取当前 `/proc/asound/cards`。
- V1 自动选择当前唯一 USB Audio DAC：
  - `plughw:CARD=<detected>,DEV=0`
- 未发现 USB Audio DAC 时 fail-fast：
  - `audio_output_unavailable`
- 同时发现多个 USB Audio DAC 时 fail-fast：
  - `audio_output_ambiguous`
- [Audio_Output_Device_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Audio_Output_Device_Plan.md) 已同步 V1 运行语义。

已验证：

- `cargo test --manifest-path services/rust/Cargo.toml -p playbackd`
- `OrbStack / lumelo-dev` arm64 release build `playbackd`
- live `T4 192.168.71.12` runtime update：
  - `/usr/bin/playbackd` 已替换并重启
  - `/etc/systemd/system/playbackd.service` 已替换并 `daemon-reload`
  - `playbackd.service = active`
  - `systemctl show playbackd.service -p Environment --value` 只剩 `LUMELO_RUNTIME_DIR=/run/lumelo`
  - startup log 显示 `audio device: auto USB DAC via /proc/asound/cards`
  - 当前真机 USB DAC：
    - `iBasso-DC04-Pro`
    - ALSA device: `plughw:CARD=iBassoDC04Pro,DEV=0`
  - `lumelo-media-smoke regress-playback --timeout 8 --skip-mixed` 通过
  - WAV path `aplay -D plughw:CARD=iBassoDC04Pro,DEV=0 ...`
  - FLAC decoded path `aplay -D plughw:CARD=iBassoDC04Pro,DEV=0 -t raw -f S16_LE -c 2 -r 44100`

尚未验证：

- 真机物理拔掉 DAC 后点击 WebUI 播放是否显示 `audio_output_unavailable`。
- 本轮是 runtime update，尚未刷入新镜像验证 cold boot。

### 5.18 2026-04-26：修复 `BUG-P0-005` remote API 绝对路径播放边界

背景：

- M1 bug 池确认：生产 / release 语义下，remote Web/API 不应能把任意绝对路径直接传给播放核心。
- `playbackd` 之前保留了 manual absolute path resolver，适合作为开发调试能力，但不能从远程控制面裸露出来。

已修复：

- `controld` 在远程 playback command ingress 处拒绝绝对路径：
  - `/api/v1/playback/commands`
  - legacy `/commands`
  - `/api/v1/library/commands`
  - legacy `/library/commands`
- `queue_play` / `queue_replace` 的 JSON track list 会逐项拒绝绝对路径。
- library context 生成 candidate queue 时也拒绝绝对路径形式的 `track_uid`。
- `playbackd` 默认关闭 absolute path resolver。
- 若确需开发调试绝对路径播放，必须显式设置：
  - `LUMELO_PLAYBACK_ALLOW_ABSOLUTE_PATHS=1`

已验证：

- `go test ./internal/api`
- `go test ./...` in `services/controld`
- `cargo test --manifest-path services/rust/Cargo.toml -p playbackd`
- `OrbStack / lumelo-dev` 原生 arm64 release build `playbackd`
- `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/lumelo-controld-arm64 ./cmd/controld`
- live `T4 192.168.71.243` runtime update：
  - `/usr/bin/playbackd` 已替换并重启
  - `/usr/bin/controld` 已替换并重启
  - `playbackd.service / controld.service` 均为 `active`
  - remote sha256 与本地构建产物一致
  - `/healthz = ok`
  - `playbackd` startup log 显示 `absolute paths: disabled`
  - `/api/v1/playback/commands` 的 `play` 绝对路径请求返回 `absolute_path_playback_forbidden`
  - `/api/v1/playback/commands` 的 `queue_play` JSON 列表内绝对路径请求返回 `absolute_path_playback_forbidden`
  - UDS 直连 `PLAY /tmp/manual.wav` 返回 `absolute_path_playback_disabled`
- Safari 真机 WebUI 手动回归：
  - 首页打开正常，`播放历史` 显示已到达历史曲目且 row 上有 `播放` 按钮
  - 首页 hero `播放 -> 暂停 -> 停止` 状态切换正常，最终回到 `stopped`
  - 曲库页打开正常，曲目列表和底部 player dock 正常显示
  - 设置页打开正常，当前解码器显示 `QTIL AURUM CANTUS Audio`

尚未验证：

- 本轮是 runtime update，尚未刷入新镜像验证 cold boot。

### 5.17 2026-04-26：修复 `play_history` 的 Play Now 语义

背景：

- 人工测试发现首页播放历史点击后没有形成预期播放体验。
- 对照 `Product_Development_Manual` 后确认：历史曲目必须走 `play_history`，且语义应是 `Play Now`，不是向队尾追加一个新的 manual entry。

已修复：

- `playbackd` 的 `PlayHistory` 不再 append 到队尾。
- `PlayHistory` 现在替换当前播放位，保留后续 `play_order`，并保留当前 `order_mode / repeat_mode`。
- 历史列表中同一 track 多次出现时，只把最近一条匹配记录标为 `当前播放`。

已验证：

- `cargo test -p ipc-proto -p playbackd`
- `go test ./internal/api ./internal/playbackclient`
- `go test ./...`
- `OrbStack / lumelo-dev` 原生 arm64 build `playbackd`
- `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/lumelo-controld-arm64 ./cmd/controld`
- live `T4 192.168.71.243` runtime update：
  - `/usr/bin/playbackd` 已替换并重启
  - `/usr/bin/controld` 已替换并重启
  - `playbackd.service / controld.service` 均为 `active`
  - `/healthz = ok`
  - API 触发 `play_history` 到 `6676b445f84728f2` 后返回 `quiet_active`
  - queue entry count 保持 `11 -> 11`
  - current queue entry 被 resolver enrich 为 `Even Heaven`
  - 首页 SSR 中 `播放记录 · 当前播放` 只出现 1 行
  - 首页 SSR 中未出现 `接下来播放` / `队列序号`
