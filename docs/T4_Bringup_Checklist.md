# NanoPC-T4 Bring-up Checklist

## 1. 文档用途

本文件是 `NanoPC-T4` 真机 bring-up 的最小核查清单。

用途不是替代开发日志，而是把“每次烧录后都该核什么”固定下来，避免再次出现：

- 镜像离线验收通过
- 但板级无线 firmware / patch 仍缺失
- 或 SSH / 蓝牙只是上层看起来正常，底层其实没真正起来

## 2. 适用时机

以下任一情况，烧录后都应跑完本清单：

- 新出一张 `lumelo-t4-rootfs-YYYYMMDD-vN.img`
- 改了蓝牙、Wi-Fi、SSH、`systemd`、board bring-up、firmware、启动链
- 现场再次出现“`/healthz` 看起来正常，但手机扫不到 / SSH 进不去 / Wi-Fi 不稳”

## 3. 官方依据

本清单不是凭经验拼出来的，主要依据以下官方资料整理：

- [FriendlyELEC NanoPC-T4 产品页](https://www.friendlyelec.com/index.php?product_id=225&route=product%2Fproduct)
  - 明确板子是 `Bluetooth 4.1`
- [FriendlyELEC NanoPC-T4 Wiki](https://wiki.friendlyelec.com/wiki/index.php/NanoPC-T4)
  - 明确官方系统支持蓝牙
  - 明确曾有 `Bluetooth BLE enabled`
  - 明确曾修复过 Bluetooth firmware 加载问题
- [Murata Wi-Fi/BT Linux Quick Start Guide](https://www.murata.com/-/media/webrenewal/products/connectivitymodule/asset/pub/rfm/data/murata_quick_start_guide_linux.ashx?cvid=20210615064818000000&la=en)
  - 明确 Wi-Fi firmware、NVRAM 与 `/etc/firmware/*.hcd` 是 bring-up 必需文件
  - 明确蓝牙验证路径是：
    - `hciattach`
    - `hciconfig hci0 up`
    - `hcitool scan`
- [BlueZ hciattach_bcm43xx.c](https://sources.debian.org/src/bluez/5.43-2%2Bdeb9u2/tools/hciattach_bcm43xx.c)
  - 明确 Broadcom 蓝牙 patch 会从 `/etc/firmware/` 查找

## 4. 核查原则

- 先过离线镜像验收，再上板
- 上板后同时看：
  - 上层状态
  - 板级日志
  - 真实接口可用性
- 不接受“服务说自己起来了”作为唯一成功标准

一句话说：

- `advertising = true` 不等于手机一定能扫到
- `ssh enabled = true` 不等于 `22` 端口一定真的能进

当前 `NanoPC-T4` 默认以 FriendlyELEC 官方运行态为无线金样：

- Wi-Fi 驱动走 `bcmdhd`
- 蓝牙控制器挂在 `ttyS0`
- 蓝牙 patch 文件是 `/etc/firmware/BCM4356A2.hcd`
- 模块策略文件是 `/etc/modprobe.d/bcmdhd.conf`
- 蓝牙 attach 先等 `/sys/module/bcmdhd`，再执行 `hciattach.rk /dev/ttyS0 bcm43xx 1500000`

## 5. 烧录前离线核查

### 5.1 必跑验收

先对镜像执行：

```sh
./scripts/verify-t4-lumelo-rootfs-image.sh /absolute/path/to/lumelo-t4-rootfs-YYYYMMDD-vN.img
```

通过标准：

- `0 failure(s), 0 warning(s)`

若本轮改动涉及 `NanoPC-T4` 无线链路，再补跑：

```sh
./scripts/compare-t4-wireless-golden.sh \
  --board-base-image /absolute/path/to/rk3399-sd-*.img.gz \
  --image /absolute/path/to/lumelo-t4-rootfs-YYYYMMDD-vN.img
```

通过标准：

- `0 failure(s), 0 warning(s)`
- `BCM4356A2.hcd`
- `hciattach.rk`
- `bcmdhd.conf`
  与官方底图一致

### 5.2 本轮必须特别确认的板级文件

离线验收之外，重点确认这些文件已被纳入镜像：

- `/usr/bin/hciattach.rk`
- `/etc/firmware/BCM4356A2.hcd`
- `/etc/modprobe.d/bcmdhd.conf`
- `lumelo-bluetooth-uart-attach.service`
- `lumelo-ssh-hostkeys.service`

## 6. 烧录后真机核查

### 6.1 第一层：最小外部可见性

先确认：

- `http://<T4_IP>:18080/` 可打开
- `http://<T4_IP>:18080/healthz` 可返回
- `http://<T4_IP>:18080/provisioning-status` 可返回
- `http://<T4_IP>:18080/logs.txt` 可返回
- `ssh root@<T4_IP>` 可登录

如果这里任何一项失败，不要先假定是手机或 APK 问题。

### 6.2 第二层：SSH 登录后必查

SSH 进入板子后，默认先跑：

```sh
systemctl --no-pager --full status \
  ssh.service \
  lumelo-ssh-hostkeys.service \
  lumelo-bluetooth-uart-attach.service \
  bluetooth.service \
  lumelo-wifi-provisiond.service
```

```sh
journalctl -b --no-pager | grep -Ei 'brcm|bcmdhd|dhd|bluetooth|hci|firmware|sshd'
```

```sh
rfkill list
bluetoothctl show
hciconfig -a
```

### 6.3 第三层：官方板级蓝牙 smoke test

如果板端 `hci0` 已存在，继续做最朴素的板级测试：

```sh
hciconfig hci0 up
hcitool scan
```

判定方式：

- `hciconfig hci0 up` 不应报 firmware / patch / UART attach 错误
- `hcitool scan` 至少应能开始 classic Bluetooth 扫描

注意：

- 这一步是“控制器最小存活测试”
- 它不能替代 BLE 配网验证
- 但如果连这一步都过不了，就不该先怀疑手机 APK

### 6.4 第四层：BLE 配网链核查

再看 BLE 配网链本身：

- `/healthz` 中 `provisioning_available` 应为 `true`
- `provisioning_state` 不应长期卡在明显异常状态
- `logs.txt` 中不应出现：
  - `Patch not found`
  - `Cannot open directory '/etc/firmware'`
  - `Direct firmware load ... failed`
  - `no hostkeys available`
- 手机端应至少能在：
  - `BLE TEST SCAN`
  - 或 Lumelo 专用扫描
  中看到设备

### 6.5 第五层：Wi-Fi 与 WebUI 闭环

如果手机已能发现并连接 T4，再继续看：

- Wi-Fi 凭据是否成功写入
- 板子是否拿到正确 IP
- WebUI 是否可打开
- `/library`、首页、播放控制页是否正常

## 7. 典型失败信号与优先怀疑项

- 日志出现 `sshd: no hostkeys available -- exiting`
  - 优先怀疑 SSH host key 自动生成链未生效

- 日志出现 `Cannot open directory '/etc/firmware'`
  - 优先怀疑蓝牙 patch 目录没有从官方底图正确带进镜像

- 日志出现 `Patch not found, continue anyway`
  - 优先怀疑 `BCM4356A2.hcd` 未命中或 `hciattach.rk` 未按官方链路正确 attach

- 日志出现 `Direct firmware load for brcm/brcmfmac4356-sdio.bin failed`
  - 优先怀疑镜像仍在走错误的 `brcmfmac` 路线，而不是官方 `bcmdhd` 路线

- `bluetoothctl show` 里仍是 `Discoverable: no`
  - 优先怀疑适配器状态没有真正切到可发现

- App 的 `BLE TEST SCAN` 能看到很多别的设备，但看不到 `Lumelo T4`
  - 优先怀疑 T4 板端广播没真正发出来

## 8. 与经典蓝牙测试的关系

经典蓝牙测试有价值，但角色要限定清楚：

- 它适合做“控制器活没活”的快速 smoke test
- 它不等于 BLE 广播一定正常
- 它也不等于 GATT 配网链一定正常

所以后续如果加“经典蓝牙可发现”诊断模式，它应该作为：

- 辅助诊断项

而不是替代当前的 BLE 配网路径。
