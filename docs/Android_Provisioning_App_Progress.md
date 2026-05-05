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
- 本文件
  - 维护“当前 APK 做到了什么、接下来怎么开发、结构如何拆分”

## 2. 当前产品定位

当前手机 APK 不是一个完整播放器，也不是 V1 的 steady-state 主控制端。

当前定位是：

- `经典蓝牙 + Wi-Fi provisioning` 的手机 setup 工具
- 板端异常时的第一诊断入口
- 配网成功后承载一个 APK 内部 `WebView` 外壳
- `Raw BLE Scan` 只保留为诊断能力

当前不做的事情：

- 本地曲库浏览主实现
- 主播放控制实现
- 云账号
- 后台同步
- App Store 级产品化打磨

V1 的 steady-state 主交互仍然是：

- Ethernet / Wi-Fi 下的 WebUI

## 3. 当前状态

截至 `2026-04-15`，APK 侧当前已经做到：

- 经典蓝牙扫描 `Lumelo` 设备
- 经典蓝牙扫描结果合并：
  - 首次 `ACTION_FOUND`
  - 后续 `ACTION_NAME_CHANGED`
- 经典蓝牙候选提示增强：
  - `[LAST]`
  - `[PAIRED]`
  - `[NAME]`
  - `[CLASSIC]`
  - `RSSI`
- 在拿不到稳定 `nameMatch` 时：
  - 会优先选中最近成功连接过的 classic 候选
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
- 完成 `hello.security` 协商
- 读取 `device_info`
- 把 `device_info` 展示成结构化摘要
- 发送 Wi-Fi 凭据
- 当前只发送：
  - `wifi_credentials_encrypted`
- 触发 `apply`
- 读取 provisioning `status`
- classic provisioning 会话已补：
  - `ack timeout`
  - 最小 `retry`
  - 最小 `reconnect`
  - 失败后主动拉 `status` 判定当前阶段
- 连接后自动进入 APK 内 `WebView`
- APK 内打开：
  - Home
  - Library
  - Provisioning
  - Logs
  - Healthz
- classic 会话中途断开或 `reset` 后：
  - 会保留 `Last known WebUI`
  - 会保留 `Last known T4 Wi-Fi`
  - `OPEN WEBUI / OPEN PROVISIONING / OPEN LOGS / OPEN HEALTHZ` 不再一起失效
- 若 classic 扫描能看到 `lumelo`，但 `RFCOMM` connect 全部 fallback 仍失败：
  - 状态文案会明确提示：
    - `Lumelo was discovered, but the provisioning service did not answer`
  - 若已有上次确认过的 `WebUI` 地址：
    - 会额外提示直接尝试 `OPEN WEBUI`
  - 最终状态不会再被一条泛化的：
    - `Disconnected from T4`
    覆盖掉真正的 connect failure 归因
  - APK 还会后台 probe 一次上次记住的 `/healthz`
    - 如果这台手机也打不通：
      - 会直接提示：
        - `Last known WebUI is unreachable from this phone right now`
        - `T4 may have lost Wi‑Fi or changed IP`
    - 若手机当前在 `/24` 私网：
      - APK 还会继续探测当前 `/24` 里是否有别的 Lumelo `/healthz`
    - 如果也没找到：
      - 会直接提示：
        - `No Lumelo WebUI responded on the current /24 subnet`
- APK 内 `WebView` 壳顶部入口已补中文化：
  - `首页`
  - `曲库`
  - `日志`
  - `设置`
  - `重试`
  - `浏览器`
  - `返回`
- 页面内 debug log
- 导出诊断日志
- 可一键复制当前 `Diagnostic Summary`
  - 包含：
    - 当前 `status`
    - 手机当前 `Wi-Fi / IPv4`
    - `Last known T4 Wi‑Fi`
    - `Last known WebUI`
    - 最近一次 `WebUI probe`
    - 最近一次当前 `/24` 子网扫描结果
    - `classic session` 摘要
  - 复制动作现在只走 `Toast + debug log`
    - 不再把顶部主状态覆盖成：
      - `Copied...`
- `Export Diagnostics` 现在会把这段 `Quick Summary` 放在完整诊断正文前面
- 当前已额外确认：
  - `lintDebug`
    - 已无 hard error
    - 当前剩余 warning 已只收敛到：
      - `MissingApplicationIcon`
      - `SetTextI18n`
    - 也就是说，当前 APK 已没有明显还会影响运行时行为的 lint warning
- 页面内显示：
  - `App version`
  - `build time`
  - `git short SHA`
- `Use Current Wi-Fi` 预填当前 SSID
- 表单区会显示：
  - `Credential form`
  - `Target SSID`
  - `Password length`
  - `Password type`
  - `Phone Wi-Fi`
  - `Last T4 Wi-Fi`
- 若用户改了 `SSID` 但沿用旧 password
  - 发送前会直接拦住并提示
- `WebView` 错网页已补：
  - 当前 `T4` host
  - `Last confirmed T4 Wi-Fi`
  - `Phone Wi-Fi`
  - `Phone IPv4`
  - 显式 `Retry`
  - 网络轮询补偿

当前已知边界：

- 手机侧经典蓝牙扫描链路已作为主通道
- `Raw BLE Scan` 保留为诊断能力，不再承担主配网职责
- 官方金样真机上，APK 已经能扫到：
  - `NanoPC-T4`
  - 但官方系统不提供 `Lumelo` 的 `RFCOMM` provisioning service
  - 因此只能验证“经典蓝牙发现链路”和手机兼容性
  - 不能直接完成 `device_info / credentials / apply / status` 全闭环
- APK 仍以 bring-up / diagnostic 为主，不是最终交付形态
- 这版最新 debug APK 已重新装到：
  - `PJZ110`
- 2026-04-25 本轮复验设备：
  - `BKQ-AN90 / Android 16`
- 已补过一轮最小真机 smoke：
  - 主界面可启动
  - classic 扫描可见 `lumelo`
  - classic 连接可拿到 `device_info`
  - APK 内 `WebView` 壳可打开
  - `Retry` 按钮在场
- 2026-04-25 已在连接态逐个点开：
  - `OPEN WEBUI`
  - `OPEN PROVISIONING`
  - `OPEN LOGS`
  - `OPEN HEALTHZ`
- 2026-04-26 已在 `PJZ110 / Android 16` 跑通真实 Wi-Fi provisioning：
  - `SCAN -> CONNECT -> USE CURRENT WI-FI -> SEND WI-FI CREDENTIALS`
  - 手机 Wi-Fi：`iSee`
  - T4 `wlan0`：`192.168.71.243/24`
  - APK 自动进入 `http://192.168.71.243:18080/`
  - 手机侧 `/healthz` 返回 `provisioning_state=connected`
- 2026-04-26 已安装新版 debug APK 到当前连接手机：
  - APK path：`/tmp/lumelo-android-build/app/outputs/apk/debug/app-debug.apk`
  - APK sha256：`2906907abf42f94fa41fdb77c8a3cc1a43167272f33e78c7b4257a72f5ace370`
  - 新版默认使用 WebUI port `80`，生成 `http://<T4_IP>/`
  - 当前产品入口策略已固定：
    - 默认入口：`http://lumelo.local/`
    - 可靠入口：`http://<T4_IP>/`
    - 放弃入口：`http://lumelo/`
  - 2026-05-05 已实现 P0：配网成功后 APK 先用短超时 probe `http://lumelo.local/healthz`；成功则打开 `.local`，失败则打开 provisioning status 返回的 IP URL。
  - 尚未构建新版 APK / 安装到手机 / 真机验证 `.local` success 与 fallback 两条路径；当前 Mac 环境缺 Java Runtime，`./gradlew :app:assembleDebug` 无法启动。
- 但下面两类分支还没做专门现场回归：
  - 人工制造的 `ack timeout / write failed / auto reconnect`
  - 新手机首次 classic 首配、完全拿不到稳定 `nameMatch` 时的候选判断
- `2026-04-22` 新增一条现场事实：
  - 手机仍能扫到 `lumelo`
  - 但 classic `RFCOMM` connect 现场出现：
    - `HCI_ERR_PAGE_TIMEOUT`
    - `SDP_CFG_FAILED`
  - 当前更像是板端 classic provisioning service / 可达性问题，不像 APK 扫描问题
- `64-char hex PSK` 真机验证与相关开发已明确延后到 `V2`

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
- 当前匿名 classic 候选识别已经做过一轮 UX 改善
- 后续仍可继续优化成更完整的“扫描响应合并”和更稳定的首次配对提示

### 4.3 传输会话层

负责：

- 经典蓝牙连接
- `RFCOMM` socket 建立
- 逐行 JSON 收发
- 状态读取
- 断开连接

当前痛点：

- 逻辑仍集中在一个 `MainActivity`
- 目前已补最小 `ack timeout / retry / reconnect`
- 后续应继续把经典蓝牙会话和 BLE 诊断会话拆开维护

### 4.4 Provisioning 流程层

负责：

- 组织 Wi-Fi 凭据 payload
- 写入凭据
- 触发 apply
- 轮询状态
- 根据状态切换 UI

当前痛点：

- 最小 `ACK / retry / reconnect` 已经落地
- `disconnect -> 保留 WebUI 诊断入口` 已做过一轮真机回归
- 但人工制造的 `ack timeout / write failed / auto reconnect` 还缺少专门现场回归
- 不同手机在经典蓝牙配对 / 非配对连接上的兼容性还要继续测

### 4.5 APK 内主界面壳层

负责：

- 在 `connected` 后进入 APK 内 `WebView`
- 提供 Home / Library / Provisioning / Logs 等入口
- 在错网或断网时提供恢复引导与显式 `Retry`

定位：

- 这是 `WebUI` 的容器
- 不是要在 APK 内重做一套主播放器 UI

## 5. 代码结构建议

当前代码主要集中在：

- [MainActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java)
- [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)
- [MainInterfaceActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainInterfaceActivity.java)

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

当前状态：

- 经典蓝牙扫描摘要、`Raw BLE Scan`、构建信息、导出日志都已落地
- `WebView` 错网恢复引导和 `Retry` 也已补上
- 当前还需要补的是：
  - 新手机首次 classic 首配的候选判断现场回归
  - 对扫描会话和诊断会话的代码拆分

完成标准：

- 不接 `adb` 也能判断问题是在经典蓝牙发现、BLE 诊断链，还是板端空口广播

### 6.2 APK V2：配网闭环版

目标：

- 把“连接后能否稳定把 Wi-Fi 凭据送到 T4”做扎实

当前状态：

- `device_info` 结构化摘要已落地
- `ACK timeout / retry / reconnect / status 判相` 的最小版本已落地
- 当前还需要补的是：
  - recovery 分支的专门现场验证
  - 状态文案和失败态提示继续打磨
  - `64-char hex PSK` 真机验证与相关开发

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

1. `经典蓝牙 provisioning recovery` 专门现场回归
2. `新手机首次 classic 首配` 候选提示回归
3. `V3` 产品化整理

原因：

- 当前 APK 最大价值仍然不是“更像正式 App”
- 而是把 classic Bluetooth 主链的 recovery 和可诊断性真正压实

## 8. 当前验收重点

下一轮 APK / 真机联调，优先看：

1. 手机是否能在 `Lumelo Scan` 中扫到 `Lumelo T4`
2. 连接后是否能稳定完成 `device_info`
3. Wi-Fi 凭据是否成功写入并触发 `apply`
4. `status` 是否能推进到 `connected`
5. 人工制造 `ack timeout / disconnect / write failed` 后，最小 `retry / reconnect` 是否能恢复
   - 当前 `disconnect -> 保留 WebUI 诊断入口` 已实测
   - `ack timeout / write failed / auto reconnect` 仍待专门诱发验证
6. APK 内 `WebView` 是否能稳定进入主界面，并在错网时给出正确恢复引导
   - 当前 `OPEN WEBUI / OPEN PROVISIONING / OPEN LOGS / OPEN HEALTHZ` 正常进入 APK 内 `MainInterfaceActivity`
   - 错网恢复引导仍需单独制造现场验证
7. `Raw BLE Scan` 是否还能看到目标广播
   - 这条是诊断项，不再单独作为主链通过标准

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

当前目录中保留的最新 APK build record 是：

- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256)

要特别注意：

- `2026-04-15` 之后补的这些 APK 小修：
  - `WebView` 错网提示 / 恢复引导
  - classic 匿名候选识别 UX
  - `device_info` 结构化摘要
  - `ack timeout`
  - 最小 `retry / reconnect`
  当前已经在源码里
- 但还没有重新归档成新的命名 APK 文件
- 因此如果要拿“当前最新 APK 行为”去真机联调
  - 应以当前源码重新 `assembleDebug` 为准

当前这条 APK 记录的定位是：

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

## 9.1 2026-04-25 live classic connect 复验结论

这轮在 wired live `T4 192.168.71.7` 上，已经把 APK 侧一个之前的误判纠正掉了。

已验证事实：

- 手机上的 APK 重新 classic scan 后，能看到：
  - `[LAST] [NAME] Lumelo T4 (C0:84:7D:1F:37:C7)`
- 页面里 `selected=C0:84:7D:1F:37:C7`
  是成立的
- 之前之所以误判“没有进入 connect / 像是 UI-state bug”，是因为：
  - `CONNECT` 按钮在列表下方
  - 需要把页面往下滚到 `Selected:` 区域后面才能看到
- live 复验已确认：
  - `CONNECT` 按钮存在
  - `enabled=true`
  - 点击后 `Classic session: connected=true`
  - `Device info` 能成功回读：
    - `Name: Lumelo T4`
    - `Hostname: lumelo`
    - `IP: 192.168.71.7`
    - `Web port: 80`
  - 再向下滚动后：
    - `READ STATUS = enabled`
    - `DISCONNECT = enabled`
    - `READ STATUS` 结果已成功回读 `advertising` payload

因此当前 APK 的 live 结论要明确修正为：

- 当前不是“选中设备后 connect UI/state 断了”
- 真实状态是：
  - board-side classic provisioning 修复后
  - APK 的 `SCAN -> CONNECT -> device_info -> status`
    主链已经在真机手机上跑通

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
