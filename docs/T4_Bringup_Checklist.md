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

- `http://<T4_IP>/` 可打开
- `http://<T4_IP>/healthz` 可返回
- `http://<T4_IP>/provisioning-status` 可返回
- `http://<T4_IP>/logs.txt` 可返回
- `http://lumelo.local/` 在支持 mDNS 的同一局域网客户端上可打开
- `_http._tcp` mDNS service 发布 `lumelo.local:80`
- `ssh root@<T4_IP>` 可登录
  当前开发 / 测试镜像默认登录信息见 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)

如果这里任何一项失败，不要先假定是手机或 APK 问题。

### 6.2 第二层：SSH 登录后必查

SSH 进入板子后，默认先跑：

```sh
systemctl --no-pager --full status \
  ssh.service \
  lumelo-ssh-hostkeys.service \
  lumelo-bluetooth-uart-attach.service \
  lumelo-bluetooth-provisioning.service \
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
- 它不能替代经典蓝牙 `RFCOMM / SPP` provisioning 主链验证
- 但如果连这一步都过不了，就不该先怀疑手机 APK

### 6.4 第四层：经典蓝牙 provisioning 主链核查

再看当前真正的 provisioning 主链：

- `/healthz` 中 `provisioning_available` 应为 `true`
- `provisioning_state` 不应长期卡在明显异常状态
- `logs.txt` 中不应出现：
  - `Patch not found`
  - `Cannot open directory '/etc/firmware'`
  - `Direct firmware load ... failed`
  - `no hostkeys available`
- 手机端应至少能在：
  - 系统蓝牙设置页
  - 或 APK 的 `Lumelo Scan`
  中看到 `Lumelo T4`
- `Raw BLE Scan`
  - 现在只作为诊断项
  - 看不到 BLE 广播本身，不再单独等于“主配网失败”

### 6.5 第五层：Wi-Fi 与 WebUI 闭环

如果手机已能发现并连接 T4，再继续看：

- Wi-Fi 凭据是否成功写入
- 板子是否拿到正确 IP
- WebUI 是否可打开
- `/library`、首页、播放控制页是否正常

### 6.6 第六层：真实曲库与 ALSA smoke

若本轮需要继续验证真实媒体链，优先先跑最小 smoke，而不是一开始就上整套业务逻辑：

```sh
lumelo-media-smoke smoke --skip-play
lumelo-media-smoke list --first-wav
lumelo-media-smoke play --first-wav
```

判定方式：

- `smoke --skip-play` 应能生成 demo `WAV` 并让 `media-indexd` 成功写入 `library.db`
- `list --first-wav` 应至少返回一条可解析的真实轨道
- `play --first-wav` 应调用 `aplay -D default` 并正常退出
- `http://<T4_IP>/library` 应能看到真实条目，而不是全 `0`

注意：

- 这一步验证的是：
  - 真实媒体文件
  - 索引写库
  - `controld` 读库
  - `ALSA hw` 最小播放链
- 它当前还不等于：
  - `playbackd` 已经接入真实解码与真实输出
  - 首页上的播放控制已经具备最终用户态能力

### 6.7 第七层：`playbackd` 真机输出回归

若本轮要继续验证首页播放控制是否已具备真实用户态能力，至少准备：

- 一首真实 `wav`
- 一首非 `wav` 轨道
  - 当前优先用 `m4a/aac`

最低回归集：

- 也可以直接先跑板端 helper：
  - `lumelo-media-smoke regress-playback --timeout 8`
  - 如果板子上已经有多个 indexed volume：
    - `lumelo-media-smoke regress-playback --mount-root /var/lib/lumelo/test-media --timeout 8`
  - 若要强制指定解码格式：
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format flac`
  - 若是长时长压缩格式：
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format mp3 --skip-mixed`
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format ogg --skip-mixed`
  - 若要真的等待长 `mp3` 自然播完并验证自动切歌：
    - `lumelo-media-smoke regress-playback --timeout 140 --decoded-format mp3`
  - 若要真的等待长 `ogg` 自然播完并验证自动切歌：
    - `lumelo-media-smoke regress-playback --mount-root /var/lib/lumelo/test-media --timeout 140 --decoded-format ogg`
- 对 `wav` 执行：
  - `play / pause / resume / stop`
- 对 `m4a/aac` 执行：
  - `play / pause / resume / stop`
- 做一次混合队列：
  - `m4a -> wav`
  - 等第一首自然播完，看是否自动切到第二首
- 再补一次：
  - `prev`
  - `play_history`

判定方式：

- `wav` 路径应看到：
  - `aplay -D default <real file>`
- 已解码非 `wav` 路径应看到：
  - `aplay -D default -t raw -f S16_LE -c <channels> -r <sample_rate>`
- `STATUS` 与队列当前项应和真机输出保持一致
- 长时长 `ogg` 回归通过时，应看到：
  - 第一首为：
    - `aplay -D default -t raw -f S16_LE ...`
  - 自然播完后自动切到：
    - `aplay -D default /var/lib/lumelo/test-media/Blue Room Sessions/01 - Warmup Tone.wav`

### 6.8 第八层：批量曲库扫描回归

若本轮要确认曲库扫描已经不只是“单首 smoke”，可直接在板子上执行：

```sh
lumelo-media-smoke regress-library-scan
```

判定方式：

- helper 会在独立目录下生成多目录、多格式 fixture
- 也会额外生成专辑封面 fixture：
  - `Album Alpha/folder.jpg`
  - `Album Alpha/cover.jpg`
  - `Album Beta/cover.jpg`
- 然后执行：
  - `media-indexd scan-dir <fixture_root>`
- 最终应输出：
  - `Library scan regression passed`
- 当前这条回归通过时，至少会覆盖：
  - `wav`
  - `m4a`
  - `flac`
  - `mp3`
  - `ogg`
- 并且还应覆盖：
  - `albums = 3`
  - `covered_tracks = 4`
  - `artwork refs = 2`
  - `Album Alpha` 命中 `folder.jpg` 优先级

若这轮还要从 WebUI 再看一眼，执行：

```sh
curl -fsSL http://192.168.1.121/library | rg "library-cover-art|Album Alpha|Album Beta"
curl -I http://192.168.1.121/artwork/thumb/320/<hash>.jpg
```

判定方式：

- `/library` 页面应出现：
  - `class="library-cover-art"`
- `/artwork/thumb/320/...jpg` 应返回：
  - `200 OK`
  - `Content-Type: image/jpeg`

### 6.9 第九层：tagged 元数据真机回归

若本轮要确认“更像真实用户曲库”的元数据已经贯通到索引与 WebUI，至少补一轮：

- 专辑名
- 专辑艺人
- 曲目艺人
- 年份
- 流派
- `disc_no`
- `track_no`
- 专辑封面

当前推荐最小样本：

- `Northern Signals`
  - `Disc 1`
  - `Disc 2`
- `Transit Lines`

判定方式：

- `media-indexd scan-dir <tagged_root>` 后，数据库中应能查到：
  - `album_title`
  - `album_artist`
  - `year`
  - `disc_no`
  - `track_no`
  - `genres`
- `/library` 页面应出现：
  - 专辑标题
  - 专辑艺人
  - 年份
  - 对应曲目标题与曲目艺人
  - 封面缩略图
- 至少再从这套 tagged 曲库里直接点播一首：
  - 确认不是“只会展示，不会播放”

### 6.10 第十层：外部媒体最小入口

若当前现场还没有把 TF / USB 热插拔链完全做完，至少先确认“已挂载介质如何安全入库”有明确落脚点。

当前可用命令：

```sh
lumelo-media-import list-mounted
lumelo-media-import scan-mounted
lumelo-media-import scan-path /absolute/path/to/media-root
```

判定方式：

- 没有外部介质时：
  - `list-mounted` 应返回：
    - `[]`
- 对一个显式目录执行：
  - `scan-path <path>`
  - 应能成功触发：
    - `media-indexd scan-dir <path>`
- 若当前正处于播放期：
  - 再执行 `scan-path` 或 `scan-mounted`
  - 应拒绝扫描并返回：
    - `playback quiet mode is active; refusing media scan unless --force is used`

### 6.11 第十一层：模拟块设备导入回归

若现场暂时没有真 TF / USB 介质，但想先确认“块设备导入主链”是否成立，可以用 loop ISO 做一轮替代回归。

最小流程：

```sh
losetup -f --show /tmp/lumelo-metadata-suite.iso
lumelo-media-import import-device /dev/loop0
lumelo-media-import reconcile-volumes
```

判定方式：

- `import-device` 应能完成：
  - 挂载到 `/media/<label>`
  - 触发 `media-indexd scan-dir <mountpoint>`
  - 输出稳定 `volume_uuid`
- 随后数据库中应能查到：
  - 该 `mountpoint` 下的真实轨道
- 再至少从这块模拟介质里点播一首：
  - 验证不是“只会挂载入库，不会播放”
- 卸载后执行：
  - `reconcile-volumes`
  - 数据库中的该 volume 应转为离线

### 6.12 第十二层：稳定性回归

当前板端已补了两条正式回归命令，优先用它们收口“服务恢复”和“坏文件边界”。

执行：

```sh
lumelo-media-smoke regress-playbackd-restart --mount-root /var/lib/lumelo/test-media-tagged --timeout 8
lumelo-media-smoke regress-bad-media --timeout 8
```

判定方式：

- `regress-playbackd-restart`
  - 应输出：
    - `Playbackd restart regression passed`
  - 且应确认：
    - `queue_entries` 保持
    - `current_track` 保持
    - 服务重启后状态回到 `stopped`
- `regress-bad-media`
  - 应输出：
    - `Bad-media regression passed`
  - 当前允许出现的现状是：
    - 坏文件仍会被索引
  - 但必须满足：
    - 播放坏文件不会拖挂 `playbackd`
    - 状态会进入 `quiet_error_hold`
    - 随后有效轨道仍能恢复为 `quiet_active`

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

- App 的 `Lumelo Scan`、或系统蓝牙设置页都看不到 `Lumelo T4`
  - 优先怀疑 classic Bluetooth discoverable / pairable 主链没真正起来

- App 的 `BLE TEST SCAN` 能看到很多别的设备，但看不到 `Lumelo T4`
  - 这条现在只说明：
    - Raw BLE 诊断广播没真正发出来
  - 不能单独替代 classic 主链判断

## 8. 与 Raw BLE 诊断的关系

当前角色要限定清楚的是：

- 经典蓝牙发现 / 连接是当前 provisioning 主链
- `Raw BLE Scan` 只保留为诊断项
- `Raw BLE Scan` 看不到目标广播，不再自动等于“APK 主流程失败”

因此当前 bring-up 判断顺序应是：

- 先看 classic Bluetooth 主链是否可发现、可连接、可拿到 `device_info`
- 再把 `Raw BLE Scan` 当成“板端是否仍在发 BLE 诊断广播”的辅助证据
