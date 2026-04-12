# Lumelo 手机 APK 进度文档

## 1. 文档用途

本文件单独维护 `Lumelo` 手机 APK 的：

- 当前定位
- 已完成能力
- 当前阻塞点
- 功能结构
- 分阶段开发计划
- 近期验收重点

文档边界：

- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - 维护当前经典蓝牙配网协议、保留 BLE 诊断范围和安全传输契约
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
  - 维护每天真实发生的开发过程
- [archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md)
  - 保留 APK 初版目标和历史 MVP 边界，仅作历史参考
- 本文件
  - 维护“当前 APK 做到了什么、接下来怎么开发、结构如何拆分”

## 2. 当前产品定位

当前手机 APK 不是一个完整播放器，也不是 V1 的 steady-state 主控制端。

当前定位是：

- `BLE + Wi-Fi provisioning` 的手机 setup 工具
- 板端异常时的第一诊断入口
- 配网成功后承载一个 APK 内部 `WebView` 外壳

当前不做的事情：

- 本地曲库浏览主实现
- 主播放控制实现
- 云账号
- 后台同步
- App Store 级产品化打磨

V1 的 steady-state 主交互仍然是：

- Ethernet / Wi-Fi 下的 WebUI

## 3. 当前状态

截至 `2026-04-12`，APK 侧已经有这些能力：

- 经典蓝牙扫描 `Lumelo` 设备
- 经典蓝牙扫描结果合并：
  - 首次 `ACTION_FOUND`
  - 后续 `ACTION_NAME_CHANGED`
- `Raw BLE Scan` 自检入口
- 扫描摘要：
  - 设备总数
  - `UUID matched`
  - `Name matched`
  - 当前选中设备
- 原始扫描结果详情：
  - `MAC`
  - `RSSI`
  - `Local Name`
  - `Device Name`
  - `Service UUIDs`
  - `Manufacturer Data`
- 连接经典蓝牙 `RFCOMM / SPP` provisioning service
- 读取 `device_info`
- 发送 Wi-Fi 凭据
- 触发 `apply`
- 读取 provisioning `status`
- 连接后自动进入 APK 内 `WebView`
- APK 内打开：
  - Home
  - Library
  - Provisioning
  - Logs
  - Healthz
- 页面内 debug log
- 导出诊断日志
- 页面内显示：
  - `App version`
  - `build time`
  - `git short SHA`
- `Use Current Wi-Fi` 预填当前 SSID

当前已知边界：

- 手机侧经典蓝牙扫描链路已作为主通道
- `Raw BLE Scan` 保留为诊断能力，不再承担主配网职责
- 官方金样真机上，APK 已经能扫到：
  - `NanoPC-T4`
  - 但官方系统不提供 `Lumelo` 的 `RFCOMM` provisioning service
  - 因此只能验证“经典蓝牙发现链路”和手机兼容性
  - 不能直接完成 `device_info / credentials / apply / status` 全闭环
- APK 仍以 bring-up / diagnostic 为主，不是最终交付形态
- `WebView` 切网恢复链已经完成一轮真机修复：
  - 断网后会停留在错误页并显示明确文案
  - 重新回到与 T4 同一热点后，会自动恢复页面
- `WebView` 切网恢复链现已补上主线程轮询补偿：
  - 当某些 Android 机型的网络回调不稳定时
  - 错误页会周期性重新评估当前网络状态
  - 一旦手机回到与 T4 可互通的网络，会主动重试恢复
- 但手机系统有时会优先自动连回其他已保存 Wi-Fi
  - 此时 App 不会崩溃
  - 但会继续停留在错误页，直到手机回到与 T4 可互通的网络

## 4. 当前功能结构

从功能上看，当前 APK 可以拆成 5 层：

### 4.1 Setup Shell

负责：

- 权限检查
- 页面状态切换
- 当前环境状态展示
- 用户输入与主流程按钮

### 4.2 蓝牙扫描层

负责：

- 普通 `Lumelo` 经典蓝牙扫描
- `Raw BLE Scan` 自检
- 扫描结果过滤
- 扫描结果列表展示
- 扫描摘要展示
- 原始广播详情展示

当前痛点：

- 详情仍集中在一个 `MainActivity`
- 后续还应拆成：
  - `ClassicBluetoothScanner`
  - `BleScanner`
- 当前结果去重已支持“同一 `MAC` 的发现事件与后续名称更新合并”
- 后续仍可继续优化成更完整的“扫描响应合并”

### 4.3 传输会话层

负责：

- 经典蓝牙连接
- `RFCOMM` socket 建立
- 逐行 JSON 收发
- 状态读取
- 断开连接

当前痛点：

- 逻辑仍集中在一个 `MainActivity`
- 后续应继续把经典蓝牙会话和 BLE 诊断会话拆开维护

### 4.4 Provisioning 流程层

负责：

- 组织 Wi-Fi 凭据 payload
- 写入凭据
- 触发 apply
- 轮询状态
- 根据状态切换 UI

当前痛点：

- 还缺少更完整的 ACK / retry / reconnect 策略
- 不同手机在经典蓝牙配对 / 非配对连接上的兼容性还要继续测

### 4.5 APK 内主界面壳层

负责：

- 在 `connected` 后进入 APK 内 `WebView`
- 提供 Home / Library / Provisioning / Logs 等入口

定位：

- 这是 `WebUI` 的容器
- 不是要在 APK 内重做一套主播放器 UI

## 5. 代码结构建议

当前代码主要集中在：

- [MainActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java)
- [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)

继续沿用单文件会越来越难维护。后续推荐拆成：

- `ClassicBluetoothScanner`
  - 经典蓝牙发现、结果解析、过滤
- `BleScanner`
  - `Raw BLE Scan` 诊断能力
- `ClassicBluetoothTransport`
  - `RFCOMM` 连接、逐行 JSON 收发
- `ProvisioningSession`
  - 凭据发送、状态推进、失败态归一化、重试策略
- `DebugLogStore`
  - 屏内日志、导出日志
- `ProvisioningWebViewActivity`
  - APK 内部 WebView 外壳

当前阶段先不强制大重构，但 `V1 诊断版` 开始建议按这个方向收敛。

## 6. 后续开发计划

### 6.1 APK V1：诊断增强版

目标：

- 优先解决“为什么扫不到 / 连不上 / 卡在哪一步”不可见的问题

建议能力：

- 增加经典蓝牙扫描摘要
- 保留 `Raw BLE Scan`
- 列出全部 BLE 广播结果
- 每条结果显示：
  - `MAC`
  - `RSSI`
  - `Local Name`
  - `Service UUIDs`
  - `Manufacturer Data`
- 页面显示：
  - `App version`
  - `build time`
  - `git short SHA`
- 增加导出诊断日志
- 扫描结果标签区分：
  - `All BLE`
  - `UUID matched`
  - `Name matched`
- 扫描结束后给 summary：
  - 扫到设备总数
  - 是否命中 `Lumelo UUID`
  - 是否命中名称过滤

完成标准：

- 不接 `adb` 也能判断问题是在经典蓝牙发现、BLE 诊断链，还是板端空口广播

### 6.2 APK V2：配网闭环版

目标：

- 把“连接后能否稳定把 Wi-Fi 凭据送到 T4”做扎实

建议能力：

- 连接后先做设备指纹确认
- 显示：
  - `hostname`
  - `ip`
  - `wifi_interface`
- Wi-Fi payload 改成：
  - ACK 驱动
  - 超时重试
  - 断线重连恢复
- 自动轮询 `status`
- 状态明确展示：
  - `connected`
  - `credentials received`
  - `applying`
  - `waiting for dhcp`
  - `connected`
  - `failed`
- 成功后直接显示 WebUI 地址并支持打开

完成标准：

- 长 SSID / 长密码在经典蓝牙通道上稳定传输
- 用户可以明确知道当前卡在哪一阶段

### 6.3 APK V3：交付整理版

目标：

- 从 bring-up 工具整理成可对外使用的手机配网工具

建议能力：

- 把调试入口收进高级诊断页
- 首页只保留最短配网流程
- release signing
- 更清晰的错误提示
- 关键失败事件记录
- 提升不同 Android 机型上的权限提示稳定性

完成标准：

- 非开发者也能独立完成一次配网
- 出问题时也能带回足够诊断信息

## 7. 当前优先级

当前建议顺序：

1. `V1 真机回归：经典蓝牙扫描 + Raw BLE 诊断 + 导出日志`
2. `V2 ACK / retry + 状态闭环`
3. `V3 产品化整理`

原因：

- 当前 APK 最大价值不是“更像正式 App”
- 而是先把经典蓝牙主通道和 BLE 诊断边界查清楚

## 8. 当前验收重点

下一轮 APK / 真机联调，优先看：

1. 手机是否能在 `Lumelo Scan` 中扫到 `Lumelo T4`
2. 手机是否能在 `Raw Scan` 中看到目标广播
3. 连接后是否能稳定完成 `device_info`
4. Wi-Fi 凭据是否成功写入并触发 `apply`
5. `status` 是否能推进到 `connected`
6. APK 内 `WebView` 是否能稳定进入主界面

## 9. 当前 APK 产物

当前 `out/android-provisioning` 目录中已经出现过的 APK 产物包括：

- `lumelo-android-provisioning-20260409-mainui-debug.apk`
- `lumelo-android-provisioning-20260410-bletest-debug.apk`
- `lumelo-android-provisioning-20260411-mtufix-debug.apk`
- `lumelo-android-provisioning-20260412-rawscan-debug.apk`
- `lumelo-android-provisioning-20260412-classicbt-debug.apk`
- `lumelo-android-provisioning-20260412-classicscanfix-debug.apk`
- `lumelo-android-provisioning-20260412-webviewthreadfix-debug.apk`
- `lumelo-android-provisioning-20260412-webviewpollfix-debug.apk`

当前最新 APK 产物是：

- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256)

当前最新 APK 的定位是：

- 仍然属于 bring-up / debug 阶段
- 已包含：
  - 经典蓝牙 `Lumelo Scan`
  - 经典蓝牙扫描的名称更新合并修复
  - WebView 切网恢复时的主线程修复
  - WebView 错误页下的网络状态补偿轮询
  - `Raw BLE Scan`
  - 扫描摘要
  - 构建信息展示
  - 诊断日志导出
  - 经典蓝牙 `RFCOMM` 配网会话
- 仍不是 release 交付包

## 10. APK 产物命名规则

当前 APK 先沿用“日期 + 变更标签 + 构建类型”的命名方式。

格式：

- `lumelo-android-provisioning-YYYYMMDD-<tag>-debug.apk`
- `lumelo-android-provisioning-YYYYMMDD-<tag>-release.apk`

其中：

- `YYYYMMDD`
  - 表示该轮 APK 产物的出包日期
- `<tag>`
  - 用一个短标签概括本轮主要变化
  - 例如：
    - `mainui`
    - `bletest`
    - `mtufix`
- `debug / release`
  - 明确区分 bring-up 包和正式交付包

命名原则：

- 一个 APK 产物名只表达“这轮主要变化”，不要把多个历史修复全串进文件名
- 若同一天多次出包，优先更换 `<tag>`，避免同名覆盖
- 真正面向交付时，必须输出对应 `.sha256`
- debug 包和 release 包不能混名

当前阶段默认：

- 真机联调优先使用 `debug.apk`
- release signing 进入 `APK V3` 阶段后再成为常规流程

如果后续 APK 也要像 `T4 rootfs image` 一样改为全局递增版本号，应在本文件中单独更新规则，不默认沿用 rootfs 的 `v数字` 方案。

## 11. 相关文档

- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
- [apps/android-provisioning/README.md](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/README.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md)
