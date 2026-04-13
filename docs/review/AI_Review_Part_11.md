# AI Review Part 11

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `docs/Development_Progress_Log.md` (2/4)

- bytes: 119711
- segment: 2/4

~~~md
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
~~~

