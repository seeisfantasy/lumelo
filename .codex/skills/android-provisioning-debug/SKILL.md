---
name: android-provisioning-debug
description: 当任务涉及 Android 配网 APK、classic Bluetooth 扫描/连接、加密 Wi-Fi 凭据下发、WebView 恢复或手机与板端归因时使用。
---

## 触发时机
- 改动 Android APK
- 用户反馈：
  - “手机扫不到”
  - “连上后打不开页面”
  - “切网后卡住”
  - “系统蓝牙能看到，App 看不到”
- 需要分辨问题在 APK 还是在 T4 板端

## 先确认当前产品定位
- APK 是 setup path，不是 steady-state 主控制端
- APK 是诊断壳，不是主播放器 App
- 当前联网成功后的主交互仍然是 WebUI
- 当前主链是：
  - classic Bluetooth provisioning
  - encrypted Wi-Fi credentials
  - WebView 壳
- BLE scan 现在更多是 diagnostic，不是 steady-state 主路径

## 调试顺序
1. 先确认系统 Bluetooth settings 能不能看到设备
2. 再确认 App 自己的扫描 / 连接链
3. 再确认板端 classic Bluetooth / provisioning service 是否真的起来
4. 再确认 `hello.security`、加密凭据下发和 `device_info`
5. 最后看 WebView 和网络切换恢复

## 重点观察
- 系统 Bluetooth settings 是否可见 `Lumelo T4`
- App 内：
  - `BLE TEST SCAN`
  - Lumelo 专用扫描
  - `CONNECT`
  - `device_info`
  - Wi-Fi 凭据下发
  - 状态轮询
- 板端：
  - `/healthz`
  - `/provisioning-status`
  - `/logs.txt`
  - classic Bluetooth / Wi-Fi apply 日志

## 当前协议边界
- 当前应优先看到：
  - `hello.security`
  - encrypted credentials
- 不要把 plaintext fallback 当成当前正确主线

## 归因原则
- 如果系统蓝牙设置能看到设备，但 App 看不到：
  - 优先怀疑 APK 扫描 / 过滤 / 连接逻辑
- 如果系统蓝牙设置也看不到：
  - 优先怀疑板端 discoverable / controller / service
- 如果配网成功但 WebView 打不开：
  - 先确认手机和板子是否真的在同一 reachable network
  - 不要先怀疑 WebView 本身

## 禁止
- 把 APK 误判成 steady-state 主控制端
- 跳过板端日志，直接把问题归咎于手机
- 只因为 `advertising = true` 就认定手机一定能扫到
