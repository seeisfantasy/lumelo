# T4 USB-to-eMMC Firmware Requirements

## 1. 结论

`USB 线刷进 eMMC` 是 Lumelo 后续正式交付目标之一，但它不是当前 `v24` checkpoint rootfs image 的同义词。

当前 `v24` 产物是可烧录 / 可验证的 T4 rootfs raw image：

- `out/t4-rootfs/lumelo-t4-rootfs-20260502-v24.img`

后续正式发布还需要新增一类独立产物：

- `Lumelo USB-to-eMMC firmware package`

目标是用户或维护人员可以通过 NanoPC-T4 的 Type-C / Rockusb / MaskROM 路径，把 Lumelo 整包刷入板载 eMMC。

2026-05-05 重新校准后的结论：

- Win11 首版主线应是 `RKDevTool` 的 `Download Image` 多分区 USB package。
- 这个 package 应对齐 FriendlyELEC 官方 `usb` 固件形态，而不是把完整 raw disk `.img` 直接放进 `System` 行刷写。
- `Upgrade Firmware` 只适合 Rockchip packed firmware / single-image firmware，不适合当前 Lumelo raw disk image。
- 早前的 `raw image + RKDevTool Download Image address 0x0` 方案不再作为 Win11 主线，只保留为后续 Linux `rkdeveloptool wl 0` 工程验证候选。

## 2. 官方资料确认的基础事实

FriendlyELEC 官方把 NanoPC-T4 image 明确分成三类：

- `sd`：从 TF / microSD 启动整个系统
- `eflasher`：从 TF 启动 eFlasher，再把系统写入 eMMC
- `usb`：通过 USB 把系统写入 eMMC

NanoPC-T4 官方 USB 刷机路径：

1. 下载 `01_Official images/03_USB upgrade images` 下的 USB upgrade image。
2. Windows 安装 Rockchip USB driver。
3. 按住 `MASK` 键，通过 USB 数据线连接 NanoPC-T4 和 PC。
4. 状态灯亮起至少 3 秒后松开 `MASK`。
5. `RKDevTool` 显示 `Found One MASKROM Device`。
6. 必要时加载 `MiniLoaderAll.bin` 并 `EraseAll`。
7. 按固件形态选择：
   - FriendlyELEC 风格多分区 package：`Download Image`
   - Rockchip packed single firmware：`Upgrade Firmware`

FriendlyELEC 还明确说明：

- eMMC 内已有不同系统，或刷完无法启动时，才需要先擦除 eMMC。
- Mac 上旧版 `upgrade_tool` 官方测试不正常，建议优先 Windows 或 Linux。
- NanoPC-T4 不能直接从 M.2 / USB 做第一阶段启动；系统可以放到 M.2 / USB，但 boot 仍需要 eMMC 或 TF。

## 3. 官方 USB package 拆解结论

已验证成功刷写的官方包：

- `rk3399-usb-debian-trixie-core-4.19-arm64-20260319.zip`
- zip sha256：
  - `46a4352c4935053d63a2620d69135c580d1aa89dd6bbdcd6b414a3948031cf2d`

解压后是 FriendlyELEC 风格多分区 package：

- `RKDevTool.exe`
- `bin/AFPTool.exe`
- `bin/RKImageMaker.exe`
- `Language/Chinese.ini`
- `Language/English.ini`
- `doc/RKDevTool_manual.pdf`
- `MiniLoaderAll.bin`
- `config.cfg`
- `config.ini`
- `parameter.txt`
- `info.conf`
- `uboot.img`
- `trust.img`
- `misc.img`
- `dtbo.img`
- `resource.img`
- `kernel.img`
- `boot.img`
- `rootfs.img`
- `userdata.img`

其中：

- `rootfs.img` 是 Android sparse image。
  - sparse block size：4096
  - expanded size：2 GiB
- `userdata.img` 是 Android sparse image。
  - sparse block size：4096
  - expanded size：200 MiB
- `MiniLoaderAll.bin` sha256：
  - `c2a830841eb8c5b0e124816d1201e9c10778751a9912e57a906000477e89d096`

官方 `parameter.txt` 的分区布局：

```text
0x00002000@0x00004000(uboot)
0x00002000@0x00006000(trust)
0x00002000@0x00008000(misc)
0x00002000@0x0000a000(dtbo)
0x00008000@0x0000c000(resource)
0x00014000@0x00014000(kernel)
0x00018000@0x00028000(boot)
0x00400000@0x00040000(rootfs)
-@0x00440000(userdata:grow)
```

RKDevTool 对应表格：

| Row | Name | Address | File |
| --- | --- | --- | --- |
| 1 | Loader | `0x00000000` | `MiniLoaderAll.bin` |
| 2 | Parameter | `0x00000000` | `parameter.txt` |
| 3 | Uboot | `0x00004000` | `uboot.img` |
| 4 | Trust | `0x00006000` | `trust.img` |
| 5 | Misc | `0x00008000` | `misc.img` |
| 6 | Dtbo | `0x0000A000` | `dtbo.img` |
| 7 | Resource | `0x0000C000` | `resource.img` |
| 8 | Kernel | `0x00014000` | `kernel.img` |
| 9 | Boot | `0x00028000` | `boot.img` |
| 10 | Rootfs | `0x00040000` | `rootfs.img` |
| 11 | Userdata | `0x00440000` | `userdata.img` |

## 4. Lumelo 首版 USB package 需求

Lumelo 的第一版 USB-to-eMMC package 应生成官方同形态目录：

需要包含：

- Rockchip / FriendlyELEC 对应版本 `MiniLoaderAll.bin`
- `parameter.txt`
- boot 链分区镜像：
  - `uboot.img`
  - `trust.img`
  - `resource.img`
  - `kernel.img`
  - `boot.img`
- Lumelo rootfs：
  - `rootfs.img`
- data / persistent 分区初始化镜像：
  - `userdata.img` 或空分区声明
- RKDevTool 配置：
  - `config.cfg`
  - 或等价的 Download Image table
- Windows 工具说明：
  - `DriverAssitant`
  - `RKDevTool`
- Linux 工具说明：
  - `upgrade_tool`
  - `rkdeveloptool`
- checksums：
  - `SHA256SUMS`
- release manifest：
  - image version
  - source git commit
  - base FriendlyELEC image version
  - loader version
  - kernel / DTB / firmware source
  - rootfs build profile

Linux `upgrade_tool` 分区刷写形态参考：

```sh
upgrade_tool ul MiniLoaderAll.bin
upgrade_tool di -p parameter.txt
upgrade_tool di uboot uboot.img
upgrade_tool di trust trust.img
upgrade_tool di resource resource.img
upgrade_tool di kernel kernel.img
upgrade_tool di boot boot.img
upgrade_tool di rootfs rootfs.img
upgrade_tool RD
```

正式包优点：

- 更接近 FriendlyELEC 官方发布形态。
- Windows 用户可以用 `RKDevTool` 图形界面。
- 分区更新和整包更新边界更清楚。

代价：

- 现有 Lumelo build script 需要新增“导出分区镜像和 RKDevTool config”的能力。
- 必须严格维护 `parameter.txt` 与分区镜像 offset / size 的一致性。

首版建议：

- 复用官方 USB package 作为 reference template。
- 复用官方 `config.cfg` / `config.ini` / `RKDevTool.exe` / `bin/` / `Language/` / `doc/`。
- 输出目录仍放在 `out/` 下，不进 repo。
- `MiniLoaderAll.bin` 不进 repo；打包时从 reference package 或显式路径复制，并记录 hash。
- 不修改原来的 TF/raw `.img` 出包链。
- 不修改原始 `out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img`。

Lumelo source image 与官方 package 的差异：

- Lumelo `v24` raw image 的 p1-p8 起始 offset 与官方布局一致。
- Lumelo `v24` raw image 的 rootfs partition 是 1 GiB。
- 官方 USB package 的 rootfs 分区声明是 2 GiB，`userdata` 从 `0x00440000` 开始。
- 因此首版 package 应优先把 Lumelo rootfs partition image resize 到 2 GiB，再转为 Android sparse image。
- `userdata.img` 可生成一个 200 MiB sparse ext4 初始镜像，或复制并 resize 官方 userdata 模板。

生成步骤草案：

1. 校验 source raw image 通过 `verify-t4-lumelo-rootfs-image.sh`。
2. 校验 source raw image GPT 包含：
   - `uboot/trust/misc/dtbo/resource/kernel/boot/rootfs/userdata`
3. 用 `dd` 从 source raw image 提取：
   - `uboot.img`
   - `trust.img`
   - `misc.img`
   - `dtbo.img`
   - `resource.img`
   - `kernel.img`
   - `boot.img`
   - raw `rootfs` partition
4. 对 raw `rootfs` partition：
   - `e2fsck`
   - `resize2fs` 到 2 GiB
   - 转成 Android sparse `rootfs.img`
5. 生成或 resize `userdata.img` 为 200 MiB Android sparse image。
6. 从 reference package 复制：
   - `RKDevTool.exe`
   - `bin/`
   - `Language/`
   - `doc/`
   - `MiniLoaderAll.bin`
   - `config.cfg`
   - `config.ini`
   - `parameter.txt`
   - `info.conf`
7. 生成：
   - `manifest.json`
   - `SHA256SUMS.txt`
   - `README-WIN11-RKDEVTOOL.md`
8. 离线 verifier 检查：
   - 所有文件存在
   - hash 匹配
   - `parameter.txt` 与 RKDevTool table 匹配
   - `rootfs.img` / `userdata.img` sparse header 合法
   - source raw image hash 未变化

## 5. 可选量产产物：Rockchip update.img

这是更完整的 mass-production 形态。

需要：

- Rockchip firmware packing tools
- package-file
- parameter
- loader
- 所有 partition image
- `update.img`

用途：

- Windows `RKDevTool` 的 `Upgrade Firmware` 页面选择单个 firmware 文件刷写。
- 更像消费级固件包。

风险：

- packing tool 链更复杂。
- 需要确认 raw image、RK firmware、partition image 三种格式不要混用。
- 当前阶段不建议作为第一个 USB-to-eMMC MVP。

## 6. Host 工具需求

### Windows

需要：

- `DriverAssitant_v5.12.zip`
- `RKDevTool_v3.37_for_window.zip` 或 USB 固件包内置 RKDevTool
- Type-C 数据线
- NanoPC-T4 12V/2A DC 供电

验收信号：

- Device Manager 中出现 Rockusb device
- RKDevTool 显示：
  - `Found One MASKROM Device`
  - 或 `Found One LOADER Device`

### Linux

需要其一：

- Rockchip `upgrade_tool`
- open-source `rkdeveloptool`

还需要：

- `MiniLoaderAll.bin`
- udev rule 或 `sudo`
- `lsusb` 可见 `2207:330c` 类 Rockchip RK3399 device

`rkdeveloptool` 注意事项：

- 不会自动解压 `.gz`。
- 不能选择多台 MaskROM device，也不能软件选择写入哪块 storage。
- 一次只连接一块目标板和目标 storage 更安全。

### macOS

结论：

- 不作为 Lumelo V1 官方推荐线刷环境。
- 可以作为后续实验项尝试 `rkdeveloptool`，但不作为正式交付验收入口。

原因：

- NanoPC-T4 官方文档明确说旧 `upgrade_tool` 在 macOS 测试不正常，建议 Windows 或 Linux。

## 7. 板端进入刷机模式

推荐方式：

1. 拔掉 TF 卡和不必要 USB 外设。
2. 使用 DC 电源给 T4 供电。
3. 按住 `MASK` / `BOOT` 键。
4. 用 Type-C 数据线连接 T4 与电脑。
5. 状态灯亮起 3 到 4 秒后松开按键。
6. PC 端确认 `MASKROM` device。

软件方式：

```sh
reboot loader
```

适用范围：

- 只适合当前系统仍能启动时进入 `LOADER` mode。
- 不等价于空白 eMMC / 救砖时的 `MASKROM`。

不推荐方式：

- 手工短接 eMMC clock 到 GND。

原因：

- NanoPC-T4 已有 `MASK` / `BOOT` 键。
- 短接属于救砖级硬件操作，风险高。

## 8. 风险和保护要求

### 8.1 eMMC 数据破坏

`EraseAll` / `EF` 会清除 eMMC。

要求：

- 正式脚本必须二次确认。
- 默认不要自动 erase。
- 只有跨系统、刷写失败后无法启动、救砖时才建议 erase。

### 8.2 MAC / vendor storage

风险：

- 全盘 raw write 或 erase 可能导致 SN / MAC / vendor storage 类信息丢失或重复。

要求：

- 线刷前尽量备份：
  - `/proc/cmdline`
  - `ip link`
  - `lsblk`
  - vendor storage dump，若后续确认路径
- 首版 MVP 至少要记录 MAC 保护策略。

### 8.3 loader 不匹配

`MiniLoaderAll.bin` 必须匹配 RK3399 / NanoPC-T4 / DDR 配置。

风险：

- loader 不匹配时可能出现：
  - DRAM 初始化失败
  - `Download Boot Fail`
  - `libusb_bulk_transfer timeout`
  - 识别到 MaskROM 但无法读写 flash

要求：

- 固件包必须记录 loader 来源和 hash。
- 不允许混用其他 RK3399 板子的 loader。

### 8.4 image 格式混用

必须区分：

- raw disk image
- Rockchip `update.img`
- partition image package
- eFlasher SD-to-eMMC image

要求：

- README 中必须明确每种文件对应哪种工具页签和命令。
- 不允许把 raw image 当作 RK firmware 直接走错误入口。

## 9. Lumelo build system 需要新增 / 调整的脚本

已新增但需降级为实验项：

- `scripts/package-t4-usb-emmc-raw.sh`
  - 早期 raw full-disk package 脚本。
  - 不再作为 Win11 `RKDevTool` 主线路径。
  - 后续若保留，只能明确标记为 Linux `rkdeveloptool wl 0` 工程实验。
- `scripts/verify-t4-usb-emmc-raw-package.sh`
  - 只验证上述 raw package，不代表 Win11 USB/eMMC package 已可交付。

已新增主线脚本：

- `scripts/package-t4-usb-emmc-official-layout.sh`
  - 输入：当前 Lumelo raw image、官方 USB reference package、version。
  - 输出：FriendlyELEC 风格 `Download Image` 多分区 package。
  - 从 Lumelo raw image 提取 p1-p7。
  - 将 Lumelo rootfs resize 到 2 GiB 并转 Android sparse `rootfs.img`。
  - 生成 200 MiB Android sparse `userdata.img`。
  - 复制 reference package 中的 `RKDevTool.exe`、`bin/`、`Language/`、`doc/`、`config.cfg`、`config.ini`、`parameter.txt`、`MiniLoaderAll.bin`。
  - 生成 `manifest.json`、`SHA256SUMS.txt`、Win11 README。
- `scripts/verify-t4-usb-emmc-official-layout-package.sh`
  - 检查 package 完整性。
  - 检查 loader hash。
  - 检查 `parameter.txt` 地址。
  - 检查 RKDevTool table 对应文件存在。
  - 检查 `rootfs.img` / `userdata.img` sparse header。
  - 检查 source raw image hash 未变化。

当前已生成并通过离线 gate 的首个 package：

- `out/t4-usb-emmc-official-layout/lumelo-t4-usb-emmc-official-layout-20260502-v24/`
- 可交付 zip：
  - `out/t4-usb-emmc-official-layout/lumelo-t4-usb-emmc-official-layout-20260502-v24.zip`
  - `sha256 = db4a0f3e39f5c96478ebbf1ef6ab556d12d434281178ca48373fd7de9e618674`
- package size：约 `941 MiB`
- zip size：约 `267 MiB`
- `rootfs.img`：
  - Android sparse image
  - physical size：约 `772 MiB`
  - expanded size：`2 GiB`
- `userdata.img`：
  - Android sparse image
  - physical size：约 `36 MiB`
  - expanded size：`200 MiB`
- source image hash 未变化：
  - `945b529810fa39c95e3a707fe65fdac11710d6c8803045ed174db1fbc225229b`
- verifier：
  - `verify-t4-usb-emmc-official-layout-package.sh = 0 failure(s)`
- zip verifier：
  - `unzip -t = No errors detected`
  - `shasum -a 256 -c = OK`

典型打包命令：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/SeeDisk/Codex/Lumelo &&
  ./scripts/package-t4-usb-emmc-official-layout.sh \
    --source-image out/t4-rootfs/lumelo-t4-rootfs-20260502-v24.img \
    --reference-usb-dir out/t4-usb-emmc-raw/reference-official/rk3399-usb-debian-trixie-core-4.19-arm64-20260319 \
    --version v24 \
    --force
'
```

典型验证命令：

```sh
orb -m lumelo-dev -u root /bin/sh -lc '
  cd /Volumes/SeeDisk/Codex/Lumelo &&
  ./scripts/verify-t4-usb-emmc-official-layout-package.sh \
    out/t4-usb-emmc-official-layout/lumelo-t4-usb-emmc-official-layout-20260502-v24
'
```
- `scripts/flash-t4-emmc-linux.sh`
  - 明确高风险脚本
  - 默认 dry-run
  - 必须显式 `--i-understand-this-erases-emmc`
- `docs/T4_USB_eMMC_Flashing_Checklist.md`
  - 操作清单
  - 救砖清单
  - 刷后验收清单

## 10. 首版 MVP 建议

第一阶段不要直接做最复杂的 `update.img`，也不要把 MVP 命名成最终正式固件。

当前顺序：

1. 先做 Win11 RKDevTool `Download Image` 多分区 USB-to-eMMC package。
   - 目标：让 Lumelo package 形态对齐已验证成功的 FriendlyELEC 官方 USB package。
   - 不再把完整 raw disk image 当 `System` 行刷。
2. 离线验证：
   - source raw image 通过现有 rootfs image verifier。
   - `parameter.txt` 与 table 地址一致。
   - `rootfs.img` / `userdata.img` 是合法 Android sparse image。
   - 原始 TF/raw `.img` hash 不变。
3. 真机验证：
   - RKDevTool 显示 `Found One MASKROM Device`
   - 使用 `Download Image`
   - `Write by Address` 勾选
   - 行表与官方 package 一致
   - 刷写完成后拔 TF 卡 cold boot
4. 再补 Linux `rkdeveloptool` 工程路径。
5. 最后评估是否需要 Rockchip `update.img` mass-production package。

## 11. 刷入后验收

刷入 eMMC 后必须验证：

- `/proc/cmdline` 显示 eMMC 启动语义。
- `lsblk` 确认 rootfs 在 eMMC。
- `systemctl is-system-running` 不应有关键服务 failed。
- `playbackd / sessiond / controld / media-indexd` 状态。
- WebUI：
  - `http://<T4_IP>/`
  - `http://lumelo.local/`，仅作为增强入口
- mDNS / DNS-SD 发布。
- Wi-Fi provisioning。
- USB DAC auto-select。
- `lumelo-media-smoke play --first-wav`。
- 本地介质挂载 / 扫描。
- 重启后 queue 恢复为 stopped。

## 10. 来源

Official / primary:

- FriendlyELEC NanoPC-T4 Wiki: https://wiki.friendlyelec.com/wiki/index.php/NanoPC-T4
- FriendlyELEC NanoPC-T4 Chinese Wiki: https://wiki.friendlyelec.com/wiki/index.php/NanoPC-T4/zh
- FriendlyELEC sd-fuse_rk3399: https://github.com/friendlyarm/sd-fuse_rk3399
- FriendlyELEC RK3399 Type-C burn template: https://wiki.friendlyelec.com/wiki/index.php/Template:RK3399-BurnOS-with-TypeC
- Rockchip rkdeveloptool: https://github.com/rockchip-linux/rkdeveloptool
- Radxa rkdeveloptool docs: https://docs.radxa.com/en/som/cm/cm5/radxa-os/low-level-dev/rkdeveloptool

Community / secondary:

- Armbian NanoPC-T4 eMMC discussions: https://forum.armbian.com/
- DietPi NanoPC-T4 eMMC discussion: https://dietpi.com/forum/t/nanopc-t4-rk3399-emmc-flashing-from-sd-card/14492
- Firefly eMMC flashing docs: https://roc-rk3328-cc.readthedocs.io/en/latest/flash_emmc.html
- PINE64 RK3399 boot sequence: https://pine64.org/documentation/General/RK3399_boot_sequence/
