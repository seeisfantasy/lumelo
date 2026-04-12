# 2026-04-12 `v15` 真机问题清单

适用范围：
- Rootfs: `lumelo-t4-rootfs-20260412-v15.img`
- Android APK: `lumelo-android-provisioning-20260412-classicnamefix-debug.apk`
- 记录日期：`2026-04-12`

用途：
- 汇总本轮真机测试已经坐实的结论
- 作为后续修复和回归的工作清单
- 后续问题全部修完后，可按日期和版本清理归档

追记：
- `2026-04-12` 晚些时候，清单中的两项高优先级问题已经完成代码修复：
  - 板子蓝牙冷启动自动 bring-up
  - 手机 APK 的 WebView 切网恢复崩溃
- 相关产物分别为：
  - Rootfs: [lumelo-t4-rootfs-20260412-v16.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img)
  - APK: [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
- 本文仍保留，作为 `v15` 现场验出问题的原始清单；后续回归应以 `v16 + webviewpollfix` 为主。

## 1. 板子蓝牙

### 已确认结论
- 外接天线是必需条件。未接天线时，之前多轮“扫不到蓝牙”的判断被物理变量干扰。
- 经典蓝牙链路可用。官方金样和 Lumelo 图上都已验证到手机可发现板子经典蓝牙设备名。
- BLE 在真机上没有被稳定验证出来。官方金样下经典蓝牙可见，但手机 `Raw BLE Scan` 仍未看到稳定可用的 BLE 广播。
- 经典蓝牙 `RFCOMM` 配网主链已跑通。手机可以连接 T4 并读取 `device_info`。

### 已验出的 bug
- `v15` 开机后蓝牙并不会自动进入可连接状态。
- 根因在 [bluetooth-uart-attach](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/bluetooth-uart-attach)：
  `btmgmt info` 在“0 个控制器”时也可能返回成功，脚本把它误判成“控制器已就绪”，导致 `hciattach.rk` 被跳过。

### 待修事项
- `bluetooth-uart-attach` 的控制器就绪判断修复已打进 `v16`，下一步重点改为真机确认。
- 做一次无人工干预冷启动验证：
  冷启动后手机应能直接扫描到板子经典蓝牙并成功连接。
- 再补一轮蓝牙 patch 命中核查，确认运行态与官方金样一致。

## 2. 板子 Wi-Fi

### 已确认结论
- 官方金样的正确底座是 `bcmdhd + /system/etc/firmware vendor firmware`。
- Lumelo 早前错误地偏到了 `brcmfmac` 路线，这已经被纠正。
- 当前 Lumelo 图上，经典蓝牙下发 Wi-Fi 凭据并让 T4 成功入网已经真机跑通。
- 现场验证热点：
  `SSID=isee_test`
  `password=iseeisee`
- 成功拿到 Wi-Fi 地址：
  `192.168.43.170`
- 当前板子实际运行时走的是 `wpa_supplicant` 后备链，不是 `NetworkManager`。

### 已验出的 bug / 风险
- 当前成功入网是在手工拉起蓝牙服务后达成，不代表 `v15` 冷启动默认链路就完全没问题。
- 当前板子同时持有有线和 Wi-Fi 地址，后续页面显示和主访问地址选择还要继续观察。

### 待修事项
- 把蓝牙冷启动修复并重新回归“开机 -> 蓝牙连接 -> Wi-Fi 下发 -> connected”整链。
- 补家庭路由器场景验证，不只测手机热点。
- 验证重启后 Wi-Fi 自动回连。
- 验证双网卡 / 双 IP 下首页、配网页、状态页的地址展示是否合理。

## 3. 手机 APK

### 已确认结论
- 经典蓝牙扫描、连接、读取 `device_info`、发送 Wi-Fi 凭据，这四步已经真机跑通。
- `Lumelo Scan` 现在走经典蓝牙主通道，方向正确。

### 已修 bug
- 设备名事件归并问题：
  经典蓝牙名字有时不是首次发现就带出，App 之前会漏掉。
- 名字过滤大小写问题：
  板子对外名字实际是小写 `lumelo`，App 之前只认大写前缀，导致系统蓝牙能看到但 App 内扫不到。

### 已修 bug
- WebView 切网恢复时的崩溃问题已修复。
- 根因在 [MainInterfaceActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainInterfaceActivity.java)：
  `ConnectivityManager.NetworkCallback` 运行在 `ConnectivityThread`，旧实现直接在该线程更新 `TextView`，触发 `CalledFromWrongThreadException`。
- 修复后，恢复逻辑统一切回主线程执行。
- 真机回归已确认：
  - 断开手机 Wi-Fi 后，App 会停留在错误页并显示 `net::ERR_INTERNET_DISCONNECTED`
  - 重新切回与 T4 同一热点 `isee_test` 后，WebView 会自动恢复
  - 已实际恢复到 `http://192.168.43.170:18080/library`

### 已验出的 bug / 风险
- 手机恢复网络时，系统有时会优先自动连回其他已保存 Wi-Fi，例如家庭路由器 `iSee`，而不是与 T4 同一热点。
- 在这种情况下，App 现在不会崩溃，但仍会停留在错误页，直到手机重新回到与 T4 可互通的网络。

### 其他风险
- 当前 Wi-Fi 凭据通过经典蓝牙以明文 JSON 发送。
- 该方式适合开发 bring-up，不适合作为正式版最终安全方案。

### 待修事项
- 继续优化“恢复到错误 Wi-Fi”时的提示与引导，例如更明确提示当前手机所连 SSID 与 T4 不可互通。
- 继续观察不同 Android 机型在切网后的网络回调一致性，确认自动恢复逻辑是否还需要增加补偿轮询。
- 后续补配网安全方案：
  至少补配对后加密依赖或应用层会话保护。

## 4. 其他周边问题

### 已确认结论
- 主界面 `/` 正常。
- 曲库页 `/library` 正常。
- 配网页 `/provisioning` 正常。
- 播放区不是独立页面，而是嵌在首页 `/` 中。
- 播放控制基础链路可用：
  `status`
  `ping`
  `/events/playback`

### 已验出的 bug / 缺口
- 曲库当前仍为空：
  `Volumes=0`
  `Albums=0`
  `Tracks=0`
- 目前只验证到了播放控制接口可达，没有做真实音频播放回归。

### 待修事项
- 上真实曲库做一次完整回归：
  索引、列表、队列、播放、暂停、切歌。
- 把这轮真机验证沉淀成固定回归清单，至少覆盖：
  冷启动蓝牙
  经典蓝牙扫描连接
  Wi-Fi 入网
  WebUI 打开
  曲库
  播放
- 出包后增加“无手工干预上板验证”，避免出现镜像代码已修但现场仍要手工拉服务的假通过。

## 当前修复优先级
1. `v16` 上板后确认板子蓝牙冷启动自动 bring-up 是否真机闭环。
2. 继续优化手机连回错误 Wi-Fi 时的提示与恢复引导。
3. 真实曲库与真实播放回归。
4. 配网安全加固。
