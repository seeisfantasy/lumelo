# AI Review Part 09

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `docs/Android_Provisioning_App_Progress.md`

- bytes: 11267
- segment: 1/1

~~~md
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
~~~

## `docs/Development_Environment_README.md`

- bytes: 17508
- segment: 1/1

~~~md
# 通用开发环境 README

## 1. 文档用途

本文件独立于产品手册，专门维护：

- 软件环境
- 开发环境配置
- 宿主机文件系统选择
- 虚拟机与真机协作方式
- 可复用的环境搭建原则

使用原则：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 只保留产品原则和长期边界
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md) 只记录开发过程与阶段变化
- 本文件负责“环境怎么搭、为什么这样搭、当前机器上实际是什么环境”

这份文档既服务当前 `Lumelo`，也应能在未来新项目立项、早期技术选型和环境准备时直接复用。

## 2. 通用决策原则

### 2.1 活跃源码目录优先使用原生文件系统

- macOS 主机活跃工作区优先使用 `APFS`
- Linux 原生重负载构建优先使用 Linux 自己的原生文件系统
- `exFAT` 更适合跨设备交换、归档和临时传输，不适合长期承载元数据敏感的源码工作区

### 2.2 为什么 `exFAT` 对 Android 这类构建链不友好

- macOS 会在 `exFAT` 上生成 `._*` AppleDouble 侧写文件
- Android Gradle / 资源打包 / 目录遍历链路容易把这些侧写文件当成正常输入
- 结果通常是资源解析失败、目录扫描异常或增量构建行为不稳定

关键结论：

- 真机连接只能解决“安装、授权、BLE、硬件交互”
- 不能解决“宿主机源码目录所在文件系统不合适”这个问题
- Linux 虚拟机能绕开此问题的前提，是构建实际发生在 Linux 自己的原生文件系统里

### 2.3 真机与模拟器的边界

- BLE 配网、音频输出、Wi-Fi bring-up、板级联调，优先使用真机
- 模拟器只适合基础 UI 或通用 Android 行为验证
- 模拟器不是硬件 bring-up 的替代品

### 2.4 新建 APFS sparsebundle 前必须先确认容量

这是后续默认规则：

- 不能直接默认一个容量
- 必须先询问当前项目需要多大空间

建议按这些因素估算：

- 当前源码体量
- 构建产物体量
- 缓存体量
- 镜像、制品、测试数据是否也要放进去
- 是否会并行保留多个分支或工作副本
- 后续 1 到 3 个月的增长余量

当前 `Lumelo` 这次建立的 `80GiB` 仅是一次实际示例，不应成为以后新项目的默认值。

## 3. 推荐环境分层模板

### 3.1 macOS 主机

适合承担：

- 仓库管理与编辑
- 文档写作
- Android Studio / 真机调试
- 驱动 Linux 虚拟机
- 构建镜像、打包、刷机准备

### 3.2 Linux 虚拟机

适合承担：

- Linux `arm64` 或 `amd64` 编译与测试
- `systemd`、UDS、守护进程联调
- 更贴近目标机的软件运行环境验证

### 3.3 真机

适合承担：

- BLE / Wi-Fi / USB 权限链路验证
- 板级驱动、音频、网络等真实硬件链路验证
- 最终 bring-up 和体验验证

### 3.4 过渡期兜底策略

如果源码暂时还不能离开非原生文件系统：

- 把编译缓存放到原生文件系统
- 把构建输出放到原生文件系统
- 在构建前清理 `._*`

这只能作为过渡方案，不应作为长期默认方案。

## 4. Lumelo 当前已验证环境

### 4.1 macOS 主机

- 系统：`macOS 26.4`
- 架构：`arm64`
- `go 1.26.1 darwin/arm64`
- `rustc 1.94.1`
- `cargo 1.94.1`
- `python3 3.14.3`
- Shell：`/bin/zsh` 与 `sh`

### 4.2 OrbStack / Linux

- 虚拟化：`OrbStack`
- 当前状态：`Running`
- 当前默认且唯一必需机器：`lumelo-dev`
- 发行版：`Debian GNU/Linux 12 (bookworm)`
- 架构：`linux/arm64`
- 虚拟机内已验证：
  - `go 1.26.1 linux/arm64`
  - `cargo 1.94.1`
  - `rustc 1.94.1`

### 4.3 Android 环境

- IDE：`/Applications/Android Studio.app`
- Android Studio 自带 JBR：`OpenJDK 21.0.10`
- `adb`：`1.0.41 / 37.0.0-14910828`
- SDK 路径：`/Users/see/Library/Android/sdk`
- 当前真机调试链已验证可用

### 4.4 当前仓库的语言分工

- Rust：
  - `playbackd`
  - `sessiond`
  - `media-indexd`
  - `ipc-proto`
  - `media-model`
  - `artwork-cache`
- Go：
  - `controld`
- Shell：
  - 顶层 `scripts/`
  - 制镜、验收、开发启动脚本
- Python：
  - bring-up 与蓝牙 / Wi-Fi provisioning helper
- Java：
  - Android 配网 App `apps/android-provisioning`

### 4.5 当前 T4 开发图约定

- `T4` 开发 / bring-up 图默认开启 SSH
- 调试阶段默认远程登录方式可使用 `root/root`
- `SSH_AUTHORIZED_KEYS_FILE` 仍保留为可选能力，用于注入 `/root/.ssh/authorized_keys`
- 正式发布镜像的 SSH 默认值仍以产品手册为准，不因开发图便捷性而改变

## 5. Lumelo 当前文件系统与工作区方案

### 5.1 当前状态

- `SeeDisk`：`exFAT`
- `LumeloDev`：`APFS`

当前已建立：

- sparsebundle 文件：`/Volumes/SeeDisk/Codex/Lumelo-dev.sparsebundle`
- APFS 挂载卷：`/Volumes/LumeloDev`
- 当前推荐 macOS 主工作区：`/Volumes/LumeloDev/Codex/Lumelo`

### 5.2 为什么要这样做

这一步不是为了 Android 模拟器，也不是为了替代 Android 真机。

它解决的是：

- macOS 主机在 `exFAT` 上工作时的 `._*` 污染
- Android 构建链对侧写文件敏感的问题
- 未来其他元数据敏感工具链的类似风险

### 5.3 与 Linux 开发链的关系

- `lumelo-dev` 可以直接访问 `/Volumes/LumeloDev/Codex/Lumelo`
- 所以这个 APFS 工作区不只兼容 Android，也兼容当前 Linux 虚拟机工作流
- 需要注意的是：Linux 使用的是宿主共享目录，不是“Linux 原生挂载 APFS”

## 6. 当前项目下的辅助脚本

- [mount-lumelodev-apfs.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/mount-lumelodev-apfs.sh)
  - 若开发卷未挂载，则挂载 `LumeloDev`
- [sync-to-lumelodev-apfs.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/sync-to-lumelodev-apfs.sh)
  - 将当前工作区同步到 APFS 主工作区
  - 同步时排除：
    - `._*`
    - `out/`
    - `tmp/`
    - Android 构建缓存
    - `services/rust/target/`
- [orbstack-bootstrap-lumelo-dev.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/orbstack-bootstrap-lumelo-dev.sh)
  - 默认按脚本所在仓库根自动推导 `REPO_HOST_PATH`
  - 不再固定依赖旧的 `SeeDisk` 目录

## 7. T4 rootfs 出包运行手册

这一节是常驻运行手册。

后续只要继续给 `NanoPC-T4` 出 rootfs 镜像，默认先对照本节执行，不再临时回忆“上次是怎么绕过 `/tmp`、Orb VM、共享磁盘和权限坑的”。

### 7.1 固定约束

- 活跃源码目录统一使用：
  - `/Volumes/LumeloDev/Codex/Lumelo`
- 重负载临时目录统一使用 Linux 原生目录：
  - `/var/tmp/lumelo-<build-tag>/`
- 不再把 `LUMELO_BUILD_ROOT`、`TMPDIR`、`CARGO_TARGET_DIR`、`GOCACHE` 放到：
  - `/tmp`
  - `/Volumes/...`
- rootfs 产物命名统一为：
  - `out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img`
- 其中 `vN` 为全局递增序号：
  - 不因日期变化重置
  - 新图出包前先确认当前最新序号，再顺延 `+1`

### 7.2 标准出包步骤

1. 在 macOS 主机确认开发卷已挂载：
   - `./scripts/mount-lumelodev-apfs.sh`
2. 将当前仓库同步到 APFS 主工作区：
   - `./scripts/sync-to-lumelodev-apfs.sh`
   - 当前同步脚本会保留目标工作区中的 `out/` 与其他排除目录，避免出包后再次同步时把已生成制品误删
3. 在 macOS 主机确定本轮输出文件名：
   - 例如 `out/t4-rootfs/lumelo-t4-rootfs-20260412-v12.img`
4. 在 `OrbStack / lumelo-dev` 中以 `root` 运行制镜脚本，但显式保留调用用户上下文：
   - `SUDO_USER=see`
5. 将所有重负载目录指到 Linux 原生目录，例如：
   - `TMPDIR=/var/tmp/lumelo-20260412-v12/tmp`
   - `LUMELO_BUILD_ROOT=/var/tmp/lumelo-20260412-v12/build-root`
   - `CARGO_TARGET_DIR=/var/tmp/lumelo-20260412-v12/cargo-target`
   - `GOCACHE=/var/tmp/lumelo-20260412-v12/go-cache`
6. 在 `OrbStack` 内从 APFS 主工作区启动正式制镜：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/LumeloDev/Codex/Lumelo
  export SUDO_USER=see
  export TMPDIR=/var/tmp/lumelo-YYYYMMDD-vN/tmp
  export LUMELO_BUILD_ROOT=/var/tmp/lumelo-YYYYMMDD-vN/build-root
  export CARGO_TARGET_DIR=/var/tmp/lumelo-YYYYMMDD-vN/cargo-target
  export GOCACHE=/var/tmp/lumelo-YYYYMMDD-vN/go-cache
  ./scripts/build-t4-lumelo-rootfs-image.sh \
    --board-base-image /absolute/path/to/rk3399-base.img \
    --output /Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img
'
```

7. 出图后立即做离线验收：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/LumeloDev/Codex/Lumelo
  ./scripts/verify-t4-lumelo-rootfs-image.sh \
    /Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img
'
```

8. 若需要把制品同步回主仓库输出目录，再重新计算一次最终路径下的 `sha256`，避免校验文件还指向旧的 APFS 路径。
9. 若本轮改动涉及 `NanoPC-T4` 无线链路，还要再跑一次官方金样对比：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/LumeloDev/Codex/Lumelo
  ./scripts/compare-t4-wireless-golden.sh \
    --board-base-image /absolute/path/to/rk3399-base.img.gz \
    --image /Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img
'
```

通过标准：

- `0 failure(s), 0 warning(s)`
- `BCM4356A2.hcd`
- `hciattach.rk`
- `bcmdhd.conf`
  与官方底图一致
10. 若本轮改动涉及：
   - 蓝牙
   - Wi-Fi
   - SSH
   - firmware / patch
   - `systemd` bring-up
   - 板级启动链
   则烧录后必须继续执行：
   - [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
11. 在 `T4_Bringup_Checklist.md` 未通过前，不对外宣布“这张图已可烧录验证蓝牙 / SSH / 配网”。

### 7.2.1 T4 无线链路固定金样

针对 `NanoPC-T4` 板载 `AP6356S` 组合模组，后续出包默认以 FriendlyELEC 官方运行态为金样，不再把通用 Debian / Broadcom 路线当成等价替代。

当前已确认的官方金样要点：

- Wi-Fi 驱动走 `bcmdhd`
- 蓝牙走 `hci_uart + btbcm`
- 蓝牙控制器挂在 `ttyS0`
- 蓝牙 patch 文件是：
  - `/etc/firmware/BCM4356A2.hcd`
- 模块策略文件是：
  - `/etc/modprobe.d/bcmdhd.conf`
- 官方蓝牙 bring-up 脚本核心顺序是：
  - 等待 `bcmdhd`
  - `rfkill unblock bluetooth`
  - `hciattach.rk /dev/ttyS0 bcm43xx 1500000`

固定要求：

- 以后若 `T4` 镜像涉及蓝牙 / Wi-Fi / firmware / board bring-up，不再默认补 `brcmfmac4356-sdio.*` 这一套通用兼容文件
- 制镜与离线验收默认检查：
  - `bcmdhd.conf`
  - `/etc/firmware/BCM4356A2.hcd`
  - `bluetooth-uart-attach` 是否按 `bcmdhd` 就绪条件等待
- 若未来确实要偏离官方金样，必须先在 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md) 明确记录原因与验证结果

更细的 Wi-Fi 金样基线与和 `Lumelo` 当前实现的差异，单独维护在：

- [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)

### 7.2.2 T4 在线更新开发回路

从当前阶段开始，`NanoPC-T4` 开发不必把“每改一次都重烧整张 img”当成唯一默认路径。

推荐原则：

- 能通过 SSH 替换并重启局部服务验证的改动，优先走在线更新
- 需要验证 boot 链、内核、分区、底座镜像一致性的改动，仍必须走整包重刷

当前适合在线更新验证的改动：

- `base/rootfs/overlay/` 下的：
  - shell / Python helper
  - systemd unit
  - 配置文件
  - Web 静态资源
- 已经单独编译完成的用户态服务二进制
- 不涉及 bootloader / kernel / DTB / partition layout 的多数 bugfix 与功能迭代

当前不应只靠在线更新验证的改动：

- `bootloader`
- `kernel`
- `dtb`
- 分区表或镜像布局
- first-boot 逻辑
- 依赖“全新镜像初始状态”才能暴露的问题

仓库已提供在线部署 helper：

- [deploy-t4-runtime-update.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/deploy-t4-runtime-update.sh)

典型用法 1：直接把 overlay 文件推到线上板子，并重启对应服务

```sh
./scripts/deploy-t4-runtime-update.sh \
  --host 192.168.1.120 \
  --restart-unit lumelo-wifi-provisiond.service \
  base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond
```

典型用法 2：把已经单独编译好的服务二进制映射到远端指定路径

```sh
./scripts/deploy-t4-runtime-update.sh \
  --host 192.168.1.120 \
  --restart-unit controld.service \
  --map /absolute/path/to/controld:/usr/bin/controld
```

若开发板频繁重刷导致 SSH host key 变化，可临时附加：

```sh
env LUMELO_T4_SSH_OPTIONS='-o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/tmp/lumelo_known_hosts' \
  ./scripts/deploy-t4-runtime-update.sh \
  --host 192.168.1.120 \
  --restart-unit lumelo-wifi-provisiond.service \
  base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond
```

脚本当前行为：

- 自动为被替换文件保留远端时间戳备份
- 自动保留源文件权限位
- 若替换的是 systemd unit，自动执行 `daemon-reload`
- 可按需重启一个或多个 unit

推荐开发顺序：

1. 先在本地完成构建或脚本改动
2. 优先通过在线更新推到测试板
3. 在真机上验证最小闭环
4. 只有当改动触及“必须整镜像验证”的边界时，再出新的 rootfs img 并重刷

因此后续默认开发策略应改为：

- 日常用户态开发：
  - 在线更新优先
- 里程碑验收 / 底座变更 / 版本交付：
  - 必须保留整包重刷验证

### 7.3 常见报错与固定处理

- 现象：`No space left on device`、`mmdebstrap` 异常中断、`cargo`/`apt` 临时文件写入失败
  - 判断：`OrbStack` 内 `/tmp` 是 `tmpfs`，不适合 rootfs 制镜
  - 固定处理：把 `TMPDIR` 和 `LUMELO_BUILD_ROOT` 全部改到 `/var/tmp/lumelo-<build-tag>/`

- 现象：`missing required command: cargo`
  - 判断：以 `root` 执行时丢了调用用户的 Rust 工具链路径
  - 固定处理：在 `OrbStack` 中显式传入 `SUDO_USER=see`，并从仓库当前脚本默认逻辑继承 `cargo` 路径

- 现象：`Invalid argument`、`error deallocating`、共享卷上 `install`/`cp` 写入异常
  - 判断：重负载构建目录放在了 `/Volumes/...` 共享路径，触发宿主共享文件系统兼容性问题
  - 固定处理：共享卷只保留源码和最终制品；编译缓存、临时工作区全部放 Linux 原生目录

- 现象：Android 或其他构建链出现 `._*` 污染、副文件被当成输入
  - 判断：源码目录还在 `SeeDisk/exFAT`
  - 固定处理：先同步到 `/Volumes/LumeloDev/Codex/Lumelo` 再构建

- 现象：出图成功后再次执行同步，`LumeloDev/out/` 中的镜像或 APK 消失
  - 判断：同步脚本误把排除目录也当成“应删除目标”
  - 固定处理：同步脚本必须保留目标侧 `out/`；出包后若还要再次同步，只能使用不会删除排除目录的同步策略

- 现象：`sha256` 文件内容还指向 `/Volumes/LumeloDev/...`
  - 判断：制品移动过目录，但校验文件没重算
  - 固定处理：在最终交付路径重新执行 `shasum -a 256 <final-img> > <final-img>.sha256`

### 7.4 出包前后检查清单

- 出包前确认：
  - 当前 `vN` 序号
  - APFS 开发卷已挂载
  - 已同步到 `/Volumes/LumeloDev/Codex/Lumelo`
  - `OrbStack / lumelo-dev` 正常运行
  - 本轮临时目录全部位于 `/var/tmp/lumelo-<build-tag>/`

- 出包后确认：
  - `verify-t4-lumelo-rootfs-image.sh` 结果为 `0 failure(s), 0 warning(s)`
  - `img` 与 `.sha256` 均存在于最终交付目录
  - `.sha256` 内容中的路径就是最终交付路径
  - 若本轮涉及板级 bring-up 相关改动，已按 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md) 完成真机核查
  - 再对外宣布“可烧录”

## 8. Android 相关当前约定

当前 Android 工程为：

- `apps/android-provisioning`

当前已验证：

- 工程带 `Gradle wrapper`
- Android Gradle Plugin 对齐 `8.13.2`
- SDK 策略对齐到：
  - `compileSdk = 36`
  - `minorApiLevel = 1`
  - `targetSdk = 36`
- Android 真机优先于模拟器

更细的 App 级说明，继续放在：

- [apps/android-provisioning/README.md](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/README.md)
- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
  - 当前 APK 功能结构、进度状态、后续开发计划
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - 当前板端与手机 APK 的配网协议、经典蓝牙传输约定与安全策略
- [archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md)
  - APK 初版目标定义，保留作历史参考

## 9. 文档边界

后续默认按下面分工维护：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
  - 产品原则和长期边界
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 软件环境、开发环境、宿主机文件系统与搭建约定
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
  - 每一步实际开发进度与环境调整记录
~~~

