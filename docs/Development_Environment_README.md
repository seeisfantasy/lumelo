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

### 4.6 当前测试镜像默认 SSH 登录信息

这条作为当前 `T4` 开发 / 测试镜像的环境约定维护在这里，后续不要再靠交接记忆：

- 用户名：`root`
- 密码：`root`
- 适用范围：当前开发 / bring-up / 测试镜像
- 不适用范围：正式发布镜像；正式口径仍以 [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 为准

最小登录示例：

```sh
ssh root@<T4_IP>
# password: root
```

## 5. Lumelo 当前文件系统与工作区方案

### 5.1 当前状态

- `SeeDisk`：`APFS`
- 当前默认 macOS 主工作区：
  - `/Volumes/SeeDisk/Codex/Lumelo`

当前结论：

- 外置盘本体既然已经是 `APFS`
  - 就不再需要为了规避 `exFAT` 风险而额外套一层 `APFS sparsebundle`
- 旧的 `LumeloDev` 工作区
  - 现在只保留为历史 workaround / 兼容入口
  - 不再作为当前默认开发入口

### 5.2 为什么现在可以直接用 `SeeDisk`

这一步不是为了 Android 模拟器，也不是为了替代 Android 真机。

之前额外建立 `LumeloDev` 的根因是：

- macOS 主机在 `exFAT` 上工作时的 `._*` 污染
- Android 构建链对侧写文件敏感
- 未来其他元数据敏感工具链也可能踩同类坑

现在 `SeeDisk` 本体已经改成 `APFS` 后：

- 这些 `exFAT` 特有风险不再是当前默认环境问题
- 当前仓库可以直接留在：
  - `/Volumes/SeeDisk/Codex/Lumelo`
- 不再需要先同步到另一个 `APFS sparsebundle` 再开始开发

### 5.3 与 Linux 开发链的关系

- `lumelo-dev` 可以直接访问：
  - `/Volumes/SeeDisk/Codex/Lumelo`
- 所以当前这套 `APFS` 外置盘工作区，同时兼容：
  - macOS 编辑 / Android Studio
  - `OrbStack / lumelo-dev`
- 需要继续注意的是：
  - Linux 看到的是宿主共享目录
  - 不是“Linux 原生挂载 APFS”
  - 所以重负载临时目录仍应放：
    - `/var/tmp/lumelo-<tag>/`

## 6. 当前项目下的辅助脚本

- [mount-lumelodev-apfs.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/mount-lumelodev-apfs.sh)
  - 旧 `LumeloDev` 兼容脚本
  - 仅在需要回看或复用历史 `APFS sparsebundle` 工作区时再用
- [sync-to-lumelodev-apfs.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/sync-to-lumelodev-apfs.sh)
  - 旧 `LumeloDev` 同步脚本
  - 当前默认流程已不再依赖它
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
  - `/Volumes/SeeDisk/Codex/Lumelo`
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

### 7.1.1 出包触发规则

默认开发节奏：

- 优先在线修：
  - runtime update
  - live T4 验证
  - APK reinstall
  - WebUI / daemon / helper 小步验证
- 不把 `image/img` 当作日常调试手段。
- 只有用户明确说：
  - `出包`
  - `出 image`
  - `出 img`
  - `打镜像`
  才生成新的 T4 rootfs `image/img`。

如果某个问题确实必须通过新镜像解决，先说明：

- 为什么在线修不够
- 不出包会留下什么风险
- 建议用户下达出包命令

在用户确认前，不主动生成新的 `vN image`。

### 7.2 标准出包步骤

1. 在 macOS 主机确认当前活跃工作区就是：
   - `/Volumes/SeeDisk/Codex/Lumelo`
2. 在 macOS 主机确定本轮输出文件名：
   - 例如 `out/t4-rootfs/lumelo-t4-rootfs-20260412-v12.img`
3. 在 `OrbStack / lumelo-dev` 中以 `root` 运行制镜脚本，但显式保留调用用户上下文：
   - `SUDO_USER=see`
4. 将所有重负载目录指到 Linux 原生目录，例如：
   - `TMPDIR=/var/tmp/lumelo-20260412-v12/tmp`
   - `LUMELO_BUILD_ROOT=/var/tmp/lumelo-20260412-v12/build-root`
   - `CARGO_TARGET_DIR=/var/tmp/lumelo-20260412-v12/cargo-target`
   - `GOCACHE=/var/tmp/lumelo-20260412-v12/go-cache`
5. 在 `OrbStack` 内从当前 APFS 工作区启动正式制镜：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/SeeDisk/Codex/Lumelo
  export SUDO_USER=see
  export TMPDIR=/var/tmp/lumelo-YYYYMMDD-vN/tmp
  export LUMELO_BUILD_ROOT=/var/tmp/lumelo-YYYYMMDD-vN/build-root
  export CARGO_TARGET_DIR=/var/tmp/lumelo-YYYYMMDD-vN/cargo-target
  export GOCACHE=/var/tmp/lumelo-YYYYMMDD-vN/go-cache
  ./scripts/build-t4-lumelo-rootfs-image.sh \
    --board-base-image /absolute/path/to/rk3399-base.img \
    --output /Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img
'
```

6. 出图后立即做离线验收：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/SeeDisk/Codex/Lumelo
  ./scripts/verify-t4-lumelo-rootfs-image.sh \
    /Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img
'
```

7. 若后续把制品移动到其他目录或上传前改了最终落点，再重新计算一次最终路径下的 `sha256`。
8. 若本轮改动涉及 `NanoPC-T4` 无线链路，还要再跑一次官方金样对比：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/SeeDisk/Codex/Lumelo
  ./scripts/compare-t4-wireless-golden.sh \
    --board-base-image /absolute/path/to/rk3399-base.img.gz \
    --image /Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img
'
```

通过标准：

- `0 failure(s), 0 warning(s)`
- `BCM4356A2.hcd`
- `hciattach.rk`
- `bcmdhd.conf`
  与官方底图一致
9. 若本轮改动涉及：
   - 蓝牙
   - Wi-Fi
   - SSH
   - firmware / patch
   - `systemd` bring-up
   - 板级启动链
   则烧录后必须继续执行：
   - [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
10. 在 `T4_Bringup_Checklist.md` 未通过前，不对外宣布“这张图已可烧录验证蓝牙 / SSH / 配网”。

### 7.2.1 T4 无线链路固定金样

针对 `NanoPC-T4` 板载 `AP6356S` 组合模组，后续出包默认以 FriendlyELEC 官方运行态为金样，不再把通用 Debian / Broadcom 路线当成等价替代。

当前已确认的官方金样要点：

- Wi-Fi 驱动走 `bcmdhd`
- 蓝牙走 `hci_uart + btbcm`
- 板载无线为 `AP6356S`
- 蓝牙控制器挂在 `ttyS0`
- 官方 Wi-Fi firmware 路径是：
  - `/system/etc/firmware/fw_bcm4356a2_ag.bin`
  - `/system/etc/firmware/nvram_ap6356.txt`
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

官方金样用户态网络栈与 `Lumelo` 当前实现仍不同：

- 官方金样观察到 `NetworkManager`、`ifupdown`、`dhcpcd-base`、`wireless-tools`、`wpa_supplicant`。
- 官方金样 `NetworkManager.service` 默认 enabled，`systemd-networkd.service` 默认 disabled。
- 官方金样 `NetworkManager` 配置要点：
  - `/etc/NetworkManager/NetworkManager.conf` 使用 `plugins=ifupdown,keyfile` 且 `[ifupdown] managed=true`
  - `12-managed-wifi.conf` 使用 `unmanaged-devices=wl*,except:type:wifi`
  - `99-unmanaged-wlan1.conf` 使用 `unmanaged-devices=interface-name:wlan1`
  - `disable-random-mac-during-wifi-scan.conf` 使用 `wifi.scan-rand-mac-address=no`
  - `/etc/network/interfaces` 只 source `/etc/network/interfaces.d/*`
- `Lumelo` 当前仍以 `systemd-networkd + wpa_supplicant` 路径为主，`lumelo-wifi-apply` 支持 `NetworkManager` / `wpa_supplicant` backend auto。

判断规则：

- 板级 firmware / driver 必须继承官方 vendor 路线。
- 用户态网络策略可以保持 `Lumelo` 自己的路线。
- 后续若 Wi-Fi 连接、DHCP、重连或扫描仍不稳定，不能只归因 firmware；必须同时检查是否需要把 `NetworkManager` 设为 T4 默认 backend。

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
  - 判断：源码目录落在了非原生文件系统
  - 这条在 `Lumelo` 里历史上对应过：
    - `SeeDisk/exFAT`
  - 固定处理：把活跃源码目录迁回 `APFS`
  - 当前默认已是：
    - `/Volumes/SeeDisk/Codex/Lumelo`
  - 一般不再需要先同步到 `LumeloDev`

- 现象：出图成功后再次执行同步，`LumeloDev/out/` 中的镜像或 APK 消失
  - 判断：这是旧 `LumeloDev` 同步流的历史坑
  - 固定处理：当前默认流程不再依赖这套同步链
  - 只有回到旧 `LumeloDev` workaround 时，才需要额外注意它

- 现象：`sha256` 文件内容还指向 `/Volumes/LumeloDev/...`
  - 判断：这是旧路径时代留下的历史痕迹，或制品移动后没重算校验
  - 固定处理：在最终交付路径重新执行 `shasum -a 256 <final-img> > <final-img>.sha256`

### 7.4 出包前后检查清单

- 出包前确认：
  - 当前 `vN` 序号
  - 当前活跃工作区就是：
    - `/Volumes/SeeDisk/Codex/Lumelo`
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

## 9. 文档边界

后续默认按下面分工维护：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
  - 产品原则和长期边界
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 软件环境、开发环境、宿主机文件系统与搭建约定
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
  - 每一步实际开发进度与环境调整记录

外部 AI 静态审查文档不再作为长期 docs 维护。需要时运行：

```sh
python3 scripts/build-ai-review-docs.py
```

脚本会重新生成 `docs/review/`，该目录是 generated bundle，不提交、不作为权威文档。
