# Lumelo 项目交接记忆文件

## 1. 定位

本文件只做新窗口快速接手摘要。

它不替代：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)

新窗口先读：

1. [docs/README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md)
2. 本文件
3. [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
4. 当前任务对应专项文档

## 2. 当前项目边界

当前正式目标：

- `V1`
- `Local Mode`
- `headless` 本地音频系统

steady-state 主交互：

- Ethernet / Wi-Fi 下的 WebUI

Android APK 当前只是：

- classic Bluetooth / Wi-Fi provisioning 工具
- 联网后的 WebView 壳
- 板端异常时的诊断入口

APK 不是：

- 主播放器 App
- 主曲库浏览 App
- steady-state 主控制端

## 3. 当前工作区与环境

仓库：

- `/Volumes/SeeDisk/Codex/Lumelo`

当前默认开发方式：

- macOS 负责编辑、Android 真机、驱动 OrbStack
- `OrbStack / lumelo-dev` 负责 Linux 构建、rootfs 制镜、离线 gate
- T4 真机负责无线、蓝牙、SSH、ALSA、启动链、真实浏览器 / APK 验证

T4 开发镜像默认 SSH：

- user: `root`
- password: `root`

Android 工具：

- `adb`: `/Users/see/Library/Android/sdk/platform-tools/adb`
- Android Studio JBR:
  - `/Applications/Android Studio.app/Contents/jbr/Contents/Home`

## 4. 重要协作规则

默认不要主动出新 `image/img`。

当前规则：

- 用户明确说：
  - `出包`
  - `出 image`
  - `出 img`
  - `打镜像`
  才生成新的 T4 rootfs `image/img`。
- 其他情况优先在线修：
  - runtime update
  - live T4 验证
  - APK reinstall
  - WebUI / daemon / helper 小步验证
- 如果判断必须出包才能解决，要先说明：
  - 为什么在线修不够
  - 不出包的风险
  - 建议用户下达出包命令

这条已写入：

- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)

## 5. 当前 live T4 状态

最近可用 live 板：

- `T4 192.168.71.12`
- Wi-Fi 连接
- MAC: `C0-84-7D-1F-37-C7`
- WebUI: `http://192.168.71.12/`
- mDNS WebUI: `http://lumelo.local/`
- SSH: `root@192.168.71.12`

最近 live runtime 验证结果：

- `bluetooth.service = active`
- `lumelo-bluetooth-provisioning.service = active`
- `lumelo-wifi-provisiond.service = active`
- `http://192.168.71.12/healthz`
  - `status=ok`
  - `interface_mode=wifi`
  - `playback_available=true`
  - `playback_state=stopped`
  - `provisioning_state=connected`
- 当前 USB DAC：
  - `iBasso-DC04-Pro`
  - ALSA device: `plughw:CARD=iBassoDC04Pro,DEV=0`
- `playbackd` 当前 runtime update：
  - 不再由 `playbackd.service` 固定 `LUMELO_AUDIO_DEVICE`
  - 每次开始播放前自动选择当前唯一 USB Audio DAC
  - 未发现 USB Audio DAC 时返回 `audio_output_unavailable`
  - 多 USB Audio DAC 时返回 `audio_output_ambiguous`
- 首页播放列表区已 runtime update 为 `播放历史`：
  - 最近播放在上方
  - 不提前暴露 shuffle / 伪随机后续队列
  - 每条历史曲目有 `播放` 按钮，走 `play_history`
  - `play_history` 已修正为 `Play Now` 语义：替换当前播放位，不 append 到队尾，不重建后续 `play_order`
  - stopped 状态不把保留的 `current_track` 标成 `当前播放`
  - 同一 track 多次出现在历史里时，只标最近一条匹配项为 `当前播放`
  - history title/path 会用曲库 snapshot enrich
- 首页 / 曲库 transport 中间按钮已改为状态驱动：
  - 播放中显示 `暂停`
  - stopped / paused / 无活动播放显示 `播放`
  - 首页 hero 和曲库 mini player 都保留 `停止`
- 曲库页已补：
  - 无封面专辑默认方形 placeholder
  - 底部 mini player 曲名 / 路径单行省略，防止长文本撑高
  - `本地介质 / 挂载与扫描` section
    - 显示当前 TF / USB device path、mountpoint、fstype、volume uuid
    - 支持刷新设备、挂载、扫描此介质、扫描所有已挂载介质、选择目录扫描、同步挂载状态
    - 扫描类命令在 `pre_quiet / quiet_active` 时由 `controld` 拦截，避免播放期启动 `media-indexd`
  - live `T4 192.168.71.12` 已 runtime update 验证：
    - `/api/v1/library/media` 返回当前 U 盘 `/dev/sda1 -> /media/9cf4bd76f4bd52ee`
    - `/library` SSR 已出现本地介质管理入口

## 6. 最近钉死的 classic Bluetooth 根因

这轮不是 APK scan 问题。

已验证真根因有两层：

1. `lumelo-bluetooth-provisioning-mode`
   - 旧 `bluetoothctl` batch 路径不可靠。
   - 裸跑 `btmgmt` 在 systemd/no-TTY 条件下会挂住。
   - 已改为 pseudo-tty wrapper：
     - `script -q -c 'btmgmt ...' /dev/null`
2. `bluetooth.service`
   - 未以 `bluetoothd -C` 启动时，`sdptool browse local / sdptool add SP` compat SDP 路径不可用。
   - 已新增 drop-in：
     - `/etc/systemd/system/bluetooth.service.d/30-lumelo-compat.conf`
     - `ExecStart=/usr/libexec/bluetooth/bluetoothd -C`

相关代码：

- [lumelo-bluetooth-provisioning-mode](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-bluetooth-provisioning-mode)
- [30-lumelo-compat.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/systemd/system/bluetooth.service.d/30-lumelo-compat.conf)
- [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
- [lumelo-wifi-provisiond.service](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/systemd/system/lumelo-wifi-provisiond.service)
- [lumelo-bluetooth-provisioning.service](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/systemd/system/lumelo-bluetooth-provisioning.service)

## 7. APK 当前状态

最近真机手机：

- `PJZ110`
- `BKQ-AN90`
- Android 16

已验证：

- 最新 debug APK 已能：
  - `SCAN FOR LUMELO`
  - 看到 `[LAST] [NAME] Lumelo T4 (C0:84:7D:1F:37:C7)`
  - 点 `CONNECT`
  - 收到 `device_info`
  - 点 / 自动 `READ STATUS`
  - 回读 `advertising` payload
- 2026-04-25 本轮在 `BKQ-AN90 / Android 16` 复验：
  - `OPEN WEBUI` -> `http://192.168.71.7:18080/`
  - `OPEN PROVISIONING` -> `http://192.168.71.7:18080/provisioning`
  - `OPEN LOGS` -> `http://192.168.71.7:18080/logs`
  - `OPEN HEALTHZ` -> `http://192.168.71.7:18080/healthz`
- 2026-04-26 本轮在 `PJZ110 / Android 16` 真实执行 classic Bluetooth Wi-Fi provisioning：
  - 手机当前 Wi-Fi：`iSee`
  - APK `SCAN -> CONNECT -> USE CURRENT WI-FI -> SEND WI-FI CREDENTIALS` 已跑通
  - T4 `wlan0` 获得 `192.168.71.243/24`
  - APK 自动进入 `http://192.168.71.243:18080/`
  - 手机侧 `curl http://192.168.71.243:18080/healthz` 返回 `provisioning_state=connected`
- 之前“选中设备后没有进入 connect，像 APK UI/state bug”的判断已撤销。
  - 真实原因是 `CONNECT` 按钮在列表下方，需要滚动到 `Selected:` 后面。

当前 APK 还保留的未闭环：

- `ack timeout / write failed / reconnect`
  还没专门人为诱发完整闭环。
- `MainInterfaceActivity` Android 16 back gesture 手感还没单独验证。

APK 最近验证命令：

```sh
cd /Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning
JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew :app:assembleDebug
JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew :app:lintDebug
```

APK 输出路径：

- `/tmp/lumelo-android-build/app/outputs/apk/debug/app-debug.apk`

## 8. WebUI 当前状态

当前 WebUI 方向：

- mobile-first
- 浅色系
- `music player first`
- diagnostics second

已做：

- 首页：
  - 封面 / 当前曲目 / controls / queue / recent albums
  - 已补播放模式 controls：
    - `顺序 / 随机`
    - `不循环 / 单曲 / 列表`
  - 命令成功后不再显示 `PLAY_HISTORY -> state=...` 这类 raw IPC ack。
  - hero 已补当前曲目音频格式行，例如 `PCM · FLAC · 48 kHz` / `DSD64 · DFF · 2.8224 MHz`。
  - `media-indexd` 已补 DFF / DSF header fallback，缺 tag/properties 时仍能索引 DSD sample rate。
- 曲库：
  - album-first
  - 最近专辑 / album detail / tracklist
- 设置：
  - `配网` 文案逐步收为 `设置 / 网络与设备`
  - 已补 V1 当前解码器展示：
    - 未发现 USB Audio 解码器时显示 `未连接`
    - 发现 USB Audio 解码器时显示解码器名称
  - live `T4 192.168.71.7` 已 runtime update 验证：
    - `/api/v1/system/audio-output = 200`
    - 当前现场无 USB Audio 解码器，设置页显示 `未连接`
- 顶部和首页入口：
  - `首页 / 曲库 / 设置 / 日志`

尚未最终闭环：

- 真实专辑、真实封面、真实队列数据态下的视觉。
- 真 USB DAC 插入 / 拔出后的设置页显示与 `connected=true` 状态。
- 曲库 `本地介质` section 的真实扫描按钮尚未人工点击验证；当前只做了只读设备发现和 SSR 验证。
- 最新镜像刷入后的 Safari 真机复验。
- 真实播放 smoke 的最新镜像回归。
- 真实曲目播放到队尾后的 `repeat_mode=one/all` 自动续播行为，本轮只做了 API/UI 和 playbackd 单元验证。
- 后续新增 DFF / DSF 文件的全量扫描路径已由单元测试覆盖，但尚未在真机重新跑一次完整 `scan-mounted`。

## 9. 当前 image / 出包状态

最近已产出的 checkpoint image：

- [lumelo-t4-rootfs-20260425-v23.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260425-v23.img)
- [lumelo-t4-rootfs-20260425-v23.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260425-v23.img.sha256)
- `sha256 = 33bd44deeb630544327aa1ef28544e65ecb44b41d63e3adaaf793c30e5618c4e`

`v23` 包含：

- pseudo-tty `btmgmt` wrapper
- `bluetoothd -C` compat drop-in
- verify script 对上述两项的离线检查

已验证：

- `verify-t4-lumelo-rootfs-image.sh = 0 failure(s), 0 warning(s)`
- `compare-t4-wireless-golden.sh = 0 failure(s), 0 warning(s)`

重要：

- `v23` 已存在，但不要主动催用户刷。
- 默认继续在线修。
- 只有用户明确说出包或刷某个 image 时，再进入镜像交付 / 烧录链。

## 10. 当前文档整理状态

2026-04-25 已做文档整理：

- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
  - 已删除 `v20` 之前过程记录。
  - 只保留 `v20` 之后的压缩进展。
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 已写入“用户明确下令才出包”的规则。
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - 已更新 classic Bluetooth 当前板端要求：
    - `bluetoothd -C`
    - pseudo-tty `btmgmt`
    - service lifecycle binding
    - SDP cleanup
- 本文件已重写为当前交接入口。

## 11. 新窗口建议下一步

如果用户没有新指令，建议按这个顺序继续：

1. 继续在线验证 live `T4 192.168.71.12`。
2. 补 APK recovery 分支专门现场回归：
   - `ack timeout`
   - `write failed`
   - `auto reconnect`
3. 若继续板端 classic provisioning 改动：
   - 先 runtime update / live 验证。
   - 不主动出新 image。
4. 若用户明确说“刷 v23”或“出包”，再按 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md) 做 bring-up。
