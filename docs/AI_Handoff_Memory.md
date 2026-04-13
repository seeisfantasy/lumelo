# Lumelo 项目交接记忆文件

## 1. 这份文件的定位

本文件只做“窗口交接压缩摘要”。

它不替代：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)

用法：

- 新窗口先靠本文件快速进入状态
- 再去看上面 3 份权威文档补细节

## 2. 新窗口开始前先读哪些文档

建议按这个顺序读：

1. [README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md)
2. [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
3. [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
4. [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
5. [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
6. [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
7. [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
8. [apps/android-provisioning/README.md](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/README.md)
9. [Real_Device_Findings_20260412_v15.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Real_Device_Findings_20260412_v15.md)
   - 只在需要回看 `v15` 原始现场问题时再读

## 3. 当前项目位置与工作区

当前仓库路径：

- `/Volumes/SeeDisk/Codex/Lumelo`

当前推荐活跃工作区：

- `/Volumes/LumeloDev/Codex/Lumelo`

说明：

- `SeeDisk` 是 `exFAT`
- `LumeloDev` 是 `APFS sparsebundle`
- 真正重负载出包统一在：
  - `OrbStack / lumelo-dev`
  - Linux 原生临时目录 `/var/tmp/lumelo-<tag>/`

## 4. 当前环境与工具状态

### 4.1 macOS / OrbStack

- 宿主机：`macOS / arm64`
- 默认 Linux 开发机：`lumelo-dev`
- 出包规则已固定在：
  - [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - `7. T4 rootfs 出包运行手册`

### 4.2 Android 环境

- Android Studio：
  - `/Applications/Android Studio.app`
- SDK：
  - `/Users/see/Library/Android/sdk`
- `adb` 经常不在默认 `PATH`
- 需要时优先用：
  - `/Users/see/Library/Android/sdk/platform-tools/adb`

### 4.3 当前真机抓手

- T4 WebUI：
  - `http://<T4_IP>:18080/`
  - `/healthz`
  - `/provisioning-status`
  - `/logs`
  - `/logs.txt`
- 若 SSH 不可用，不要先慌，优先走上面这些 Web 入口

## 5. 当前主线已经变成什么

现在项目主线已经不再是 BLE bring-up。

已经得出的关键结论：

- 板子外接天线是必需条件
- 这块 T4 的经典蓝牙是可用的
- BLE 在现场没有被稳定验证成可用主链
- 因为蓝牙只承担前期配网，所以主通道已经切到：
  - 经典蓝牙 `RFCOMM`
- `Raw BLE Scan` 现在只保留为诊断能力

一句话总结：

- 当前真正主线是：
  - `经典蓝牙配网 + Wi‑Fi 入网 + APK 内 WebView + 后续曲库/播放真机回归`

## 6. 最近这几轮真正做成了什么

### 6.1 官方金样比对已经完成

已经用官方 `NanoPC-T4` 金样做过静态与运行态比对。

结论：

- 正确无线底座是：
  - `bcmdhd`
  - `/etc/firmware/BCM4356A2.hcd`
  - `/system/etc/firmware/fw_bcm4356a2_ag.bin`
  - `/system/etc/firmware/nvram_ap6356.txt`
- 我们之前偏到了 `brcmfmac` 路线
- 这条偏差已经修回官方金样

### 6.2 经典蓝牙配网主链已真机打通

在 `v15` 真机上，以下链路已经实际跑通过：

- 手机能扫描到板子经典蓝牙
- 手机能连接板子经典蓝牙
- `device_info` 能正常返回
- Wi‑Fi 凭据能通过经典蓝牙下发
- T4 能实际入网

现场已跑通的热点样例：

- `SSID = isee_test`
- `password = iseeisee`

当时 T4 成功拿到：

- `192.168.43.170`

`2026-04-12` 现场补测又确认了一次更关键的兼容性问题：

- 在 `PJZ110` 这台 Android 真机上
  - 系统蓝牙设置能看到 `lumelo`
  - App 也能选中 `lumelo`
  - 但标准 `SPP UUID` 连接路径会失败
- 根因已经定位到 Android 端 `SPP / SDP` 兼容性：
  - 蓝牙栈日志可见 `scn: 0`
  - `channel: -1`
- 源码现已补上固定 `RFCOMM channel 1` fallback：
  - [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)
- 同一台手机已经重新确认完整闭环：
  - `CONNECT`
  - `device_info`
  - `status`
  - `wifi_credentials`
  - `apply`
  - APK 内 `WebView`
- 板子本轮再次拿到：
  - `192.168.43.170`
- 手机同网段验证：
  - `/healthz`
  - `/provisioning-status`
  - `/`
  - `/library`
  均可正常访问

### 6.3 APK 的 WebView 切网恢复 bug 已修

真实修掉过两个阶段的问题：

第一阶段：

- 修掉了切网恢复时的崩溃
- 根因是 `ConnectivityThread` 里直接改 UI，触发 `CalledFromWrongThreadException`

第二阶段：

- 又补了一层错误页下的网络状态补偿轮询
- 这样某些 Android 机型即使网络回调不稳定，也会周期性重评网络状态
- 一旦回到与 T4 可互通的网络，会主动重试恢复

当前状态：

- 回到与 T4 同一热点后，WebView 已能自动恢复
- 但如果手机自动连回别的已保存 Wi‑Fi，例如 `iSee`
  - App 不会崩
  - 但仍会停留在错误页
  - 直到手机重新回到与 T4 可互通的网络

### 6.4 板子蓝牙冷启动 bug 已修入新图

此前在 `v15` 上，经典蓝牙链能通，但需要手工拉服务。

根因已明确：

- [bluetooth-uart-attach](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/bluetooth-uart-attach)
- `btmgmt info` 在“0 个控制器”时也可能返回成功
- 脚本把它误判成“蓝牙已就绪”
- 结果 `hciattach.rk` 被跳过

这个修复已经打进了新图 `v16`。

## 7. 当前最新产物

### 7.1 最新 rootfs

- [lumelo-t4-rootfs-20260412-v16.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img)
- [lumelo-t4-rootfs-20260412-v16.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img.sha256)

SHA256：

- `ea6d85c85335fa736ac73cf678456122a319a886d98277f88bdbeebeb8e7c160`

这版已完成：

- `verify-t4-lumelo-rootfs-image.sh`
  - `0 failure(s), 0 warning(s)`
- `compare-t4-wireless-golden.sh`
  - `0 failure(s), 0 warning(s)`

### 7.2 最新 APK

- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256)

SHA256：

- `acd72ee79d511193df76e4e3a716b992dd714531517446e274d84cc01ea3982c`

当前最新 APK 已包含：

- 经典蓝牙扫描
- 名称更新归并
- 名字大小写不敏感过滤
- `RFCOMM` 配网会话
- 经典蓝牙 `hello.security` 协商
- `wifi_credentials_encrypted`
- `device_info`
- Wi‑Fi 凭据下发
- `WebView` 切网恢复主线程修复
- 错误页下的网络状态补偿轮询

补充说明：

- 当前源码还额外包含：
  - 固定 `RFCOMM channel 1` fallback
- 这部分已现场 `assembleDebug + adb install` 真机验证通过
- 这轮还额外完成了现场部署验证：
  - `PJZ110` 已安装最新 debug APK
  - 真机 T4 已覆盖部署新版
    [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
  - App debug log 已明确看到：
    - `Classic Bluetooth credential security negotiated: dh-hmac-sha256-stream-v1`
    - `hello` 中真实带有 `security`
  - 说明新版安全握手不只是代码存在，而是已在现场真机上跑起来
- 后续又继续完成：
  - App / 板端的明文回退移除
  - 在线部署脚本
    [deploy-t4-runtime-update.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/deploy-t4-runtime-update.sh)
    已落仓
  - 使用新版脚本把板端 daemon 直接热更新到 `192.168.1.120`
  - 去掉明文回退后，又重新用 `isee_test / iseeisee` 真机跑到 `connected`
- 但还没有单独归档新的命名 APK 产物

## 8. 当前仓库与文档已更新到什么状态

这轮已经同步更新过的文档：

- [README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md)
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
- [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)
- [Real_Device_Findings_20260412_v15.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Real_Device_Findings_20260412_v15.md)
- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)

这轮文档收口后，当前约定已经变成：

- `docs/` 顶层只保留仍在使用的主文档
- `docs/README.md` 作为统一索引与读法入口
- 原 `Bluetooth_WiFi_Provisioning_MVP.md` 已改名为 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
- 历史 MVP、旧提案和阶段性 checklist 统一放进 `docs/archive/`
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 后半段已进一步减重，只保留长期原则，不再重复维护环境和 bring-up 操作细节

因此新窗口拿到的仓库与文档，当前已经是最新状态。

## 9. 当前已验证事实与真正未闭环事项

### 9.1 已验证事实

下面这些点已经做成，不应再作为当前待办：

- 经典蓝牙配网主链已在真机重新闭环：
  - `SCAN / SELECT / CONNECT / device_info / apply / WebView`
- Android 端 `SPP / SDP` 兼容性已补：
  - 固定 `RFCOMM channel 1` fallback 已落地
- 经典蓝牙凭据传输已切到非明文：
  - App 发送 `wifi_credentials_encrypted`
  - 明文 `wifi_credentials` 回退已移除
- 板端 `wpa_supplicant` 主路径已不再落明文密码：
  - daemon 只保留 `wpa_psk_hex`
  - `/etc/wpa_supplicant/wpa_supplicant-wlan0.conf` 只写 `psk=<64hex>`
- 上面这条已经做过现场直查验证：
  - `grep 'iseeisee\\|#psk'` 为空
  - `wpa_cli -i wlan0 status` 返回 `wpa_state = COMPLETED`
- Android 端扫描兼容性已补两层兜底：
  - remembered device 回填
  - 首次扫描保留 `[CLASSIC]` 候选并优先显示明确 `nameMatch`
- `v16` 的板子侧回归已补齐一轮现场真机验证：
  - 冷启动后确认仍是 `sd` 引导：
    - `/proc/cmdline` 含 `storagemedia=sd`
    - `who -b` / `uptime` 可见新启动已发生
  - 开机后无需手工拉服务：
    - `bluetooth.service`
    - `lumelo-wifi-provisiond.service`
    - `wpa_supplicant@wlan0.service`
    都会自动进入 `active`
  - 冷启动后手机无需手工拉服务就能重新：
    - 扫到 `Lumelo T4`
    - `CONNECT`
    - 收到 `hello / device_info`
  - 重启后自动回连 `isee_test` 已验证通过
  - 双网卡 / 双 IP 的状态页与 JSON 已完成现场验证：
    - 热点场景：
      - `wired_ip = 192.168.1.120`
      - `wifi_ip = 192.168.43.170`
    - 家庭路由器场景：
      - `wired_ip = 192.168.1.120`
      - `wifi_ip = 192.168.1.121`
  - 家庭路由器 `iSee` 场景也已验证通过：
    - 板子能在经典蓝牙配网后切到 `iSee`
    - Mac / 手机 / T4 最终处于同一 `192.168.1.x` 网段
    - `/provisioning-status` 与首页都能正确显示：
      - `ssid`
      - `wifi_ip`
      - `wired_ip`
      - `all_ips`
- `controld` 已切到纯 Go SQLite 驱动：
  - 板端 `/library` 不再因 `CGO_ENABLED=0` 落到 sqlite stub
  - 当前家庭路由器场景下，Mac 访问 `http://192.168.1.121:18080/library` 已能看到真实条目
- `/run/lumelo` 运行目录已恢复稳定：
  - `lumelo-wifi-provisiond.service` 已补 `RuntimeDirectoryPreserve=yes`
  - `playback_cmd.sock` / `playback_evt.sock` 已重新出现
- 真实曲库索引与 `ALSA hw` 最小闭环已经做过现场 smoke：
  - 板端生成真实 `WAV`
  - `media-indexd scan-dir /var/lib/lumelo/test-media` 已写入 `library.db`
  - `/library` 现可见：
    - `1 volume`
    - `1 album`
    - `1 track`
  - `aplay -D default` 已成功播放这首真实 `WAV`
  - 新增板端 helper：
    - `/usr/bin/lumelo-media-smoke`
    - 已现场验证：
      - `lumelo-media-smoke list --first-wav`
      - `lumelo-media-smoke play --first-wav`
      - `lumelo-media-smoke smoke --skip-play`
- `playbackd` 已接入真机最小真实输出：
  - 当前使用 `library.db` 按 `track_uid` 解析真实媒体路径
  - 当前板端输出实现已经分成两条：
    - `wav` 直通：
      - `playbackd -> aplay -D default <file> -> ALSA hw`
    - 第一版非 `wav` 解码：
      - `playbackd -> symphonia decode -> aplay -t raw -f S16_LE -c <channels> -r <sample_rate> -> ALSA hw`
  - 当前真机已验证通过的格式：
    - `wav`
    - `m4a/aac`
    - `flac`
    - `mp3`
    - `ogg`
  - 板端 helper 已补充自动回归命令：
    - `lumelo-media-smoke regress-playback --timeout 8`
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format flac`
    - 长时长压缩格式可用：
      - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format mp3 --skip-mixed`
      - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format ogg --skip-mixed`
    - 如果板子上已经有多个 indexed volume：
      - 可加：
        - `--mount-root /var/lib/lumelo/test-media`
      - 避免 helper 误选到批量回归生成的另一棵测试树
  - 板端批量扫描 helper 已落地：
    - `lumelo-media-smoke regress-library-scan`
    - 当前会在：
      - `/var/lib/lumelo/test-media-batch`
      生成独立 fixture 根目录
    - 当前已现场验证通过：
      - `tracks = 5`
      - `directories = 3`
      - `formats = ["flac", "m4a", "mp3", "ogg", "wav"]`
      - `albums = 3`
      - `covered_tracks = 4`
      - `artwork refs = 2`
    - helper 现在还会同时验证：
      - `Album Alpha/folder.jpg` 优先于 `cover.jpg`
      - `Album Beta/cover.jpg` 能正常生成 `thumb/320`
      - `Album Gamma` 保持无封面
  - `controld /library` 已不再只是显示 `thumb_rel_path` 文本
    - 现在新增：
      - `/artwork/...` 只读路由
      - 专辑列表真实 `<img>` 缩略图渲染
    - 现场已验证：
      - `curl -I http://192.168.1.121:18080/artwork/thumb/320/...jpg`
      - 返回：
        - `200 OK`
        - `Content-Type: image/jpeg`
- 真机已验证通过的 `playbackd` 控制链：
  - `Play`
  - `Pause`
  - `Play same track` 作为 `Resume`
  - `Stop`
  - `Queue Append + Next`
  - `Prev`
  - `Play History`
  - `m4a -> wav` 混合队列下的自动切歌
  - 长时长 `mp3 -> wav` 混合队列下的自动切歌
  - 长时长 `ogg -> wav` 混合队列下的自动切歌
  - 单轨自然播完后自动回到 `stopped`
- 首页默认 `track id` 已改成：
  - 若当前已有播放曲目，用当前曲目
  - 否则自动填充曲库中第一首已索引轨道
- `/` 页的播放事件 SSE 路径已修正：
  - 浏览器端现在会订阅 `/events/playback`
  - 不再是之前那种双重引号路径

当前只需记住一个现场说明：

- 若直接在板子 shell 调 `lumelo-wifi-apply`
  - `wlan0` 可以已经连上
  - 但 `/provisioning-status` 仍可能显示 `advertising`
  - 因为这次没走 daemon 的状态机

### 9.2 板子侧仍未闭环

- 本轮原先列出的板子侧验证项已经补齐
- 当前只剩一个硬件启动边界仍需单独记住：
  - 这块板子若不按键，默认会进 `eMMC`
  - 因此“无人工干预冷启动进入调试 `sd` 系统”本身还不成立
  - 当前已验证的是：
    - 在人工按键选择 `sd` 启动后
    - `v16` 系统内的蓝牙、Wi‑Fi、自恢复与状态展示都能自动起来

### 9.3 手机 APK 仍未闭环

- 手机自动连回其他已保存 Wi‑Fi 时
  - App 不会崩
  - 但提示与恢复引导还不够好
- 经典蓝牙首配仍有一个边界：
  - 如果是一台从未成功连接过的新手机
  - 且系统整轮 inquiry 完全不给稳定 `nameMatch`
  - App 现在至少会展示 `[CLASSIC]` 候选，不再是 `0 设备`
  - 但用户仍可能需要在匿名 classic 候选里手工判断一次

### 9.4 安全尾项

- 明文密码在“解密完成 -> 派生 WPA PSK”之间，仍会短暂存在于进程内存
- 若未来启用 `NetworkManager` 作为主 Wi‑Fi 后端，需要补一条等价的非明文凭据写入方案

### 9.5 业务功能仍未闭环

- 主界面 `/`、曲库 `/library`、配网页 `/provisioning` 都已能打开
- 真实曲库索引、`ALSA hw` 最小 smoke、以及 `playbackd` 的 `wav + m4a/aac + flac + mp3 + ogg` 真机输出都已起步并通过
- tagged 元数据真机回归也已完成：
  - 新增一套真实标签 fixture：
    - `Northern Signals`
    - `Transit Lines`
  - 已现场确认：
    - 专辑艺人
    - 曲目艺人
    - 年份
    - 流派
    - `disc_no`
    - `track_no`
    - 目录封面缩略图
    - 从 tagged 曲库直接点播 `mp3` 轨道
- 外部媒体主链也已再往前推进一段：
  - [lumelo-media-import](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-import)
    已支持：
    - `list-mounted`
    - `scan-path`
    - `scan-mounted`
    - `import-device`
    - `reconcile-volumes`
  - 当前已现场验证：
    - 无介质时 `list-mounted = []`
    - 播放期扫描会拒绝
    - loop ISO 模拟块设备可以完成：
      - 挂载
      - 入库
      - 点播
      - 卸载后 reconcile
- 稳定性回归也已补上两条正式板端命令：
  - `lumelo-media-smoke regress-playbackd-restart`
  - `lumelo-media-smoke regress-bad-media`
  - 当前已现场确认：
    - `playbackd` 重启后：
      - `queue_entries` 保持
      - `current_track` 保持
      - 状态会回到 `stopped`
    - 坏文件当前仍会被索引进曲库
    - 但播放坏文件时：
      - 会进入 `quiet_error_hold`
      - `playbackd.service` 不会被拖挂
      - 随后仍能立刻恢复正常播放有效轨道
- 当前真正仍未闭环的只剩：
  - 真外部 TF / USB 介质在场下的热插入 / 热拔出闭环
  - 整机重启后的状态回归
  - 是否要在索引层直接过滤坏文件，避免它们出现在用户曲库里

另外一个新的现状判断：

- 当前板子现场依旧没有真外部 TF / USB 介质插着
- 所以“真硬件插入 -> udev 触发 -> 自动挂载 -> 自动导入 -> 拔出下线”这整条链还没有完成最终真机闭环
- 但在没有真介质的前提下，已经不只是探路：
  - 最小入口命令已落地
  - 模拟块设备导入也已通过

## 10. 测试环境这轮新增的重要事项

- 经典蓝牙测试必须默认认为：
  - 板子外接天线已经接好
- 手机与 T4 若要在 APK 内打开 WebUI，必须处在可互通网络
- 只靠蓝牙能完成配网
- 但 WebView 要打开 `http://<T4_IP>:18080/`，手机和 T4 必须真的在同一可达网络
- 现场测试中，手机若切回 `isee_test`，WebView 已经能自动恢复

另外一个实操事项：

- 当前 shell 里 `adb` 可能不在 `PATH`
- 新窗口如果要直接操作手机，优先先执行：

```sh
export PATH="/Users/see/Library/Android/sdk/platform-tools:$PATH"
```

## 11. 新窗口当前接手顺序

按这个顺序接手最稳：

1. 先看：
   - [docs/README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md)
   - [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
   - [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
   - [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
2. 若要给外部 AI 做静态审查：
   - 直接看 `docs/review/`
3. 若板子当前不在线：
   - 先确认是否掉回默认 `eMMC`
   - 当前调试系统仍需要人工按键选择 `sd`
4. 回到 `sd` 系统后，当前优先只补：
   - 真 TF / USB 介质在场下的热插入 / 热拔出闭环
   - 整机重启后的状态回归
   - 坏文件是否要在索引层直接过滤
5. 上面三项补齐后：
   - 这个阶段的底座和主链基本就可以视为完成
   - 后续主重心再转向 UI、美化与易用性

## 12. 当前里程碑一句话版

当前项目已经不在“蓝牙 bring-up / Wi‑Fi 配网是否能跑通”的阶段，而是进入了：

- 经典蓝牙加密配网已真机跑通
- 真曲库索引、封面缩略图、WebUI 页面和 `playbackd` 真机输出已打通
- `wav + m4a/aac + flac + mp3 + ogg` 都已在板子上验证通过
- 外部媒体最小入口、模拟块设备导入、扫描/播放互斥、`playbackd` 重启恢复、坏文件恢复都已有正式回归入口

现在离“底座完整、主流程能跑、后面主要做 UI/易用性”的里程碑，真正剩下的重点只剩：

- 真 TF / USB 热插入 / 热拔出闭环
- 整机重启后的状态回归
- 坏文件索引策略是否要继续收严
