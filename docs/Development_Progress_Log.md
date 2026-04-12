# Lumelo 音频系统开发进度日志

## 1. 文档用途

本文件只负责记录：

- 当前做到哪一步
- 当前主线是什么
- 还没闭环的问题是什么
- 每次实际开发推进新增了什么
- 接下来最应该做什么

本文件不再重复承载已经稳定收口的产品原则、长期边界和服务职责。

这些稳定约束的权威来源统一为：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)

## 2. 使用规则

- 只记录“当前进度”和“阶段变化”
- 已经形成长期结论、预计不会频繁变更的内容，统一放到 [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- 每完成一步实际开发、验证、构建或策略调整，都要及时更新本日志
- 真机镜像出包前，先结合现场现象分析 bug 与根因；如用户还有其他待验需求，优先合并后统一出包
- 交接压缩摘要不写在这里，需要时单独整理到 [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
- `NanoPC-T4` rootfs 出包前，先对照 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md) 中的 “`7. T4 rootfs 出包运行手册`” 执行，避免重复踩 `/tmp`、`/Volumes` 共享目录、`cargo` 路径和 `sha256` 路径残留等环境坑
- 若本轮改动触及板级无线、SSH、firmware 或启动链，烧录后继续按 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md) 做真机核查

## 3. 当前开发基线

- 当前目标版本：`V1`
- 当前工作模式：`Local Mode`
- 当前产品形态：`headless` 本地音频系统
- 当前开发策略：`OrbStack + NanoPC-T4 真机` 双层推进

当前分工：

- `OrbStack / lumelo-dev` 负责服务逻辑、持久化、IPC、WebUI、`systemd` 基础验证
- `NanoPC-T4` 真机负责真实 `ALSA hw`、DAC、板级、启动链、热插拔和长稳验证

一句话约束：

- 在 `OrbStack` 中执行 `PLAY`，目前只代表“逻辑播放链路成功”
- 不代表“真实音频已经通过 ALSA 输出到 DAC”

## 4. 当前里程碑状态

- `M0` 开发环境与验证层搭建：已完成
- `M1` 仓库骨架与服务最小闭环：已完成
- `M2` `queue.json` 持久化与恢复语义：已完成第一版
- `M3` 继续留在 `OrbStack` 中补服务逻辑：阶段性完成基础闭环
- `M4` 切到 `T4` 的触发点：一旦进入真实 `ALSA` 输出实现就切换
- `M5` T4 真机首轮 bring-up：当前主线
- `M6` 真机音频参数与稳定性收敛：未开始
- `M7` 打包、恢复与交付链路：未开始

## 5. 当前未闭环事项

### 5.1 T4 现阶段未闭环问题

- 当前离线验收通过、且目录中实际保留的最新主图已更新为：
  - [lumelo-t4-rootfs-20260412-v16.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img)
  - [lumelo-t4-rootfs-20260412-v16.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img.sha256)
- `v16` 已完成两道离线关卡：
  - `verify-t4-lumelo-rootfs-image.sh`：`0 failure(s), 0 warning(s)`
  - `compare-t4-wireless-golden.sh`：`0 failure(s), 0 warning(s)`
- 当前板级无线路线已经收口到官方金样：
  - `bcmdhd`
  - `/etc/firmware/BCM4356A2.hcd`
  - `/system/etc/firmware/fw_bcm4356a2_ag.bin`
  - `/system/etc/firmware/nvram_ap6356.txt`
- 经典蓝牙 `RFCOMM` 配网主链已在真机上跑通：
  - 手机能扫描并连接 `lumelo`
  - `device_info` 可读
  - Wi-Fi 凭据可下发
  - 热点 `isee_test` 场景下已推进到 `connected`
  - T4 已拿到 `192.168.43.170`
- 当前剩余板端未闭环点：
  - `v16` 还未完成“无人工干预冷启动”真机回归
  - 需要确认蓝牙冷启动修复是否在上板后真正生效，不再需要手工拉起：
    - `hciattach.rk`
    - `bluetoothd`
    - `classic-bluetooth-wifi-provisiond`
  - 需要补家庭路由器场景验证，不只测手机热点
  - 需要补重启后 Wi-Fi 自动回连验证
  - 需要补双网卡 / 双 IP 场景下首页与配网页的地址展示验证

### 5.2 Android 配网链未闭环

- Android 配网 App 当前主通道已切到经典蓝牙，当前目录中的最新 APK 为：
  - [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
- 当前 APK 已在真机上验证：
  - 经典蓝牙扫描
  - 名称更新归并
  - 大小写不敏感过滤
  - `RFCOMM` 连接
  - `device_info`
  - Wi-Fi 凭据发送
  - WebView 打开主界面
- `Raw BLE Scan` 保留为诊断能力，不再承担主配网职责
- WebView 切网恢复崩溃问题已修复：
  - 断网时会停留在错误页
  - 重新回到与 T4 同一热点后会自动恢复
- 当前 APK 仍未闭环点：
  - 手机系统有时会自动连回其他已保存 Wi-Fi，例如 `iSee`
  - 在这种情况下 App 不会崩，但仍会停留在错误页，直到回到与 T4 可互通的网络
  - 当前 Wi-Fi 凭据仍通过经典蓝牙以明文 JSON 下发，开发阶段可用，正式版仍需安全加固

### 5.3 音频真机验证未开始

- 真实 `ALSA hw` 打开
- `audio_thread`
- 首帧写入 ALSA 后进入正式 Quiet Mode 的最终语义
- `Strict Native DSD / DoP`
- `period / buffer / XRUN`

这些都还没有进入真机闭环验证。

## 6. 当前推荐下一步

按当前状态，建议直接按下面顺序推进：

1. 将 [lumelo-t4-rootfs-20260412-v16.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img) 烧写到 TF 卡并上板
2. 手机安装并使用：
   - [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
3. 做一次无人工干预冷启动回归，优先确认：
   - 经典蓝牙是否自动起来
   - `Lumelo Scan` 是否能直接扫到板子
   - `CONNECT -> device_info -> SEND WI-FI CREDENTIALS` 是否无需手工拉服务即可完成
4. 分别补两类网络场景：
   - 手机热点场景
   - 家庭路由器场景
5. 在手机切网测试中继续确认：
   - 回到与 T4 同一热点后，WebView 是否自动恢复
   - 连回错误 Wi-Fi 时，提示是否足够明确
6. 只有在经典蓝牙配网链、WebView、家庭路由器回连都稳定后，再切入：
   - 真实曲库
   - 真实播放
   - `ALSA hw` 与最小音频链验证

## 7. 时间线日志

### 7.1 2026-04-09：Android SDK 版本对齐与开发环境文档补齐

本轮已完成：

- Android 配网 App 工程改为：
  - `compileSdk 36`
  - `targetSdk 36`
- Android Gradle Plugin 升级为 `8.13.2`
- 已生成 `Gradle wrapper`
- Android App README 已补充当前对齐到本机 `Android SDK Platform 36.1`
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 已补“当前推荐开发环境”章节
- Android 命令行构建已验证可用
- debug APK 已成功安装到 Android 真机 `PJZ110`
- App 已成功启动到前台，且 BLE / 定位关键权限已授予
- 已完成一次带真机在线连接的重新构建、重装与前台复验
- 当前首屏已确认为 `Lumelo Wi-Fi Setup`
- 首屏当前状态已确认：
  - `SCAN FOR LUMELO` 可点击
  - 尚未发现设备时，`CONNECT` / `SEND WI-FI CREDENTIALS` / `OPEN WEBUI` 处于禁用态
  - 页面已包含 `Wi-Fi SSID` 与 `Wi-Fi password` 输入框
- 已完成一次真机空扫描路径验证：
  - 扫描开始后状态文案会切到 `Scanning for Lumelo T4...`
  - 扫描期间 `SCAN FOR LUMELO` 会禁用，防止重复触发
  - 扫描窗口结束后会回到 `Scan finished. No Lumelo device found.`，并恢复再次扫描能力
- 已修复 Android 构建前 `._*` 清理任务的可靠性问题：
  - 原 `Delete` 方案会被 Gradle 判定为 `UP-TO-DATE`
  - 现已改为基于 Java NIO 的目录遍历删除，并完成独立自测
- 已确认 `SeeDisk` 文件系统为 `exFAT`
- 已在 `SeeDisk` 上创建 Android/macOS 长期开发用 APFS sparsebundle：
  - 镜像文件：`/Volumes/SeeDisk/Codex/Lumelo-dev.sparsebundle`
  - 当前挂载点：`/Volumes/LumeloDev`
- 已完成首轮工作区同步：
  - 新主工作区候选路径：`/Volumes/LumeloDev/Codex/Lumelo`
  - 本轮同步已排除 `._*`、`out/`、`tmp/` 与 Android 构建缓存目录
- 已在新 APFS 工作区完成一次 Android 命令行构建复验：
  - 构建命令成功
  - `apps/android-provisioning` 下未出现新的 `._*`
  - `problems-report.html` 已正常生成
- 已确认 `lumelo-dev` 可直接访问 `/Volumes/LumeloDev/Codex/Lumelo`
- 已新增 APFS 开发卷辅助脚本：
  - `scripts/mount-lumelodev-apfs.sh`
  - `scripts/sync-to-lumelodev-apfs.sh`
- 已修正 `scripts/orbstack-bootstrap-lumelo-dev.sh`：
  - 默认 `REPO_HOST_PATH` 现按当前仓库根自动推导
  - 不再固定依赖旧的 `SeeDisk` 目录
- 后续 APFS 同步将排除 `services/rust/target/`，避免把 Linux/Rust 旧构建产物一并固化到新工作区
- 已新增独立环境文档：
  - [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 统一承接软件环境、开发环境配置、宿主机文件系统与 APFS 工作区约定
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 已移除大段环境细节，改为指向环境 README
- 已明确后续规则：
  - 新建 APFS sparsebundle 前必须先确认所需容量
  - `80GiB` 只是本次 Lumelo 示例，不作为后续默认值
- 已对现有 `20260408-bootfix` 图做离线验收：
  - 该图仍未包含 BLE 配网底座
  - 验收结果为 `2 failure(s), 6 warning(s)`
- 已增强 `bluetooth-wifi-provisiond`：
  - 启动时会等待 BLE adapter 出现，而不是立即退出
  - adapter 缺失时改为返回非零退出码，避免 systemd 误判为成功退出
  - GATT / 广播注册失败时会明确报错并返回非零
- 已修正 `build-t4-lumelo-rootfs-image.sh` 在 Linux `sudo` 场景下的工具链兼容性：
  - 自动补入 `/usr/local/go/bin`
  - 自动补入调用用户的 `cargo` 路径
  - 自动继承调用用户的 `CARGO_HOME / RUSTUP_HOME`
- 已确认 `lumelo-dev` 的 `/tmp` 为 `tmpfs`，不适合承载 T4 rootfs 制镜工作目录
- 本轮实际制镜改为使用 `/var/tmp`
- 已从 APFS 主工作区重建新图：
  - `/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260409-bleprov.img`
  - `/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260409-bleprov.img.sha256`
- 已对 `20260409-bleprov` 新图完成离线验收：
  - `0 failure(s), 0 warning(s)`
  - BLE provisioning helper / Wi-Fi helper / GATT daemon / systemd units / wireless DHCP network 均已在图中
- 当前已明确：
  - Android 真机解决的是安装、权限与 BLE 调试
  - APFS sparsebundle 解决的是宿主机 `exFAT -> ._*` 污染问题

本轮确认的当前主开发环境：

- macOS 主机
- `OrbStack / lumelo-dev`
- Android Studio
- Android 真机

阶段意义：

- Android 工程不再继续锁在旧的 `SDK 35`
- 文档里已经明确列出当前多语言开发栈和多环境协作方式
- OrbStack 口径已收口为“`lumelo-dev` 为唯一默认开发机，`fono-dev` 非必需”
- Android bring-up 已从“只有工程骨架”推进到“真机已装包并成功启动”

本轮额外结论：

- 由于仓库位于外接盘环境，Android 构建输出若直接写回项目目录，会被 `._*` AppleDouble 文件污染
- 当前已通过两层方式稳定规避：
  - 构建前自动清理 `._*`
  - 将 Gradle cache、project cache 和 Android build 输出统一落到 `/tmp`

### 7.1.1 2026-04-09：开发版 T4 镜像默认 SSH 登录

本轮已完成：

- `build-t4-lumelo-rootfs-image.sh` 默认值已改为开发 / bring-up 图默认 `ENABLE_SSH=1`
- 不再要求 `ENABLE_SSH=1` 时必须额外提供 `SSH_AUTHORIZED_KEYS_FILE`
- `SSH_AUTHORIZED_KEYS_FILE` 现保留为可选项：
  - 需要时可继续向 `/root/.ssh/authorized_keys` 注入公钥
- `t4-bringup-postbuild.sh` 已新增开发态 SSH 策略写入：
  - `PermitRootLogin yes`
  - `PasswordAuthentication yes`
- 当开发图启用 SSH 时：
  - 会启用 `ssh.service`
  - 会把 `/etc/lumelo/config.toml` 的 `ssh_enabled` 写成 `true`
- 当显式 `ENABLE_SSH=0` 时：
  - 会移除开发态 SSH 配置和 root `.ssh` 目录
- `build-t4-ssh-bringup-image.sh` 已改成：
  - 无公钥也可直接构建开发 SSH 图
  - 如需公钥登录，再额外传 `--ssh-authorized-keys`
- `verify-t4-lumelo-rootfs-image.sh` 已增强：
  - 若镜像声明 `SSH enabled in image: 1`
  - 会额外验证 `ssh.service`、`ssh_enabled = true` 和开发态 `sshd_config` 片段
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 已明确：
  - 正式发布镜像默认 SSH 关闭
  - 开发 / bring-up 镜像可默认开启 SSH
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md) 与 [packaging/image/README.md](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/README.md) 已同步这条开发约定

当前阶段意义：

- 下一张 T4 开发图将不再依赖本地显示器与键盘才能排障
- 真机一旦拿到 IP，就能直接通过 SSH 进入板子定位 BLE、Wi-Fi、networkd 和 WebUI 问题
- “开发图默认 SSH 开、正式图默认 SSH 关”的边界已经在脚本和文档两侧同时收口

### 7.1.2 2026-04-09：修复蓝牙 rfkill 解锁时序

真机现场新增发现：

- 用户在 [lumelo-t4-rootfs-20260409-bleprov-ssh.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260409-bleprov-ssh.img) 上实测时，
  `bluetooth.service` 启动失败
- 现场 `rfkill list` 已明确看到：
  - `bt_default` 为 `Soft blocked: yes`
  - `hci0` 为 `Soft blocked: yes`
- `lumelo-bluetooth-provisioning.service` 与 `lumelo-wifi-provisiond.service`
  也因此一起落入 dependency failure

本轮修正：

- 新增 `bluetooth.service` 的 systemd drop-in：
  - `base/rootfs/overlay/etc/systemd/system/bluetooth.service.d/10-lumelo-rfkill-unblock.conf`
- 修正后的策略改为：
  - 在 `bluetooth.service` 启动前先执行 `rfkill unblock bluetooth`
  - 并额外执行 `rfkill unblock all`
- `lumelo-bluetooth-provisioning-mode` 也同步补强为：
  - 继续执行 `rfkill unblock bluetooth`
  - 再执行 `rfkill unblock all`
- `verify-t4-lumelo-rootfs-image.sh` 已把该 drop-in 纳入离线验收

阶段判断：

- 当前问题更像是板级蓝牙被 `systemd-rfkill` 或固件默认状态恢复成 soft-block 后，
  我们的解锁逻辑放得太晚
- 这次修正的核心不是“再多做一次 unblock”，而是把 unblock 时机前移到
  `bluetoothd` 真正启动之前

### 7.1.3 2026-04-09：改为批量准备真机验收，并增强 APK / 诊断可观测性

本轮策略调整：

- 真机验证改为“先讨论清楚问题和验证目标，再统一出包”
- 不再因为单点怀疑立即重建镜像并要求用户上板
- 后续尽量把多个改动攒成一轮，在同一晚集中验收

本轮已落地：

- Android 配网 App 已增强可观测性：
  - 新增环境摘要区：
    - 蓝牙适配器状态
    - 扫描权限状态
    - 连接权限状态
    - Android SDK 级别
  - 新增屏内 debug log：
    - 扫描开始 / 结束
    - 发现设备
    - 连接状态变化
    - 服务发现结果
    - GATT read / write 状态
    - Result payload
  - 结果页新增：
    - `Open WebUI`
    - `Open Logs`
    - `Open Healthz`
- `lumelo-t4-report` 已增强：
  - 输出 `systemctl cat bluetooth.service`
  - 输出 `bluetooth.service` drop-in 目录
  - 输出 `10-lumelo-rfkill-unblock.conf`
  - 输出 `brcm` 与 `rtl_bt` 固件目录
  - 输出聚焦 `bluetooth|brcm|hci|firmware|rfkill` 的 dmesg 过滤尾部
- 已核实当前镜像与官方 FriendlyELEC 底图都包含：
  - `brcmfmac43455/43456-sdio.bin`
  - `brcmfmac43455/43456-sdio.txt`
  - 多组 `rtl_bt/*` 固件

当前判断：

- 从当前证据看，眼前的首要问题仍然更像：
  - 蓝牙被 soft-block
  - `bluetooth.service` 启动时序不对
- “定制系统完全漏装常见固件包”不是当前最匹配的第一判断
- 但 Broadcom 蓝牙侧若后续仍异常，仍需继续核实是否存在更具体的蓝牙固件命名 / 加载问题

明晚统一验收前，优先继续做：

- APK 侧更清晰的失败态提示
- 板端更细的 BLE / Wi-Fi 诊断输出
- 尽量把下一轮真机验收所需改动一次性收齐

### 7.1.4 2026-04-09：补齐配网状态机、Web 状态入口与动态 Wi-Fi 接口适配

本轮新增能力：

- `bluetooth-wifi-provisiond` 不再只在 apply 后等 2 秒看一次 IP：
  - 现在会把状态持续写到 `/run/lumelo/provisioning-status.json`
  - 状态细化为：
    - `advertising`
    - `credentials_ready`
    - `applying`
    - `waiting_for_ip`
    - `connected`
    - `failed`
  - `lumelo-wifi-apply` 成功后会继续异步等待 DHCP / Wi-Fi IP
  - 若超时，会明确落成 `failed`
  - 若 BLE adapter 缺失或 GATT / 广播注册失败，也会先写出失败态状态文件
- `lumelo-wifi-apply` 已改为动态匹配无线接口：
  - 优先 `LUMELO_WIFI_IFACE`
  - 其次 `WIFI_INTERFACE`
  - 再回退到 `iw dev` / `/sys/class/net/*/wireless` 自动探测
  - 不再硬编码只写 `wlan0`
- `controld` 已新增配网页面与状态 JSON：
  - `GET /provisioning`
  - `GET /provisioning-status`
  - `GET /healthz` 也同步带出 provisioning 摘要字段
- Android 配网 App 已继续增强：
  - 新增 `Read Status`
  - 新增 `Disconnect`
  - 新增 `Open Provisioning`
  - 新增 `Clear Debug Log`
  - 启用状态通知后会主动再读一次 `status` characteristic
- `lumelo-t4-report` 已继续增强：
  - 输出 `/run/lumelo/provisioning-status.json`
  - 根据当前探测到的无线接口，输出对应 `wpa_supplicant@<iface>.service`
  - 避免后续再被 `wlan0` 假设误导

本轮本地验证：

- `go test ./...` 在 `services/controld` 下通过
- Android `:app:assembleDebug` 重新构建通过
- `python3 -m py_compile` 继续通过 `bluetooth-wifi-provisiond`
- `sh -n` 继续通过：
  - `lumelo-wifi-apply`
  - `lumelo-t4-report`

### 7.1.5 2026-04-09：继续压实失败态诊断，并把 provisioning 摘要推进到首页

本轮继续收口：

- `bluetooth-wifi-provisiond` 的状态 payload 继续增强：
  - 新增 `error_code`
  - 新增 `apply_output`
  - 新增 `diagnostic_hint`
  - 新增 `wpa_unit`
  - 新增 `ip_wait_seconds`
- `waiting_for_ip` 不再沿用 `lumelo-wifi-apply` 的 stdout 当状态文案：
  - 现在会明确写成“已应用凭据，正在等待 DHCP”
  - 但仍保留 `apply_output` 便于定位
- `controld` 首页 `/` 现在也会直接展示 provisioning 摘要：
  - 当前状态
  - SSID
  - Wi-Fi 接口
  - IP
  - `error_code`
  - `apply_output`
  - `diagnostic_hint`
  - 以及直达 `/provisioning`、`/provisioning-status`、`/healthz`、`/logs` 的入口
- `lumelo-t4-report` 继续增强：
  - 若探测到当前无线接口，会额外输出该接口的 `ip addr show`
  - 会额外输出 `networkctl status <iface>`

本轮本地验证：

- `go test ./internal/provisioningclient ./internal/api` 继续通过
- 这轮没有出新的 T4 镜像，继续遵守“先讨论清楚再统一出包”的真机流程

### 7.1.6 2026-04-09：继续把手机主流程做顺，并补页面自动刷新

本轮继续增强：

- Android 配网 App 新增 `Use Current Wi-Fi`
  - 当 Android 能提供当前连接的 SSID 时，可直接回填到 SSID 输入框
- Android 配网 App 在发送凭据后会启动短时间自动状态轮询
  - 减少用户反复手动点击 `Read Status`
  - 仍保留手动 `Read Status` 作为 fallback
- Android 结果区改为更适合验收的摘要格式：
  - `state`
  - `message`
  - `ssid`
  - `ip`
  - `wifi_interface`
  - `wpa_unit`
  - `error_code`
  - `apply_output`
  - `diagnostic_hint`
  - 原始 JSON
- `/provisioning` 页面增加自动刷新
  - 当 `/provisioning-status` 的 `updated_at` 变化时自动重载页面
  - 适合手机浏览器在配网过程中持续观察状态推进

本轮本地验证：

- Android `:app:assembleDebug` 再次通过
- `go test ./internal/api ./internal/provisioningclient` 再次通过
- 继续未出新镜像

### 7.1.7 2026-04-09：补上 APK 内主界面跳转，并把现有 WebUI 页面纳入今晚验收

本轮继续推进：

- Android 配网 App 已新增 APK 内主界面：
  - 新增 `MainInterfaceActivity`
  - 成功拿到 `connected + web_url` 后，会自动进入 APK 内的 `WebView`
  - 当前属于方案1：
    - 本质还是加载 `http://<T4_IP>:18080/`
    - 但在 APK 框架里呈现
- APK 内主界面已加入基础入口：
  - `Home`
  - `Library`
  - `Logs`
  - `Provisioning`
  - `Browser`
  - `Setup`
- Android manifest 已补：
  - `INTERNET`
  - `usesCleartextTraffic=true`
  - `MainInterfaceActivity`
- 今晚可一起验的页面框架现状已明确：
  - 播放 / 首页基础框架：`services/controld/web/templates/index.html`
  - 曲库页面基础框架：`services/controld/web/templates/library.html`
  - 它们当前不要求功能完整，但已具备可浏览的初版框架

本轮本地验证：

- Android `:app:assembleDebug` 再次通过
- 继续未出新镜像

### 7.1.8 2026-04-09：按今晚验收范围正式出包

本轮已产出今晚验收所需的 2 个包：

- Android APK：
  - `out/android-provisioning/lumelo-android-provisioning-20260409-mainui-debug.apk`
  - `sha256 = 1c440f70d224b94dda3d34ac4ddd0cb76f72d74c0c7335d0925e8afcd98f6df7`
- T4 开发镜像：
  - `out/t4-rootfs/lumelo-t4-rootfs-20260409-bleprov-ssh-btfix-mainui.img`
  - `sha256 = d520c3a693242ddcb794145d81db0c1113ad8b54f01f7d0325754f33e8cfd12b`

这次 T4 图包含：

- 开发态 SSH 默认开启
- 蓝牙 rfkill 时序修复
- BLE / Wi-Fi provisioning 状态文件与更细诊断
- 动态 Wi-Fi 接口适配

这次 APK 包含：

- `Use Current Wi-Fi`
- 自动状态轮询
- `connected` 后自动进入 APK 内主界面
- APK 内 `Home / Library / Provisioning / Logs` 基础入口

镜像离线验收结果：

- `verify-t4-lumelo-rootfs-image.sh` 通过
- `0 failure(s), 0 warning(s)`

### 7.1.9 2026-04-10：定位到 overlay 权限污染导致的非 root 系统服务启动失败

本轮不是新增功能，而是定位一类比预期更底层的镜像构成问题。

真机新增现场现象：

- `ip -4 addr show | grep inet` 只有：
  - `127.0.0.1/8`
- `ip route` 为空
- `systemctl status systemd-networkd` 显示：
  - `Failed to execute /usr/lib/systemd/systemd-networkd: Permission denied`
  - `status=203/EXEC`

已确认的根因：

- 不是“网卡模块没装”
- 也不是 `20-wired-dhcp.network` 名称匹配错误
- 真正问题是：
  - 宿主机工作区中的 `base/rootfs/overlay` 目录树整体权限为 `0700`
  - `build-t4-lumelo-rootfs-image.sh` 之前用 `rsync -a` 原样保留了这些权限
  - 结果把镜像中的 `/usr`、`/usr/lib`、`/etc` 等关键目录也污染成了 `0700`
  - `systemd-networkd` 这类以非 root 用户运行的系统服务因此无法遍历路径并执行二进制，直接报 `Permission denied`

离线复核已坐实：

- 当前已出包镜像中：
  - `/usr` 为 `0700`
  - `/usr/lib` 为 `0700`
  - `/etc` 为 `0700`
- 同一张镜像里的 `/usr/lib/systemd/systemd-networkd` 二进制本身其实是 `0755`
- 因此问题不在可执行文件本身，而在父目录不可遍历

本轮已修复源码侧制镜逻辑：

- `build-t4-lumelo-rootfs-image.sh`
  - 继续复制 overlay 内容
  - 但新增 `normalize_overlay_permissions()`
  - 不再把宿主机 overlay 的 `0700` 目录权限原样带进镜像
  - 对 overlay 对应目标路径统一归一化：
    - 目录 `0755`
    - 普通配置文件 `0644`
    - `usr/bin/lumelo-*` 与 `usr/libexec/lumelo/*` / `usr/libexec/product/*` 再显式修正为 `0755`

- `verify-t4-lumelo-rootfs-image.sh`
  - 新增关键目录可遍历性检查：
    - `/etc`
    - `/usr`
    - `/usr/lib`
  - 新增系统二进制检查：
    - `/usr/lib/systemd/systemd-networkd`
    - `/usr/lib/systemd/systemd-resolved`
  - 下次若再出现“目录权限污染”，离线验收阶段就会直接失败，不再流到真机

当前状态：

- 根因已经明确
- 源码侧修复已经完成
- 还未按这次修复重建新镜像
- 下次出包前，需要先把这次权限修复和蓝牙 bring-up 路线一起复核清楚，再统一产出下一张真机图

### 7.1.10 2026-04-10：Android APK 新增独立 BLE Test Scan 自检入口

本轮目标不是新增 Lumelo 功能，而是把“手机 APK 自己的 BLE 扫描 / GATT 连接链路是否正常”从 T4 板端问题中拆出来单独验证。

本轮已落地：

- Android APK 新增独立测试按钮：
  - `BLE Test Scan`
- 这条路径与 `Scan for Lumelo` 分离：
  - `Scan for Lumelo` 继续只筛选 `Lumelo` 设备
  - `BLE Test Scan` 会显示附近任意 BLE 设备
- 测试路径支持：
  - 选择任意 BLE 设备
  - 发起通用 GATT 连接
  - 完成服务发现
  - 在结果区显示：
    - 设备名 / 地址
    - service 数量
    - 前若干个 service UUID 与 characteristic 数量
- 这样今晚可直接用耳机等 BLE 设备验证：
  - 手机扫描是否正常
  - App 的 BLE 连接是否正常
  - 若 T4 仍扫不到，可更明确判定问题在板端，而不是 APK 扫描链

本轮本地验证：

- Android `:app:assembleDebug` 通过
- 真机安装通过：
  - 设备：`PJZ110`
  - `adb install -r` 返回 `Success`

本轮 APK 产物：

- `out/android-provisioning/lumelo-android-provisioning-20260410-bletest-debug.apk`
- `sha256 = 2ba8bce57daba11323d61a2ed4ca0d134ab35f834ed7e73d16e697a9a6e2e68f`

本轮未出新 T4 镜像：

- 继续遵守“真机镜像先讨论清楚再统一出包”的流程

### 7.1.11 2026-04-10：重建 permission-fix 调试图，离线确认修复 networkd/dbus/bluez 权限链路

本轮目标：

- 不再让真机继续停留在已知损坏的旧镜像上
- 基于 `7.1.9` 已确认的根因，产出一张修复 overlay 权限污染的新调试图
- 在出包前离线确认关键目录权限、`systemd-networkd`、`systemd-resolved`、蓝牙 / Wi-Fi 配网底座和 SSH 配置都已落入镜像

本轮已完成：

- 重新构建新镜像：
  - [lumelo-t4-rootfs-20260410-permfix.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260410-permfix.img)
- 生成摘要：
  - [lumelo-t4-rootfs-20260410-permfix.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260410-permfix.img.sha256)
  - `sha256 = de092c64cb5fcd996b971abe37e029c728b0f775cef824c712a11bcd759615df`

这张图相对上一轮的关键修复点：

- 不再把宿主机 overlay 中的错误 `0700` 目录权限复制进目标 rootfs
- 关键系统目录在镜像中已恢复为非 root 系统服务可遍历：
  - `/etc = 0755`
  - `/usr = 0755`
  - `/usr/lib = 0755`
- BlueZ 要求严格的配置目录权限也已收口：
  - `/etc/bluetooth = 0555`

离线验收结果：

- `verify-t4-lumelo-rootfs-image.sh` 对新图验证通过
- 结果：
  - `0 failure(s), 0 warning(s)`
- 本轮特别确认通过的关键项包括：
  - `/etc`、`/usr`、`/usr/lib` 权限
  - `/etc/bluetooth` 权限
  - `/usr/lib/systemd/systemd-networkd`
  - `/usr/lib/systemd/systemd-resolved`
  - `20-wired-dhcp.network`
  - 蓝牙 rfkill unblock drop-in
  - `lumelo-wifi-provisiond` 与 BLE 配网底座
  - `ssh.service` 与开发态 root 登录策略

本轮阶段判断：

- 已修掉上一张图里导致 `networkd`、`dbus`、`bluez` 连锁失败的同类权限污染问题
- 这张图现在适合重新进入真机验证
- 但真机层面的最终结论仍需下一轮上板验证：
  - 有线网络是否正常拿到 DHCP
  - `bluetooth.service` 是否稳定拉起
  - 手机是否能重新扫到 `Lumelo T4`

产物整理：

- `out/t4-rootfs` 中旧的历史调试图已删除
- 当时仅保留：
  - `lumelo-t4-rootfs-20260410-permfix.img`
  - `lumelo-t4-rootfs-20260410-permfix.img.sha256`
- `SeeDisk/exFAT` 仍可能出现 `._*` 侧写文件，这不代表旧镜像本体仍在占用空间

### 7.1.12 2026-04-10：定位到 FriendlyELEC 蓝牙 UART attach 链缺失，重建 btattach 调试图

本轮新增真机结论：

- `20260410-permfix` 已经修复了 `networkd/dbus/bluez` 的目录权限污染问题
- 但真机仍然扫不到 BLE 设备
- 同时用户现场反馈网络已经起来，说明问题已不再是系统级权限崩坏，而是更聚焦的蓝牙 bring-up 缺口

进一步离线对比 FriendlyELEC 官方底图后，确认到关键差异：

- 官方底图不仅有 `bluetoothd` / `bluetoothctl`
- 还额外带了：
  - `/usr/bin/hciattach.rk`
  - `/etc/init.d/init_bt_uart.sh`
- 且官方脚本核心动作是：
  - 等待 `bcmdhd`
  - `rfkill unblock bluetooth`
  - 调用：
    - `hciattach.rk /dev/ttyS0 bcm43xx 1500000`

这说明 `NanoPC-T4` 的板级蓝牙 bring-up 不是“只启动 bluez 即可”，还依赖一条 Rockchip / Broadcom 的 UART attach 链。

本轮已完成的源码修复：

- `build-t4-lumelo-rootfs-image.sh`
  - 制镜时从 FriendlyELEC 底图复制 `/usr/bin/hciattach.rk`
  - 若底图缺失该 helper，制镜直接失败，不再产出缺功能镜像
- 新增 `lumelo-bluetooth-uart-attach.service`
- 新增 `/usr/libexec/lumelo/bluetooth-uart-attach`
  - 等待 `bcmdhd`
  - 解锁蓝牙 rfkill
  - 调用 `hciattach.rk /dev/ttyS0 bcm43xx 1500000`
- `bluetooth.service` 新增 drop-in：
  - `Requires=lumelo-bluetooth-uart-attach.service`
  - `After=lumelo-bluetooth-uart-attach.service`
- `lumelo-t4-report` 同步增强：
  - 会收集新 UART attach unit / drop-in / helper
- `verify-t4-lumelo-rootfs-image.sh` 同步增强：
  - 校验 `hciattach.rk`
  - 校验 UART attach unit / wrapper / drop-in / enablement

本轮新镜像：

- [lumelo-t4-rootfs-20260410-btattach.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260410-btattach.img)
- [lumelo-t4-rootfs-20260410-btattach.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260410-btattach.img.sha256)
- `sha256 = 0bc0f7dba6646dfeade0e05e33ec99e3c0bca59d386a51109f6eabdad3e35c15`

本轮离线验收结果：

- `0 failure(s), 0 warning(s)`

本轮阶段判断：

- 这张 `btattach` 图相比 `permfix` 图，补上了更接近官方板级蓝牙 bring-up 的关键链路
- 下一轮真机验证重点应直接聚焦：
  - 开机后手机是否能扫到 `Lumelo T4`
  - `lumelo-bluetooth-uart-attach.service` 是否成功
  - `bluetooth.service` 是否进入 `active (running)`
  - `lumelo-wifi-provisiond.service` 是否随之进入运行态

产物整理：

- `permfix` 图已被 `btattach` 图取代，不再保留
- 当前实际保留：
  - `lumelo-t4-rootfs-20260410-btattach.img`
  - `lumelo-t4-rootfs-20260410-btattach.img.sha256`

### 7.1.13 2026-04-10：收紧 networkd / resolved 网络行为，重建 netfix 调试图

本轮背景：

- 用户家庭网络为双路由结构：
  - 光猫路由：`192.168.71.x`
  - 客厅路由：`192.168.1.x`
- `T4` 实际插在客厅路由 `LAN` 侧
- 但 `btattach` 图上，用户现场观察到 `T4` 拿到了 `192.168.71.5`
- 官方固件在同一环境下通常能拿到 `192.168.1.x`

本轮判断：

- 没有发现任何写死 `192.168.71.x`、静态网关或预置 Wi-Fi 网络的配置
- 但现有 bring-up 镜像在网络层面仍然过于“活跃”：
  - 有线 / 无线 `.network` 配置都启用了 `LinkLocalAddressing=yes`
  - 同时启用了 `LLMNR=yes`
  - 同时启用了 `MulticastDNS=yes`
  - `lumelo-wifi-apply` 还会重启整套 `systemd-networkd`
- 在 `TL-XDR1860` 这类 `WAN/LAN 自适应（盲插）` 路由环境里，这些行为很可能会增加口角色误判或 DHCP 重新协商的不确定性

本轮已完成的修复：

- `20-wired-dhcp.network`
  - 改为：
    - `LinkLocalAddressing=no`
    - `LLMNR=no`
    - `MulticastDNS=no`
- `30-wireless-dhcp.network`
  - 同步改为：
    - `LinkLocalAddressing=no`
    - `LLMNR=no`
    - `MulticastDNS=no`
- `resolved.conf.d/lumelo.conf`
  - 改为：
    - `LLMNR=no`
    - `MulticastDNS=no`
- `lumelo-wifi-apply`
  - 不再重启整套 `systemd-networkd`
  - 改为只对目标 Wi-Fi 接口执行：
    - `networkctl reload`
    - `networkctl reconfigure <iface>`

本轮新镜像：

- [lumelo-t4-rootfs-20260410-netfix.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260410-netfix.img)
- [lumelo-t4-rootfs-20260410-netfix.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260410-netfix.img.sha256)
- `sha256 = 89e6cea794982655724f57e4e55d9810e3aa343d77fe75dc263cc93f35cde447`

本轮离线验收结果：

- `0 failure(s), 0 warning(s)`
- 已额外确认：
  - wired `LinkLocalAddressing=no`
  - wired `LLMNR=no`
  - wired `MulticastDNS=no`
  - wireless `LinkLocalAddressing=no`
  - wireless `LLMNR=no`
  - wireless `MulticastDNS=no`
  - `resolved` 的 `LLMNR=no`
  - `resolved` 的 `MulticastDNS=no`
  - 蓝牙 UART attach 链仍完整保留

本轮阶段判断：

- 这张 `netfix` 图是目前同时包含：
  - 权限污染修复
  - 蓝牙 UART attach 修复
  - 网络静音 / DHCP 行为收紧
  的最新主验证图

产物整理：

- `btattach` 图已被 `netfix` 图取代，不再保留
- 当前实际保留：
  - `lumelo-t4-rootfs-20260410-netfix.img`
  - `lumelo-t4-rootfs-20260410-netfix.img.sha256`

### 7.1.14 2026-04-11：修复 BLE 注册超时与 SSH host key 生成，重建 remotefix 调试图

本轮新增现场结论：

- `netfix` 图已让 `T4` 回到正确的 `192.168.1.x` 网段
- 但手机仍扫不到 `Lumelo T4`
- 同时远程验证发现：
  - `ssh` 端口 `22` 未监听
  - WebUI `18080` 已可用
  - `/logs.txt` 已能远程暴露更细的 bring-up 日志

通过远程日志进一步定位到两个新的明确问题：

- 蓝牙链路：
  - `lumelo-bluetooth-uart-attach.service` 已成功
  - `bluetooth.service` 已成功
  - 真正失败点在 `lumelo-wifi-provisiond.service`
  - BlueZ 侧关键报错：
    - `src/gatt-database.c:client_ready_cb() No object received`
    - `RegisterApplication / RegisterAdvertisement` 最终超时 `NoReply`
- SSH 链路：
  - `ssh.service` 启动前缺失 host keys
  - 发行版自带 `sshd-keygen.service` 因 `ConditionFirstBoot=yes` 被跳过
  - 导致 `ssh.service` 反复失败、`22` 端口不可用

本轮已完成的修复：

- `bluetooth-wifi-provisiond`
  - 为 DBus 服务显式申请独立 bus name：`org.lumelo.provisioning`
  - 将 `RegisterApplication` / `RegisterAdvertisement` 改为异步注册
  - 让 `GLib.MainLoop` 在注册阶段就能处理 BlueZ 回调，避免同步注册过程中因对象树查询超时而死锁
- 新增 `ssh.service.d/10-lumelo-hostkeys.conf`
  - 在 `ssh.service` 启动前执行：
    - `ssh-keygen -A`
  - 不再依赖 `sshd-keygen.service` 的 first-boot 条件
- `verify-t4-lumelo-rootfs-image.sh`
  - 新增 `ssh-keygen -A` drop-in 检查
- `lumelo-t4-report`
  - 同步收集 `ssh.service.d` drop-in，便于后续远程排障

本轮新镜像：

- [lumelo-t4-rootfs-20260411-remotefix.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260411-remotefix.img)
- [lumelo-t4-rootfs-20260411-remotefix.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260411-remotefix.img.sha256)
- `sha256 = 779797517594f1b29320c18370afd5bfd876ec3be5df05ad4fdd97c40caa828b`

本轮离线验收结果：

- `0 failure(s), 0 warning(s)`
- 已确认：
  - 网络静音修复仍保留
  - 蓝牙 UART attach 链仍保留
  - `ssh_enabled = true`
  - `ssh.service` 已有 `ssh-keygen -A` 启动前策略

本轮阶段判断：

- 这张 `remotefix` 图是目前最适合“无显示器 / 通过 SSH + WebUI 排障”的最新主验证图
- 下一轮真机重点应同时验证：
  - `22` 端口是否恢复
  - 手机是否终于能扫到 `Lumelo T4`

产物整理：

- `netfix` 图已被 `remotefix` 图取代，不再保留
- 当前实际保留：
  - `lumelo-t4-rootfs-20260411-remotefix.img`
  - `lumelo-t4-rootfs-20260411-remotefix.img.sha256`

### 7.1.15 2026-04-11：整理交接文档，收口当前验证入口与出包协作规则

本轮主要不是新增运行时代码，而是把最近几轮已经形成的结论正式收口到文档中。

本轮已完成：

- 更新本日志的：
  - `当前未闭环事项`
  - `当前推荐下一步`
  - `当前交接入口`
- 明确当前最新主验证图为：
  - `lumelo-t4-rootfs-20260411-remotefix.img`
- 明确当前最新已知真机事实：
  - `20260410-netfix` 已让 `T4` 回到 `192.168.1.x`
  - `18080` 已可访问
  - BLE 扫描仍失败
  - `22` 端口仍未通过现场验证
- 明确当前最新 APK 产物与边界：
  - 目录内最新 APK 仍为 `20260410-bletest`
  - 最近 T4-only 轮次没有继续改动 APK 代码
- 将已形成长期结论的 bring-up 约束同步进产品手册：
  - V1 可用 BLE 做初次联网 / Wi-Fi provisioning，但 steady-state 主交互仍是 WebUI
  - T4 bring-up 图必须保留板级蓝牙 UART attach 链
  - bring-up 默认网络行为应保持保守客户端姿态
  - `/healthz`、`/provisioning-status`、`/logs`、`/logs.txt` 作为默认 bring-up 诊断入口
- 重写交接记忆文件，明确：
  - 新窗口先读哪些文档
  - 项目路径、环境与工具
  - 当前未闭环事项
  - “先分析 bug，再统一出包”的协作规则

本轮阶段判断：

- 当前下一窗口最自然的接手点，不再是重新梳理历史，而是：
  - 先读手册和日志
  - 直接拿 `20260411-remotefix` 做下一轮真机验证
  - 先确认 SSH 与 BLE 是否恢复

### 7.1.16 2026-04-11：真机复验确认 DBus policy 缺失，补上 BLE provisioning bus-name 授权

本轮新增现场与验证结论：

- Android 真机已通过 `adb` 复验：
  - App 可正常拉起
  - `BLUETOOTH_SCAN` / `BLUETOOTH_CONNECT` / 定位权限均已授予
  - `BLE Test Scan` 能扫到附近其他 BLE 设备
  - `Scan for Lumelo` 仍为 `No Lumelo device found`
- T4 当前 `18080` 诊断接口进一步暴露出更直接的板端根因：
  - `/healthz` 与 `/provisioning-status` 显示 provisioning 状态文件缺失
  - `/logs.txt` 持续出现：
    - `org.freedesktop.DBus.Error.AccessDenied`
    - `Connection ... is not allowed to own the service "org.lumelo.provisioning"`
- 因此本轮结论已进一步收缩为：
  - 手机端通用 BLE 扫描链路正常
  - 当前主故障仍在 T4 板端
  - 真正缺的是 `org.lumelo.provisioning` 对应的 system bus policy，而不是继续先改 APK

本轮已完成的修复：

- rootfs overlay 新增：
  - `/etc/dbus-1/system.d/org.lumelo.provisioning.conf`
  - 允许 root 占用 `org.lumelo.provisioning`
  - 允许默认上下文向该服务发消息
- `verify-t4-lumelo-rootfs-image.sh` 已增强：
  - 新增 BLE provisioning DBus policy 存在性检查
  - 新增 `allow own="org.lumelo.provisioning"` 检查
  - 新增 `allow send_destination="org.lumelo.provisioning"` 检查

本轮额外离线结论：

- 更新后的离线验收脚本反查现有：
  - [lumelo-t4-rootfs-20260411-remotefix.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260411-remotefix.img)
- 结果显示：
  - 该图确实缺少 `/etc/dbus-1/system.d/org.lumelo.provisioning.conf`
  - 旧验收脚本之所以放过，是因为此前根本没有检查这项
- 同时也确认：
  - 本地这张 `remotefix` 图离线仍显示 `ssh_enabled = true`
  - 这与现场 WebUI 暴露出的 `ssh_enabled = false` 不一致
  - 下一轮若再次出现该现象，应优先怀疑现场板子实际运行产物与本地镜像不一致

本轮阶段判断：

- `20260411-remotefix` 不再适合作为下一轮真机主验证图
- 下一步应先重建一张带 DBus policy 的新图，再继续 BLE 与 SSH 回归

### 7.1.17 2026-04-11：补齐 DBus policy、MTU 兜底与 local-mode 曲库依赖，产出 provisionfix 新图

本轮新增修复：

- T4 rootfs：
  - 新增 `/etc/dbus-1/system.d/org.lumelo.provisioning.conf`
  - 允许 `org.lumelo.provisioning` 的 bus-name 申请与默认消息发送
- `local-mode.target`
  - 补上 `media-indexd.service`
  - 避免主界面曲库页因 `library.db` 从未创建而长期离线
- Android 配网 App：
  - BLE 连接建立后主动请求更大的 MTU
  - 若 `requestMtu()` 回调超时，自动回退到服务发现
  - 这样可降低“能连上但 Wi-Fi 凭据 JSON 写不进 characteristic”的风险

本轮验证结果：

- Android：
  - 新 APK 已命令行构建成功
  - 已 `adb install -r`
  - `am start -W` 返回 `Status: ok`
- 本地服务烟测：
  - `playbackd` 已能在本机建立 Unix socket
  - `PING` / `STATUS` / `QUEUE_SNAPSHOT` 均有正常响应
  - `media-indexd` 在可写 state/cache 目录下能成功创建 `library.db`
- 更新后的离线验收脚本现在会直接拦住：
  - 缺少 BLE provisioning DBus policy 的镜像
  - `local-mode.target` 未引用 `media-indexd.service` 的镜像

本轮新产物：

- T4 镜像：
  - [lumelo-t4-rootfs-20260411-provisionfix.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260411-provisionfix.img)
  - [lumelo-t4-rootfs-20260411-provisionfix.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260411-provisionfix.img.sha256)
  - `sha256 = 360bad64e4b9c6ebe4eafad1f2aa018de483d358fc5f463d31dd723bddce710a`
- Android APK：
  - [lumelo-android-provisioning-20260411-mtufix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260411-mtufix-debug.apk)
  - [lumelo-android-provisioning-20260411-mtufix-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260411-mtufix-debug.apk.sha256)

本轮离线验收结果：

- 新图：
  - `0 failure(s), 0 warning(s)`

本轮阶段判断：

- 代码层面当前最值得修的已知阻塞点都已补入新包
- 下一步不再是继续本地补洞，而是直接拿 `provisionfix` 图与 `mtufix` APK 进入真机回归

### 7.1.18 2026-04-12：用 eMMC 官方系统建立 AP6356S 无线金样，回滚偏离官方的构镜假设

本轮新增现场结论：

- 用户已让 `T4` 从 eMMC 中的官方系统启动
- 运行态确认不再是 Lumelo 自定义图：
  - `/proc/cmdline` 为 `storagemedia=emmc`
  - 主机名为 `NanoPC-T4`
  - `18080` 不再响应
  - `8443` / `6600` 上运行 `myMPD` / `MPD`
- 由于这台官方系统的 SSH 口令未知，本轮通过 `myMPD` 的脚本接口做了只读取证

本轮确认到的官方金样：

- 无线驱动链：
  - 已加载 `bcmdhd`
  - 已加载 `bluetooth`
  - 已加载 `hci_uart`
  - 已加载 `btbcm`
  - 未加载 `brcmfmac`
- 运行时拓扑：
  - `/sys/class/bluetooth/hci0` 挂在 `ttyS0`
  - `/sys/class/net/wlan0` 存在
- 官方板级文件：
  - `/etc/firmware/BCM4356A2.hcd`
  - `/etc/modprobe.d/bcmdhd.conf`
  - `/usr/bin/hciattach.rk`
- 官方蓝牙启动脚本：
  - `/etc/init.d/init_bt_uart.sh`
  - 核心顺序是：
    - 等待 `/sys/module/bcmdhd`
    - `rfkill unblock bluetooth`
    - `echo 1 > /sys/class/rfkill/rfkill0/state`
    - `rm -f /dev/rfkill`
    - `hciattach.rk /dev/ttyS0 bcm43xx 1500000`

由此确认此前 Lumelo 镜像存在一条关键偏差：

- 构镜链把 `NanoPC-T4` 的板级无线支持逐步带向了：
  - `brcmfmac`
  - `brcmfmac4356-sdio.*`
  - `/lib/firmware/brcm/BCM-0bb4-0306.hcd`
- 这与官方运行态的 `bcmdhd + BCM4356A2.hcd + ttyS0` 金样并不一致
- 也就解释了为什么现场持续出现：
  - `Patch not found, continue anyway`
  - `brcmfmac ... error while changing bus sleep state -110`
  - `failed backplane access over SDIO`

本轮已完成的源码修正：

- `build-t4-lumelo-rootfs-image.sh`
  - 改为从官方底图复制：
    - `/etc/firmware/`
    - `/etc/modprobe.d/bcmdhd.conf`
    - `/usr/bin/hciattach.rk`
  - 若缺少 `BCM4356A2.hcd` 或 `bcmdhd.conf`，制镜直接失败
  - 不再把 `/etc/firmware` 重写成指向 `/lib/firmware` 的兼容 symlink
- `t4-bringup-postbuild.sh`
  - 去掉 `brcmfmac4356-sdio.*` 那套面向通用 `brcmfmac` 路线的补丁文件兼容逻辑
- `bluetooth-uart-attach`
  - 回到以 `/sys/module/bcmdhd` 为默认无线就绪标记
  - 对齐官方脚本中的 rfkill 解锁动作
- `verify-t4-lumelo-rootfs-image.sh`
  - 默认改查：
    - `/etc/firmware/BCM4356A2.hcd`
    - `/etc/modprobe.d/bcmdhd.conf`
    - `bcmdhd` alias / `op_mode`
    - attach helper 对 `bcmdhd` 的等待逻辑
- `controld`
  - 同步修复一直把 `ssh_enabled` 假报为 `false` 的状态读取问题

本轮额外说明：

- `myMPD` 的 systemd sandbox 较重：
  - `PrivateDevices=yes`
  - `ProtectKernelLogs=yes`
  - `ProtectProc=invisible`
- 因此通过该旁路无法可靠读取 `rfkill`、`dmesg`、D-Bus 或 HCI socket
- 这也是为什么本轮官方系统取证以模块、文件、sysfs 和启动脚本为主，而不是直接在官方运行态做完整 BLE 扫描实验

本轮阶段判断：

- `NanoPC-T4` 当前最重要的修复方向已经收敛
- 后续 `T4` 出包与验收默认以这套官方金样为准
- 已基于这组修正重建新图：
  - [lumelo-t4-rootfs-20260412-v13.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v13.img)
  - [lumelo-t4-rootfs-20260412-v13.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v13.img.sha256)
  - `sha256 = b5beda830e13e0eada1779bfd2ef9f558049c345f4d66638042705e735b17c6d`
- 本轮本地验证：
  - `go test ./...` 在 `services/controld` 通过
  - `sh -n` 对制镜 / postbuild / 蓝牙 attach / 离线验收脚本通过
  - `verify-t4-lumelo-rootfs-image.sh` 对 `v13` 结果为：
    - `0 failure(s), 0 warning(s)`
  - 新增并跑通 `compare-t4-wireless-golden.sh`：
    - 对 `v13` 与官方底图的无线关键资产比对结果为：
      - `0 failure(s), 0 warning(s)`
  - 进一步从 `v13` 镜像内核对确认：
    - `/etc/modprobe.d/bcmdhd.conf` 内容完整，包含 `op_mode=5` 与 `sdio:c*v02D0d4356*`
    - `bluetooth-uart-attach` 默认等待 `/sys/module/bcmdhd`
    - 镜像内 `BCM4356A2.hcd` 的 `sha256` 为：
      - `f1daa6ab28699b72c8e47a34f43c095941c9aa542d0a5f4b55baebc5fd1aae99`
    - 镜像内 `hciattach.rk` 的 `sha256` 为：
      - `6a1429246318616da349328b390438ccc3667ab709ffca6c03f960a86b2a6299`
    - 以上两项已与官方底图核对一致
- 本轮流程修正：
  - `sync-to-lumelodev-apfs.sh` 不再使用会误删目标 `out/` 的 `--delete-excluded`
  - 出包完成后再次同步时，会保留 `LumeloDev/out/` 中的镜像与 APK 制品
- 本轮文档修正：
  - `T4_Bringup_Checklist.md` 已从旧的 `brcmfmac / BCM-0bb4-0306.hcd` 假设切换到官方 `bcmdhd / BCM4356A2.hcd` 金样
  - `Development_Environment_README.md` 与 `T4_Bringup_Checklist.md` 已新增官方无线金样比对脚本入口
- 下一步回到手机真机复验：
  - 手机是否能扫到 `Lumelo T4`
  - `ssh root@<T4_IP>` 是否仍正常
  - BLE 配网后 Wi-Fi 凭据是否能成功下发

### 7.1.19 2026-04-12：推进 APK V1 诊断版，补齐 Raw Scan、构建信息与导出日志

本轮 APK 侧新增：

- `BLE Test Scan` 升级为 `Raw BLE Scan`
- 扫描结果新增摘要：
  - 设备总数
  - `UUID matched`
  - `Name matched`
  - 当前选中设备
- 原始扫描结果列表现在会展示：
  - `MAC`
  - `RSSI`
  - `Local Name`
  - `Device Name`
  - `Service UUIDs`
  - `Manufacturer Data`
- 首屏新增：
  - `App version`
  - `build time`
  - `git short SHA`
- 新增：
  - `Export Diagnostics`
  - 以文本方式导出当前扫描、连接、状态与 debug log
- 页面布局补为 `ScrollView`
  - 避免诊断元素增多后在手机上被截断

本轮本地验证：

- Android debug 构建通过：
  - `:app:assembleDebug`
- 新 APK 产物：
  - [lumelo-android-provisioning-20260412-rawscan-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-rawscan-debug.apk)
  - [lumelo-android-provisioning-20260412-rawscan-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-rawscan-debug.apk.sha256)
  - `sha256 = e04d5f1572b748ac37fcdaa946dccbf43cbb03d127c2033a0e6ddf2c0dfa64ad`

本轮限制：

- 构建成功时当前宿主机未检测到已连接 Android 设备
- 因此本轮未执行：
  - `adb install -r`
  - 真机前台拉起

本轮阶段判断：

- APK `V1` 里最值钱的诊断可见性已经基本补到位
- 下一步最值得做的是拿这版新 APK 回到真机现场，配合 `v13` 镜像看：
  - `Lumelo Scan` 是否仍扫不到
  - `Raw BLE Scan` 是否能看到板端空口广播
  - 导出的诊断文本是否足够支持后续定位

### 7.1.20 2026-04-12：官方金样 Wi-Fi 用户态差异落档，并切换到经典蓝牙主通道

本轮先在官方 `rk3399-sd-debian-trixie-core-4.19-arm64-20260319` 金样上继续取证。
已确认：

- 官方板级无线链路是：
  - `bcmdhd`
  - `/system/etc/firmware/fw_bcm4356a2_ag.bin`
  - `/system/etc/firmware/nvram_ap6356.txt`
  - `/etc/firmware/BCM4356A2.hcd`
- 官方用户态 Wi-Fi 栈不是 `systemd-networkd + iw`，而是：
  - `NetworkManager`
  - `ifupdown`
  - `dhcpcd-base`
  - `wireless-tools`
  - `wpa_supplicant`
- 官方真机运行态还额外确认到：
  - `nmcli general status` 正常
  - `wlan0` 与 `p2p-dev-wlan0` 同时存在
  - `/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf` 存在

因此本轮把 Wi-Fi 改造先落到这几处：

- [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)
  - 新增 `LUMELO_WIFI_BACKEND=auto|networkmanager|wpa_supplicant`
  - `auto` 下优先使用 active 的 `NetworkManager`
  - 否则回退到当前 `wpa_supplicant + systemd-networkd`
  - 无线接口探测修正为：
    - 优先 `nmcli`
    - 再看 `iw`
    - 最后看 `/sys/class/net/*/wireless`
    - 显式跳过 `p2p-dev*`
- [lumelo-t4-report](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-t4-report)
  - 新增 `nmcli` 状态与 `NetworkManager` 配置采样
- overlay 新增官方 `NetworkManager` 基线配置：
  - [NetworkManager.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/NetworkManager.conf)
  - [12-managed-wifi.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/12-managed-wifi.conf)
  - [99-unmanaged-wlan1.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf)
  - [disable-random-mac-during-wifi-scan.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf)
  - [interfaces](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/network/interfaces)

蓝牙主通道也在这一轮正式切换方向：

- 官方金样在接好天线后，经典蓝牙可被手机系统蓝牙设置页发现
- `Raw BLE Scan` 仍不能稳定看到板子
- 因此当前决定：
  - 经典蓝牙 `RFCOMM / SPP` 作为主配网通道
  - `Raw BLE Scan` 保留为诊断工具

对应落地：

- 新增板端经典蓝牙 daemon：
  - `/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond`
- `lumelo-wifi-provisiond.service` 已切到新 daemon
- Android 端新增：
  - [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)
- `MainActivity.java` 已把：
  - `Lumelo` 扫描切到经典蓝牙
  - `Raw BLE Scan` 保留成诊断入口

本轮本地验证：

- `python3 -m py_compile` 通过：
  - `classic-bluetooth-wifi-provisiond`
- `sh -n` 通过：
  - `lumelo-wifi-apply`
  - `lumelo-t4-report`
  - `lumelo-bluetooth-provisioning-mode`
  - `build-t4-lumelo-rootfs-image.sh`
  - `verify-t4-lumelo-rootfs-image.sh`
- Android debug 构建通过：
  - `:app:assembleDebug`
- 新 APK 产物：
  - [lumelo-android-provisioning-20260412-classicbt-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-classicbt-debug.apk)
  - [lumelo-android-provisioning-20260412-classicbt-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-classicbt-debug.apk.sha256)
  - `sha256 = 18f23235c10beeade0640c8a17b893430ede1e49dae73c4bc68a7762e3742b31`
- Android 真机验证：
  - `adb install -r` 成功
  - `am start -W -n com.lumelo.provisioning/.MainActivity` 成功
  - 进程已正常拉起

### 7.1.21 2026-04-12：产出 v14，合入经典蓝牙主通道与 Wi-Fi 双路径兼容

本轮目标是把这一轮的两条主线一起落成可烧录镜像：

- 经典蓝牙 `RFCOMM / SPP` 主配网通道
- 参考官方金样后的 Wi-Fi 用户态兼容底座

本轮已落地：

- 新 rootfs 产物：
  - [lumelo-t4-rootfs-20260412-v14.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v14.img)
  - [lumelo-t4-rootfs-20260412-v14.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v14.img.sha256)
  - `sha256 = 699f819ebd450baf0a5fa1d01d35b77d767ab019f3905904c1b7ccf9ab80cb8f`
- Wi-Fi 改造：
  - [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)
    - 支持 `LUMELO_WIFI_BACKEND=auto|networkmanager|wpa_supplicant`
    - `auto` 下优先走 active 的 `NetworkManager`
    - 无线接口探测改为：
      - 优先 `nmcli`
      - 再看 `iw`
      - 最后看 `/sys/class/net/*/wireless`
      - 显式跳过 `p2p-dev*`
  - overlay 新增官方 `NetworkManager` 基线配置：
    - [NetworkManager.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/NetworkManager.conf)
    - [12-managed-wifi.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/12-managed-wifi.conf)
    - [99-unmanaged-wlan1.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf)
    - [disable-random-mac-during-wifi-scan.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf)
    - [interfaces](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/network/interfaces)
  - [lumelo-t4-report](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-t4-report)
    现在会采：
    - `nmcli general status`
    - `nmcli device status`
    - `NetworkManager` 相关配置
- 离线验收增强：
  - [verify-t4-lumelo-rootfs-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/verify-t4-lumelo-rootfs-image.sh)
    现在会检查 `NetworkManager` 基线文件

本轮本地验证：

- `sh -n` 通过：
  - `lumelo-wifi-apply`
  - `lumelo-t4-report`
  - `verify-t4-lumelo-rootfs-image.sh`
- 额外行为验证：
  - 通过 stub `nmcli` 测试确认：
    - 在同时存在 `wlan0` 与 `p2p-dev-wlan0` 的场景下
    - `lumelo-wifi-apply` 会正确选择 `wlan0`
- `v14` 离线验收结果：
  - `0 failure(s), 0 warning(s)`
- `v14` 官方无线金样比对结果：
  - `0 failure(s), 0 warning(s)`
  - `BCM4356A2.hcd`
  - `hciattach.rk`
  - `bcmdhd.conf`
  - `fw_bcm4356a2_ag.bin`
  - `nvram_ap6356.txt`
    已和官方底图对齐

当前阶段判断：

- `v14` 已经是这一轮可以直接上板验证的主图
- 下一步真机优先看：
  - 手机能否在 `Lumelo Scan` 中扫到 `Lumelo T4`
  - 经典蓝牙连接后 Wi-Fi 凭据是否能成功写入
  - `status` 是否推进到 `connected`
  - 成功后 WebUI 是否可达

### 7.1.22 2026-04-12：经典蓝牙扫描真机打通，产出 v15 修正图

这一轮继续沿“经典蓝牙主通道”往前推，并把真机验证从“系统蓝牙可见”推进到“APK 内经典扫描可见”。

先在官方 `rk3399-sd-debian-trixie-core-4.19-arm64-20260319` 金样上做了真机回归：

- 接好天线后，`NanoPC-T4` 已能被安卓系统蓝牙设置页发现
- 但 APK 当时的 `Lumelo Scan` 仍显示：
  - `Classic Bluetooth scan | devices=0 | uuidMatch=0 | nameMatch=0`
- 继续对照官方金样与手机现场行为后确认：
  - 不是手机型号问题
  - 不是经典蓝牙不可见
  - 而是 APK 侧经典蓝牙发现结果处理还有一层 bug

本轮定位出的两个关键问题：

- Android 端经典蓝牙扫描最初只依赖 `device.getName()`
  - 但官方金样上设备名称会晚于首个 `ACTION_FOUND` 到达
  - 后续是通过 `ACTION_NAME_CHANGED` 才补齐 `NanoPC-T4`
- 板端经典蓝牙 daemon 的无线接口探测还沿用了旧逻辑
  - 在存在 `wlan0` 与 `p2p-dev-wlan0` 的场景下
  - 仍有把 `p2p-dev-wlan0` 误判成主接口的风险

对应修复已落地：

- Android 端：
  - [MainActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java)
    现在同时处理：
    - `BluetoothDevice.ACTION_FOUND`
    - `BluetoothDevice.ACTION_NAME_CHANGED`
  - 经典蓝牙结果改为按 `MAC` 合并观察项
  - 若后续名称事件命中 `Lumelo` 过滤条件，会更新既有条目并重建列表
- 板端：
  - [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
    的无线接口探测已改为：
    - 优先 `nmcli`
    - 再看 `iw`
    - 最后看 `/sys/class/net`
    - 显式跳过 `p2p-dev*` 与 `lo`

本轮真机验证结果：

- 手机重新安装最新 APK 后：
  - `Lumelo Scan` 已能在官方金样上扫到：
    - `NanoPC-T4 (C0:84:7D:1F:37:C7)`
  - 扫描摘要为：
    - `Classic Bluetooth scan | devices=1 | uuidMatch=0 | nameMatch=1`
- 这说明：
  - 手机型号没有问题
  - APK 经典蓝牙扫描主链已经打通
  - 接好天线后，`NanoPC-T4` 的经典蓝牙发现链路可用
- 官方金样仍不能完成 `CONNECT -> provisioning` 全链路
  - 因为官方系统不提供 `Lumelo` 自己的经典蓝牙 `RFCOMM` provisioning service
  - 所以这里只能验证“发现链路”和手机兼容性

本轮新增 APK 产物：

- [lumelo-android-provisioning-20260412-classicscanfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-classicscanfix-debug.apk)
- [lumelo-android-provisioning-20260412-classicscanfix-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-classicscanfix-debug.apk.sha256)
- `sha256 = 9f550c031f7cb9ab71d06f5da537fa4384c3b4624d556270ac8c6df34439341a`

为把板端 `p2p-dev-wlan0` 风险一并带进可烧录镜像，本轮重新出图：

- 新 rootfs 产物：
  - [lumelo-t4-rootfs-20260412-v15.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v15.img)
  - [lumelo-t4-rootfs-20260412-v15.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v15.img.sha256)
  - `sha256 = 1071ebf9d8eaf52433da2a9d68b910fb1ff4d2ff3f3fc973c0e9d0c359a19b7c`

本轮本地验证：

- `python3 -m py_compile` 通过：
  - `classic-bluetooth-wifi-provisiond`
- Android debug 构建通过：
  - `:app:assembleDebug`
- Android 真机验证通过：
  - `Lumelo Scan` 在官方金样上已能看到 `NanoPC-T4`
- `v15` 离线验收结果：
  - `0 failure(s), 0 warning(s)`
- `v15` 官方无线金样比对结果：
  - `0 failure(s), 0 warning(s)`

当前阶段判断：

- 经典蓝牙主通道的“手机扫描能否发现板子”这一关已经过了
- 下一步主验证对象应切回 `Lumelo v15` 真机镜像
- 上板后优先看：
  - `Lumelo Scan` 是否能扫到 `Lumelo T4`
  - `CONNECT` 后是否能读到 `device_info`
  - Wi-Fi 凭据是否能写入并触发 `apply`
  - `status` 是否推进到 `connected`
  - `WebView` 是否能稳定进入主界面

### 7.1.23 2026-04-12：经典蓝牙配网真机闭环、WebView 恢复修复与 `v16` 出包

本轮现场真机结论：

- 板端经典蓝牙 `RFCOMM` 配网主链已在真机上跑通：
  - 手机可扫描到板子
  - 连接可成功
  - `device_info` 可读
  - 热点 `isee_test` 场景下，Wi-Fi 凭据成功下发
  - T4 最终状态推进到 `connected`
  - T4 实际拿到 Wi-Fi 地址：
    - `192.168.43.170`
- WebUI 本身没有服务问题，之前手机内打不开的根因是：
  - 手机与 T4 不在同一可互通网络
  - 或 App 停留在切网前的错误页
- 手机切回与 T4 同一热点后，系统浏览器可直接打开：
  - `http://192.168.43.170:18080/`
- 主界面、曲库页、配网页已确认可打开：
  - `/`
  - `/library`
  - `/provisioning`

本轮修复：

- 修复 [MainInterfaceActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainInterfaceActivity.java) 中的切网恢复崩溃：
  - 根因是 `ConnectivityManager.NetworkCallback` 运行在 `ConnectivityThread`
  - 旧实现直接在该线程更新 `TextView`
  - 触发 `CalledFromWrongThreadException`
  - 现已统一切回主线程执行恢复逻辑
- 该修复已真机验证通过：
  - 断网后 App 会停留在错误页
  - 重新切回 `isee_test` 后会自动恢复
  - 已实际恢复到：
    - `http://192.168.43.170:18080/library`
- 继续补了 `WebView` 错误页下的网络状态补偿轮询：
  - 即使个别 Android 机型没有及时给出理想的网络回调
  - App 也会周期性重评当前网络状态
  - 一旦回到与 T4 可互通的网络，会主动重试恢复页面
- 板端蓝牙冷启动修复已完成并打入新图：
  - [bluetooth-uart-attach](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/bluetooth-uart-attach)
  - 不再把 `btmgmt info` 在“0 个控制器”时的返回误判成“已就绪”

本轮新增产物：

- 新 APK：
  - [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
  - [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk.sha256)
  - `sha256 = acd72ee79d511193df76e4e3a716b992dd714531517446e274d84cc01ea3982c`
- 新 rootfs：
  - [lumelo-t4-rootfs-20260412-v16.img](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img)
  - [lumelo-t4-rootfs-20260412-v16.img.sha256](/Volumes/LumeloDev/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260412-v16.img.sha256)
  - `sha256 = ea6d85c85335fa736ac73cf678456122a319a886d98277f88bdbeebeb8e7c160`

本轮本地验证：

- `v16` 离线验收：
  - `0 failure(s), 0 warning(s)`
- `v16` 官方无线金样比对：
  - `0 failure(s), 0 warning(s)`

当前剩余边界：

- `v16` 还需要上板做“无人工干预冷启动”真机回归
- 手机系统仍可能自动连回其他已保存 Wi-Fi，例如 `iSee`
  - 此时 App 不会崩
- 但会停留在错误页，直到回到与 T4 可互通的网络
- 真实曲库与真实播放链路仍未开始真机回归

### 7.1.24 2026-04-12：修复 Android 经典蓝牙 `RFCOMM` SDP 兼容性，并重新确认 `CONNECT -> device_info -> Wi-Fi -> WebView` 真机闭环

本轮现场背景：

- 在同一台真机上，系统蓝牙设置已经能看到并记住：
  - `lumelo (C0:84:7D:1F:37:C7)`
- App 经过前一轮扫描修复后，也已经能在 `Lumelo Scan` 中选中该设备
- 但点击 `CONNECT` 仍会立刻失败：
  - `Classic Bluetooth connect failed: A Bluetooth Socket failure occurred`

本轮根因定位：

- 手机蓝牙栈日志显示，标准 `SPP UUID` 连接路径在这台 Android 机型上没有拿到有效 `RFCOMM` channel：
  - `scn: 0`
  - `channel: -1`
- 这说明：
  - 板端经典蓝牙服务本身并没有挂
  - 真正失败点在 Android 端 `SPP / SDP` 解析兼容性
- 板端实现仍是正确的：
  - [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
  - 使用 `SPP UUID = 00001101-0000-1000-8000-00805F9B34FB`
  - 固定监听 `RFCOMM channel 1`

本轮修复：

- 在 [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java) 增加多段连接兜底：
  - 先尝试 `insecure RFCOMM + SPP UUID`
  - 再尝试 `secure RFCOMM + SPP UUID`
  - 若仍失败，再尝试固定 `channel 1` 的 `insecure RFCOMM`
  - 最后再尝试固定 `channel 1` 的 `secure RFCOMM`
- 同时补充了更细的连接日志，方便区分：
  - 是标准 `UUID` 路径失败
  - 还是固定 `channel` 路径成功

本轮真机验证结果：

- 现场手机型号：
  - `PJZ110`
- 重新安装最新 debug 构建后：
  - `Lumelo Scan` 可见 `lumelo`
  - `CONNECT` 已成功
  - App 能收到：
    - `hello`
    - `device_info`
    - `status`
- 现场继续完成经典蓝牙配网闭环：
  - 热点：
    - `SSID = isee_test`
    - `password = iseeisee`
  - Wi-Fi 凭据可成功下发并触发 `apply`
  - `/provisioning-status` 已推进到：
    - `connected`
  - T4 实际拿到：
    - `192.168.43.170`
  - `web_url` 为：
    - `http://192.168.43.170:18080/`
- APK 内部 `WebView` 已自动切到：
  - `http://192.168.43.170:18080/`
- 手机同网段复测通过：
  - `/healthz`
  - `/provisioning-status`
  - `/`
  - `/library`

本轮阶段结论：

- Android App 的经典蓝牙主链现在已经在这台现场手机上重新闭环：
  - `扫描 -> 连接 -> device_info -> status -> 发 Wi-Fi -> connected -> WebView`
- 当前 `RFCOMM` 兼容性问题已经不再阻塞 bring-up 主线

下一阶段已明确的安全改造目标：

- Wi-Fi 密码不应再以明文 JSON 经经典蓝牙 `RFCOMM` 发送
- 板端系统也不应再以明文 Wi-Fi 密码形式持久化存储
- 后续主任务切换为：
  - 设计并实现“非明文凭据传输”
  - 设计并实现“板端非明文凭据落盘 / 派生后存储”方案
  - 在保持当前配网闭环可用的前提下完成安全加固

### 7.1.25 2026-04-12：把经典蓝牙 Wi-Fi 凭据传输改为非明文协议，板端非明文持久化留待后续固件改造

本轮目标边界：

- 先解决：
  - Wi-Fi 密码不再以明文 JSON 经经典蓝牙应用层协议传输
- 暂不解决：
  - 板端 Wi-Fi 凭据的非明文持久化
  - `lumelo-wifi-apply` / `wpa_supplicant` 链路的最终安全存储策略

本轮协议改动：

- 经典蓝牙 `hello` 现在额外携带 `security` 协商信息
- App 与板端若都支持：
  - 默认改走 `wifi_credentials_encrypted`
- 当前加密方案为：
  - `scheme = dh-hmac-sha256-stream-v1`
  - `dh_group = modp14-sha256`
- 协议实现方式：
  - 每次连接由板端生成临时 DH 私钥 / 公钥与 `server_nonce`
  - App 根据 `hello.security` 生成 `client_public_key`
  - 双方派生会话密钥
  - Wi-Fi 凭据改为：
    - `ciphertext`
    - `mac`
    形式发送

本轮代码落点：

- Android 端：
  - [ProvisioningSecurity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ProvisioningSecurity.java)
  - [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)
- 板端：
  - [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)

兼容性策略：

- 板端仍临时兼容旧的明文：
  - `wifi_credentials`
- App 若连接到未升级板端，也仍可保留 bring-up 兼容回退
- 升级后的 App + 升级后的板端，默认会走加密路径

本轮验证：

- `python3 -m py_compile` 通过：
  - `classic-bluetooth-wifi-provisiond`
- Android 构建通过：
  - `:app:assembleDebug`
- 已做本地跨语言对拍：
  - Java 端生成的 `wifi_credentials_encrypted`
  - 可被板端 Python 实现正确解回原始 `ssid/password`

当前阶段结论：

- 经典蓝牙应用层已经不再要求以明文传输 Wi-Fi 密码
- 板端非明文持久化仍是下一阶段工作
- 后续在修改固件 / rootfs 时，应顺手把：
  - `lumelo-wifi-apply`
  - `wpa_supplicant` 配置落盘
  - 运行态凭据暴露面
  一并收口

### 7.1.26 2026-04-12：把新版安全握手现场部署到真机 T4 与 `PJZ110`，确认 `hello.security` 已真正跑起来

本轮现场部署：

- Android 真机：
  - `PJZ110`
  - 已重新 `adb install -r` 最新 debug APK
  - `am start -W -n com.lumelo.provisioning/.MainActivity` 成功
- T4 板子：
  - 当前板端可用 `root/root`
  - 已把仓库内新版
    [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
    覆盖部署到真机
  - 已在板端保留旧文件时间戳备份
  - `lumelo-wifi-provisiond.service` 重启成功

本轮现场验证结果：

- 板端新版 daemon 已实跑：
  - 远端脚本 SHA256 已变为：
    - `2ee7404830d9de3f220d33ea0a4ad01e21459c37f41d326527b205df9c7d1e50`
  - 板端脚本已包含：
    - `protocol = lumelo-json-v2`
    - `wifi_credentials_encrypted`
- 手机现场链路已跑到经典蓝牙连接成功：
  - `SCAN FOR LUMELO`
  - 选中 `lumelo (C0:84:7D:1F:37:C7)`
  - `CONNECT`
  - 收到 `device_info`
- App debug log 已明确记录：
  - `Classic Bluetooth credential security negotiated: dh-hmac-sha256-stream-v1`
  - `Classic Bluetooth hello: {... "protocol":"lumelo-json-v2", "security": {...}}`

本轮为何没有继续按下 `SEND WI-FI CREDENTIALS`：

- 当前这台 T4 已处于真实联网状态：
  - `wired_ip = 192.168.1.120`
  - `wifi_ip = 192.168.43.170`
- 在不知道当前目标 Wi-Fi 正确密码的情况下，继续下发一组新的凭据会改写板端现网配置
- 为避免把现场已联网板子从当前网络踢掉，本轮把真机验证边界停在：
  - “已连接并确认安全握手生效”

因此这轮可以下的结论是：

- 新版 App 与新版板端在真机上已经不是“只在本地代码里支持加密”
- 而是已经现场看到：
  - 板端真实下发 `hello.security`
  - 手机真实完成安全协商
- 若要继续做“加密凭据实际下发”真机验证，下一步应：
  - 使用一组明确可用的测试 Wi-Fi 凭据
  - 或在可接受改写现网配置的测试窗口内执行

### 7.1.27 2026-04-12：用 `isee_test / iseeisee` 做新版安全传输真机下发，确认 `connected`

本轮现场前提：

- 用户确认若继续做测试，可直接使用：
  - `SSID = isee_test`
  - `password = iseeisee`
- 当前板子已运行新版
  [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
- 当前手机已运行最新版 debug APK

本轮现场操作：

- 在 `PJZ110` 上：
  - 保持经典蓝牙连接
  - 在 App 输入：
    - `isee_test`
    - `iseeisee`
  - 点击 `SEND WI-FI CREDENTIALS`

本轮现场结果：

- Mac 侧轮询 `http://192.168.1.120:18080/provisioning-status` 看到：
  - 先进入：
    - `state = applying`
    - `message = applying credentials for isee_test`
  - 随后切到：
    - `state = connected`
    - `message = wifi connected on wlan0`
    - `ssid = isee_test`
    - `ip = 192.168.43.170`
    - `web_url = http://192.168.43.170:18080/`
- App 主界面结果区同步显示：
  - `State: connected`
  - `SSID: isee_test`
  - `Apply output: Wi-Fi credentials written for SSID: isee_test on interface: wlan0`
- App 随后自动打开内置 WebUI：
  - `Viewing http://192.168.43.170:18080/`

结合上一条 `7.1.26` 的现场证据：

- 发送前已经确认：
  - `Classic Bluetooth credential security negotiated: dh-hmac-sha256-stream-v1`
  - `hello` 中真实带有 `security`
- 发送时 App 源码路径会在已协商安全会话时优先发送：
  - `wifi_credentials_encrypted`
  - 见
    [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)
- 因此这轮可以把现场结论推进到：
  - 新版安全传输不只握手成功
  - 而且已经在真机上完成一次真实 Wi-Fi 凭据下发并得到 `connected`

当前仍保留的剩余安全工作：

- 板端系统仍沿用明文凭据落盘链路
- 下一阶段主任务仍是：
  - 把 `lumelo-wifi-apply` / `wpa_supplicant` 的凭据持久化改为非明文方案

### 7.1.28 2026-04-12：移除经典蓝牙明文回退，并把在线更新 / 整包重刷的双路径方案写入手册

本轮代码改动：

- Android 端：
  - [ClassicBluetoothTransport.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java)
  - 已移除“未协商到安全会话时回退发送明文 `wifi_credentials`”的逻辑
  - 当前若板端未提供 `hello.security`，App 会直接报错并要求先升级板端
- 板端：
  - [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
  - 已不再接受明文 `wifi_credentials`
  - 当前若旧客户端发送明文命令，板端返回：
    - `code = plaintext_credentials_disabled`

本轮工程侧新增：

- 新增在线部署脚本：
  - [deploy-t4-runtime-update.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/deploy-t4-runtime-update.sh)
- 当前脚本可用于：
  - 直接部署 `base/rootfs/overlay/` 下文件到线上 T4
  - 或把已编译好的本地二进制映射到远端指定路径
  - 并按需自动重启对应 systemd unit

本轮手册更新：

- 长期升级原则写入：
  - [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
  - 版本维护长期保留两种路径：
    - 在线更新
    - 整包重刷
- 开发阶段在线验证回路写入：
  - [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 已明确：
    - 哪些改动优先在线更新
    - 哪些改动仍必须整包重刷
    - 当前脚本的标准用法

本轮验证：

- `python3 -m py_compile` 通过：
  - `classic-bluetooth-wifi-provisiond`
- `sh -n` 通过：
  - `deploy-t4-runtime-update.sh`
- Android `assembleDebug` 通过
- 已使用新脚本把新版板端 daemon 在线部署到：
  - `192.168.1.120`
- 线上板子当前脚本 SHA256 已更新为：
  - `010b88295edea0c8e76de78510f8794878bfdca58f8dccc973a9642dcea0cc48`
- 之后又把新版 APK 安装到 `PJZ110`，并重新做了一轮真机下发：
  - `SCAN`
  - `CONNECT`
  - `SEND WI-FI CREDENTIALS`
  - `isee_test / iseeisee`
- 现场结果再次切到：
  - `state = connected`
  - `ssid = isee_test`
  - `ip = 192.168.43.170`
  - App 自动打开 `http://192.168.43.170:18080/`

这一轮结果的意义是：

- 现在不是“代码里去掉了明文回退”
- 而是：
  - 去掉回退后的新版 App
  - 配合新版板端
  - 已重新在真机上完成一次成功配网
- 这意味着当前经典蓝牙主链已不再依赖明文兜底

当前剩余安全工作已经收缩为：

- 板端系统不再以明文形式持久化 Wi-Fi 凭据
- `lumelo-wifi-apply` / `wpa_supplicant` 链路的运行态与落盘暴露面收口

### 7.1.29 2026-04-12：板端 Wi-Fi 持久化链路改为预派生 PSK，不再把明文密码写进 `wpa_supplicant`

本轮代码改动：

- 板端 daemon：
  - [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
  - 收到已解密的 `ssid / password` 后，不再把明文 `password` 存进 `self.credentials`
  - 当前改为先在 daemon 内按 WPA-PSK 标准参数派生：
    - `PBKDF2-HMAC-SHA1`
    - `4096` iterations
    - `32` bytes
  - 之后只保留：
    - `ssid`
    - `wpa_psk_hex`
    - `wifi_interface`
- 板端应用脚本：
  - [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)
  - 新增：
    - `--psk-hex <64-hex-psk> <ssid>`
  - 对 `wpa_supplicant` 后端：
    - 直接写入 `psk=<64hex>`
    - 不再调用会生成明文注释的 `wpa_passphrase`
    - 不再留下 `#psk=` 明文注释
  - 对 `NetworkManager` 后端：
    - 当前若传入 `--psk-hex`，明确报错退出
    - 先避免误以为已经具备同等级安全语义

本轮验证：

- 继续通过：
  - `python3 -m py_compile`
  - `classic-bluetooth-wifi-provisiond`
- 继续通过：
  - `sh -n`
  - `lumelo-wifi-apply`
- 已把新版 daemon 与新版 `lumelo-wifi-apply` 在线部署到：
  - `192.168.1.120`
- 使用同一组测试凭据：
  - `SSID = isee_test`
  - `password = iseeisee`
  - 先按 WPA-PSK 规则派生得到：
    - `30f042489d8e3a7beb0ff872b1802407f49bd4f3d2f36005d8b591aef0c84ba8`
  - 再在板子上直接执行：
    - `lumelo-wifi-apply --psk-hex <derived_psk> isee_test`
- 板子现场结果：
  - `/etc/wpa_supplicant/wpa_supplicant-wlan0.conf` 中当前只保留：
    - `ssid="isee_test"`
    - `psk=30f042489d8e3a7beb0ff872b1802407f49bd4f3d2f36005d8b591aef0c84ba8`
  - `grep 'iseeisee\\|#psk'` 为空
  - 文件权限为：
    - `600 root root`
  - `wpa_cli -i wlan0 status` 显示：
    - `wpa_state = COMPLETED`
    - `ssid = isee_test`
    - `ip_address = 192.168.43.170`

这轮验证要特别记住：

- 这次为了直查“落盘是否仍含明文”，是直接从板子上调用 `lumelo-wifi-apply`
- 因此：
  - `wlan0` 可以已经重新连上
  - 但 `/run/lumelo/provisioning-status.json` 仍可能保持 `advertising`
- 这是因为这次没有走 daemon 的 `handle_apply()` 状态机，不是实际联网失败

这一轮结果的意义是：

- 经典蓝牙传输侧：
  - 已加密
  - 已移除明文回退
- 板端持久化侧：
  - 当前 `wpa_supplicant` 路径也已不再写入明文 Wi-Fi 密码
- 当前真正还剩下的安全尾巴收缩为：
  - 明文密码在解密后到派生前仍会短暂存在于进程内存
  - 若以后启用 `NetworkManager` 作为主后端，需要为它补一条等价的非明文凭据写入路径

### 7.1.30 2026-04-12：为 PJZ110 的经典蓝牙扫描不稳定补上“remembered device”兜底

现场问题：

- 在 `PJZ110` 上冷启动新版 APK 后，再点：
  - `SCAN FOR LUMELO`
- App 页面会出现：
  - `Scan finished. No Lumelo device found.`
  - `devices = 0`
- 但手机蓝牙系统栈 `dumpsys bluetooth_manager` 同时显示：
  - `Classic inquiry` 实际有结果
  - 一次 inquiry 甚至出现：
    - `results: 2`
- 这说明这台手机上“经典扫描广播结果不稳定 / 名字不稳定”仍然存在
- 问题已经不再是：
  - 板子没广播
  - 或 App 完全不会连

本轮代码改动：

- Android 端：
  - [MainActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java)
  - 新增持久化字段：
    - `last_classic_address`
    - `last_classic_name`
  - 在经典蓝牙真正连接成功后：
    - 记住“上次成功连接的 Lumelo 设备”
  - 之后每次冷启动经典扫描：
    - 先把 remembered device 预填回候选列表
    - 即使系统这轮没有把名字稳定广播回来，也仍然能看到并点击连接
  - 列表展示新增：
    - `[LAST] [NAME] Lumelo T4 (<MAC>)`

本轮验证：

- Android `assembleDebug` 再次通过
- 把新版 APK 安装到 `PJZ110`
- 通过 App 偏好预灌当前测试板地址：
  - `C0:84:7D:1F:37:C7`
  - `Lumelo T4`
- 冷启动 App 后重新点击：
  - `SCAN FOR LUMELO`
- 现场结果：
  - `Scan finished.`
  - `Scan summary: Classic Bluetooth scan | devices=1 | nameMatch=1 | paired=0`
  - 列表出现：
    - `[LAST] [NAME] Lumelo T4 (C0:84:7D:1F:37:C7)`
- 然后直接从 remembered 候选继续：
  - 选中设备
  - `CONNECT`
- 现场结果：
  - App 进入：
    - `session = classic_connected`
  - 已收到 `device_info`
  - `Selected` 正确显示 remembered 设备地址

这一轮结果的意义是：

- 对这台已经成功配通过一次的测试手机来说
- 后续即使系统经典扫描广播偶发抽风
- App 也不再卡死在：
  - `devices = 0`
- 当前用户体验已经从“必须依赖一次完美扫描”提升为：
  - “记住上次成功设备，优先保障可重连”

当前剩余边界：

- 这次补的是“已成功连接过的手机”的稳态兜底
- 对一台从未连过、且系统又不给稳定 `ACTION_FOUND / ACTION_NAME_CHANGED` 的新手机
- 经典蓝牙首配扫描仍然可能需要继续兼容性优化

### 7.1.31 2026-04-12：把首次经典蓝牙扫描改成“命中优先 + 候选兜底 + 单命中自动选中”

基于 `7.1.30` 的现场继续推进：

- remembered device 兜底已经能解决：
  - “这台手机连通过一次后，冷启动又扫不到”
- 但首次扫描仍暴露出两个体验问题：
  - App 会把所有 classic 结果都按发现顺序平铺，真正的 `lumelo` 可能被淹没在候选里
  - 即使整轮只有一个明确 `nameMatch`，也还要用户手工再点一次选择

本轮代码改动：

- Android 端：
  - [MainActivity.java](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java)
  - 经典扫描结果现在支持：
    - `[NAME]` / `[LAST]` / `[PAIRED]` 优先排序
    - `[CLASSIC]` 作为未命名 classic 候选兜底
  - 若整轮扫描中只有一个 `nameMatch`
    - 自动选中该设备
    - 自动把 `CONNECT` 置为可点
  - 因此首次扫描体验从：
    - “可能 0 设备 / 需要人工判断”
    - 推进到：
    - “先命中真正的 Lumelo，再保留 classic 候选兜底”

本轮验证：

- 先手工清空 App 偏好中的：
  - `last_classic_address`
  - `last_classic_name`
- 也就是按“新手机首次扫描”路径重测
- 冷启动新版 APK 后重新点击：
  - `SCAN FOR LUMELO`
- 现场结果：
  - `Scan summary: Classic Bluetooth scan | devices=13 | nameMatch=1 | selected=C0:84:7D:1F:37:C7`
  - 列表顶部直接出现：
    - `[NAME] lumelo (C0:84:7D:1F:37:C7)`
  - 下方才是：
    - `[CLASSIC] ...`
  - `Selected` 已自动变成：
    - `lumelo (C0:84:7D:1F:37:C7)`
  - `CONNECT` 已直接可点
- 随后继续点：
  - `CONNECT`
- 现场结果：
  - `session = classic_connected`
  - `device_info` 正常返回
  - App 偏好里也重新写回：
    - `last_classic_address = C0:84:7D:1F:37:C7`
    - `last_classic_name = lumelo`

这一轮结果的意义是：

- 对 `PJZ110` 这台手机
- 当前已经不只是“连通过一次后可重连”
- 而是：
  - 在清空 remembered 的情况下
  - 冷启动首次扫描
  - 也能把真实 `lumelo` 顶到最前面并自动选中
  - 然后继续成功连接

当前剩余体验边界：

- 若未来遇到另一类手机：
  - 整轮 inquiry 完全不给 `nameMatch`
  - 只给一堆匿名 classic 结果
- 当前 App 已不会再卡成 `0 设备`
- 但用户仍可能需要在 `[CLASSIC]` 候选中手工判断一次

### 7.2 2026-04-06：开发环境、服务闭环与基础数据层收口

本轮阶段结论：

- 明确采用 `OrbStack + NanoPC-T4 真机` 双层开发策略
- `M0` 完成
- `M1` 完成
- `M2` 完成第一版
- 当前主线切入 `M3`

本轮已落地：

- 仓库骨架、Rust workspace、Go `controld`、基础 `systemd` unit
- `playbackd + sessiond + controld` Linux 端到端联调
- `PLAY -> quiet_active -> STOP` 的基本控制链
- `queue.json` 的写入、重启恢复与“恢复后统一进入 stopped”
- `history.json` 第一版
- 队列操作第一版：
  - `QUEUE_APPEND`
  - `QUEUE_INSERT_NEXT`
  - `QUEUE_REMOVE`
  - `QUEUE_CLEAR`
  - `QUEUE_REPLACE`
- `QUEUE_SNAPSHOT` 与最小队列页
- `library.db / media-indexd` 最小结构
- `libraryclient` 与最小库页
- `media-indexd` 目录扫描、标签解析、封面发现、`thumb/320` 生成第一版
- 项目名称统一为 `Lumelo`
- `T4_moode_port_blueprint.md` 归档为历史参考

当时形成的阶段判断：

- 产品和架构边界已基本收口
- 仓库已不再是“纯文档阶段”
- 下一步从“继续讨论”转为“按当前主线补服务逻辑，并为真机 bring-up 做准备”

### 7.3 2026-04-07：首张自定义 rootfs 图与 bring-up 入口建立

本轮已落地：

- 首张自定义 `Lumelo-defined rootfs` 图产出：
  - [lumelo-t4-rootfs-20260407.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260407.img)
- 补齐 T4 bring-up 调试入口
- 补齐自定义 rootfs 离线验收脚本
- 补齐 ALSA 手动 smoke helper

阶段结论：

- 制镜链已经打通
- 可以开始进入真机启动链与 bring-up 验证
- 但 `20260407` 图后续被证明存在启动链问题，不再作为当前真机验证主图

### 7.4 2026-04-08：bootfix 图、日志页、配网底座与 Android App 骨架

本轮关键变化：

- 识别出 `20260407` 图的启动链问题
- 产出修复图：
  - [lumelo-t4-rootfs-20260408-bootfix.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260408-bootfix.img)
- 真机反馈已确认：
  - 系统能够进入 `lumelo login:`
  - bootfix 图已经越过早期启动链失败

本轮新增：

- `controld` Web 日志页：
  - `GET /logs`
  - `GET /logs.txt`
- 下一张图所需的 bring-up 修正：
  - `CONTROLD_LISTEN_ADDR=0.0.0.0:18080`
  - networkd 可发现性补强
  - 下一张调试图默认带 `root/root`
- T4 端蓝牙 / Wi-Fi 配网底座第一版
- 最简 Android 配网 App 工程骨架：
  - [apps/android-provisioning](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning)

本轮策略调整：

- 不马上进入真机音频 bring-up
- 先把日志页、网络可发现性、蓝牙 / Wi-Fi 配网底座和最简 Android App 链路补齐
- 再重建下一张 T4 图
- 下一张图上板后优先查网络和日志，不直接跳到音频链

当前遗留：

- 用户正在验证的 `20260408-bootfix` 图不包含后续新增的日志页、端口固定、配网底座和网络修复
- Android SDK 尚未初始化，因此 APK 还没真正构建

## 8. 当前交接入口

当前继续推进时，优先阅读：

- [README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md)
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
- [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
- [apps/android-provisioning/README.md](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/README.md)
- [packaging/image/README.md](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/README.md)

当前没有单独维护一份通用 `TODO.md`。

当前待办入口以这里为准：

- “当前未闭环事项”
- “当前推荐下一步”

历史提案、阶段性 checklist 和旧 MVP 文档统一归档在：

- [README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md)
- `/Volumes/SeeDisk/Codex/Lumelo/docs/archive/`

### 8.1 2026-04-12：docs 顶层收口、协议文档正名与历史稿归档

本轮已落地：

- 删除 `docs/` 与 `docs/archive/` 下所有 `._*` AppleDouble 垃圾文件
- 新增 [README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/README.md) 作为统一文档索引
- 原文件 `Bluetooth_WiFi_Provisioning_MVP.md` 已正名为 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
- 顶层 `docs/` 只保留当前仍在使用的主文档
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 与 [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md) 顶部已补齐文档边界说明
- 下列历史文档已移入 `docs/archive/`：
  - [Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md)
  - [V1_Local_Mode_Function_and_Service_Spec.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/V1_Local_Mode_Function_and_Service_Spec.md)
  - [V1_Technical_Architecture_Proposal.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/V1_Technical_Architecture_Proposal.md)
  - [Real_Device_Findings_20260412_v15.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Real_Device_Findings_20260412_v15.md)
  - [Repo_Rename_To_Lumelo_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Repo_Rename_To_Lumelo_Checklist.md)
- 活跃文档的相互引用与文档边界说明已同步修正

阶段结论：

- 新窗口进入仓库后，不再需要先判断“哪份文档才是现行版本”
- `docs/README.md` 负责索引和阅读路径
- 活跃主文档负责当前规则，`docs/archive/` 只保留历史背景

### 8.2 2026-04-12：产品手册后半段减重，专项细节继续下沉

本轮已落地：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 的 `16. 当前推荐开发环境` 已压缩为原则摘要
- 产品手册不再重复展开：
  - 工作区与文件系统方案
  - SDK / 工具链 / 出包与在线更新细节
  - T4 bring-up 的现场操作步骤
- `21. T4 Bring-up 稳定约束` 已收成长期非协商原则
- 更细的操作性内容继续分别放在：
  - [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
  - [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)
  - [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)

阶段结论：

- 产品手册现在更像“长期规则总册”，而不是“所有操作说明的汇总页”
- 新窗口查流程时，应优先走 `docs/README.md` 分流到专项文档

### 8.3 2026-04-12：交接文件的“已验证事实 / 未闭环事项”拆分完成

本轮已落地：

- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md) 的第 `9` 节已重写为：
  - `已验证事实`
  - `板子侧仍未闭环`
  - `手机 APK 仍未闭环`
  - `安全尾项`
  - `业务功能仍未闭环`
- 已完成的扫描修复、加密传输、明文回退移除和 `wpa_psk_hex` 落盘验证，不再混写在“待办”里
- 当前真正还没闭环的板子、APK、安全和业务项已经单独列出

阶段结论：

- 新窗口读交接文件时，不会再把“已做完的安全/扫描修复”误判成待办
- `AI_Handoff_Memory.md` 现在更适合作为真正的当班交接入口
