# T4 Wi-Fi 金样基线与 Lumelo 差异

> 文档边界：
> - 本文件只固定 `NanoPC-T4` 无线板级金样、当前 `Lumelo` 的实现差异，以及需要长期保留的 Wi-Fi 结论。
> - 当天现场进展、当前阻塞和下一步待办看 [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md) 和 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)。
> - 手机与板子的经典蓝牙配网协议、安全传输契约看 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)。
> - 真机 bring-up 操作步骤和排障动作看 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md) 与 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)。

本文件用于固定 `NanoPC-T4` 官方金样在 `Wi-Fi` 相关底座上的真实做法，
并和当前 `Lumelo` 自定义 rootfs 的实现做对比。

当前金样来源：

- `rk3399-sd-debian-trixie-core-4.19-arm64-20260319.img.gz`
- `rk3399-usb-debian-trixie-core-4.19-arm64-20260319.zip`

目标不是“照抄官方整套系统”，而是明确哪些板级依赖必须继承，哪些用户态
网络策略可以继续保持 Lumelo 自己的路线。

## 1. 官方金样已确认的 Wi-Fi 底座

### 1.1 板级驱动与 firmware

- Wi-Fi 驱动走 `bcmdhd`
- 蓝牙走 `hci_uart + btbcm`
- 板载无线为 `AP6356S` 组合模组
- 官方实际加载的 Wi-Fi firmware 路径是：
  - `/system/etc/firmware/fw_bcm4356a2_ag.bin`
  - `/system/etc/firmware/nvram_ap6356.txt`
- 官方蓝牙 patch 路径是：
  - `/etc/firmware/BCM4356A2.hcd`
- 官方驱动策略文件是：
  - `/etc/modprobe.d/bcmdhd.conf`

### 1.2 官方运行态已验证现象

在官方金样真机运行态上，已经确认：

- `bcmdhd` 正常加载
- `/system/etc/firmware/fw_bcm4356a2_ag.bin` 打开成功
- `/system/etc/firmware/nvram_ap6356.txt` 打开成功
- 日志出现：
  - `Firmware up: op_mode=0x0005`
  - `wl_android_wifi_on : Success`
- 未出现我们 Lumelo 镜像那类：
  - `brcmfmac ... -110`
  - `failed backplane access over SDIO`

这说明：

- 官方板级 Wi-Fi firmware / 驱动链是健康的
- 我们后续对 `Lumelo` 的修复必须优先继承这条 vendor 路线

## 2. 官方金样的用户态网络栈

### 2.1 当前观察到的用户态组合

官方金样当前不是 `systemd-networkd + iw` 路线，而是：

- `NetworkManager`
- `ifupdown`
- `dhcpcd-base`
- `wireless-tools`
- `wpa_supplicant`

当前真机采样结果：

- `iw` 未安装
- `systemd-networkd.service` 已安装但默认 `disabled`
- `NetworkManager.service` 默认 `enabled`
- `wpa_supplicant@wlan0.service` 默认 `inactive`

### 2.2 已确认的配置要点

官方金样关键配置如下：

- `/etc/NetworkManager/NetworkManager.conf`

```ini
[main]
plugins=ifupdown,keyfile

[ifupdown]
managed=true
```

- `/etc/NetworkManager/conf.d/12-managed-wifi.conf`

```ini
[keyfile]
unmanaged-devices=wl*,except:type:wifi
```

- `/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf`

```ini
[keyfile]
unmanaged-devices=interface-name:wlan1
```

- `/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf`

```ini
[device]
wifi.scan-rand-mac-address=no
```

- `/etc/network/interfaces`

```ini
# interfaces(5) file used by ifup(8) and ifdown(8)
# Include files from /etc/network/interfaces.d:
source /etc/network/interfaces.d/*
```

## 3. 当前 Lumelo 的差异

### 3.1 已经补齐的正确项

当前 `Lumelo` 已经转回官方板级主线：

- 复制官方 `bcmdhd.conf`
- 复制官方 `/etc/firmware/BCM4356A2.hcd`
- 复制官方 `/system/etc/firmware/fw_bcm4356a2_ag.bin`
- 复制官方 `/system/etc/firmware/nvram_ap6356.txt`
- 复制官方 `hciattach.rk`

### 3.2 当前仍然不同的项

当前 `Lumelo` 用户态网络栈仍然是：

- `iw`
- `wpasupplicant`
- `systemd-networkd`
- `systemd-resolved`
- `lumelo-wifi-apply` 直接写 `wpa_supplicant-<iface>.conf`

对应文件：

- [t4-bringup-packages.txt](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/manifests/t4-bringup-packages.txt)
- [30-wireless-dhcp.network](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/systemd/network/30-wireless-dhcp.network)
- [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)

这意味着：

- 板级 firmware / 驱动链已向官方对齐
- 但用户态网络管理仍是 `Lumelo` 自己的实现
- 后续若 Wi-Fi 连接、DHCP、重连、扫描仍不稳定，不能只看 firmware，还必须检查用户态网络策略是否也要贴近官方

### 3.3 本轮已落地的 Wi-Fi 改造

本轮已经把两类高价值差异先落到仓库里：

- [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)
  现在支持双路径：
  - `NetworkManager`
  - `wpa_supplicant + systemd-networkd`
- 默认策略：
  - `LUMELO_WIFI_BACKEND=auto`
  - 如果 `NetworkManager` 处于 active，则优先走 `nmcli`
  - 否则回退到当前 `wpa_supplicant` 写配置方式
- 无线接口探测已修正：
  - 优先看 `nmcli`
  - 再看 `iw`
  - 最后看 `/sys/class/net/*/wireless`
  - 显式跳过 `p2p-dev*`

这条修正很重要，因为官方金样运行态里同时存在：

- `wlan0`
- `p2p-dev-wlan0`

如果后续镜像没有 `iw`，旧逻辑有概率误把 `p2p-dev-wlan0` 当成主无线接口。

- [lumelo-t4-report](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-t4-report)
  已新增：
  - `nmcli general status`
  - `nmcli device status`
  - `/etc/NetworkManager/NetworkManager.conf`
  - `/etc/NetworkManager/conf.d/*.conf`
  - `/etc/network/interfaces`

- overlay 里已预置官方 `NetworkManager` 基线配置：
  - [NetworkManager.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/NetworkManager.conf)
  - [12-managed-wifi.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/12-managed-wifi.conf)
  - [99-unmanaged-wlan1.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf)
  - [disable-random-mac-during-wifi-scan.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf)
  - [interfaces](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/network/interfaces)

当前这一步的目标不是立刻把 `Lumelo` 整套用户态切到 `NetworkManager`，而是：

- 先把官方金样里已确认有价值的配置固化进镜像
- 让 `lumelo-wifi-apply` 具备双路径兼容能力
- 把后续真机验证需要的诊断信息补齐

## 4. 对 Lumelo Wi-Fi 改造的规划

### 4.1 P0 必须保留

这些项后续不能再回退成通用 Debian 假设：

- `bcmdhd`
- `/etc/firmware/BCM4356A2.hcd`
- `/system/etc/firmware/fw_bcm4356a2_ag.bin`
- `/system/etc/firmware/nvram_ap6356.txt`
- `hciattach.rk`

### 4.2 P1 近期需要补的兼容与验证

- 真机验证时除了 `provisioning-status.json`，还要同时记录：
  - `wpa_supplicant@<iface>.service`
  - `networkctl status <iface>`
  - `nmcli device status`
  - `ip addr show dev <iface>`
  - DHCP 是否真实拿到 IPv4
- 真机验证要增加一条“在连接目标 Wi-Fi 前先做 scan”的观察项

### 4.3 P2 若当前系统仍不稳定，需要考虑的路线

如果板级 firmware 已经对齐官方，但 `Lumelo` 自己的 Wi-Fi 联网仍持续不稳定，
则要认真评估是否改成更接近官方的用户态方案：

- 为 `T4` 单独引入并默认启用 `NetworkManager`
- 或保留当前双路径兼容，但明确：
  - 哪块板默认走 `systemd-networkd + wpa_supplicant`
  - 哪块板默认走 `NetworkManager`

当前阶段不直接切到 `NetworkManager`，原因是：

- 先把板级 firmware / 经典蓝牙配网主通道跑通更关键
- 用户态网络管理器切换属于第二层变量，不应和当前蓝牙主通道改造绑在同一轮

## 5. 当前结论

当前 `Lumelo` Wi-Fi 相关问题不能再简单归因成“缺 firmware”。

更准确的分层应该是：

1. 板级 vendor firmware / 驱动链必须继承官方
2. 用户态网络栈当前仍与官方明显不同
3. 后续若 Wi-Fi 配网或 DHCP 有问题，要同时检查：
   - vendor firmware 是否完整
   - `lumelo-wifi-apply` 是否选择了正确 backend
   - `systemd-networkd` 路线是否足够稳定
   - 是否需要把 `NetworkManager` 设为默认路径
