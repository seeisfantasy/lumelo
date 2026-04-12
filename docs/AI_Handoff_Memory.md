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

当前只需记住一个现场说明：

- 若直接在板子 shell 调 `lumelo-wifi-apply`
  - `wlan0` 可以已经连上
  - 但 `/provisioning-status` 仍可能显示 `advertising`
  - 因为这次没走 daemon 的状态机

### 9.2 板子侧仍未闭环

- `v16` 还没有做“无人工干预冷启动”真机回归
- 需要确认：
  - 经典蓝牙是否开机自动起来
  - 手机是否无需手工拉服务就能扫描并连接
- 需要补家庭路由器场景，不只测手机热点
- 需要补重启后 Wi‑Fi 自动回连
- 需要补双网卡 / 双 IP 场景的页面与状态展示验证

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
- 但真实曲库和真实播放回归还没开始
- 当前仍未做：
  - 曲库索引真机回归
  - 真实播放
  - 播放 / 暂停 / 切歌
  - `ALSA hw` 真机音频链

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

## 11. 新窗口第一步该做什么

按这个顺序接手最稳：

1. 先读：
   - 产品手册
   - 开发日志
   - 环境 README
   - APK 进度文档
   - `v15` 真机问题清单
2. 直接使用最新产物：
   - `lumelo-t4-rootfs-20260412-v16.img`
   - `lumelo-android-provisioning-20260412-webviewpollfix-debug.apk`
3. 先上板 `v16`
4. 第一轮真机优先只做：
   - 无人工干预冷启动
   - `Lumelo Scan`
   - `CONNECT`
   - `device_info`
   - `SEND WI-FI CREDENTIALS`
   - `OPEN WEBUI`
5. 若 `v16` 冷启动就能直接扫到并连上：
   - 继续测家庭路由器场景
   - 继续测重启后自动回连
6. 只有在上面都稳后，再切入：
   - 曲库
   - 播放
   - 音频真机链

## 12. 这次最重要的结论，给新窗口一句话版

当前项目已经从“怀疑 BLE / 固件 / 手机兼容性”阶段，推进到了：

- 官方无线底座已纠正
- 经典蓝牙配网主链已真机跑通
- APK 的 WebView 切网恢复已修
- 新的 `v16` 已出好并离线验包通过

现在最关键的下一步，不是再分析方向，而是：

- 把 `v16` 上板
- 验证它是否终于能在无人工干预下，直接完成：
  - 冷启动蓝牙
  - 经典蓝牙连接
  - Wi‑Fi 下发
  - WebUI 打开
