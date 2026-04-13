# AI Review Part 10

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `docs/Development_Progress_Log.md` (1/4)

- bytes: 119711
- segment: 1/4

~~~md
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
~~~

