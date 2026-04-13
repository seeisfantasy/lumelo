# AI Review Part 12

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `docs/Development_Progress_Log.md` (3/4)

- bytes: 119711
- segment: 3/4

~~~md
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

### 8.4 2026-04-13：补齐 `9.1` 板子侧回归，完成冷启动、自动回连、家庭路由器与双 IP 现场验证

本轮现场先完成了无需冷启动的状态修正与验证：

- 板端
  [classic-bluetooth-wifi-provisiond](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond)
  已改为启动时主动探测当前 Wi‑Fi 连接状态
- `controld` 的 `provisioning-status` / 首页 / 配网页已新增：
  - `wifi_ip`
  - `wired_ip`
  - `all_ips`
- 因此板子在已经连网时，不再错误显示 `advertising`

随后完成了按键进入 `sd` 系统的冷启动真机回归：

- 板子冷启动后确认：
  - `/proc/cmdline` 仍含 `storagemedia=sd`
  - `who -b` 与 `uptime` 可见新启动已发生
- 冷启动后无需手工拉服务：
  - `bluetooth.service`
  - `lumelo-wifi-provisiond.service`
  - `wpa_supplicant@wlan0.service`
  都会自动进入 `active`
- `bluetoothctl show` 仍返回：
  - `Alias = Lumelo T4`
  - `Powered = yes`
  - `Discoverable = yes`

冷启动后的手机侧也完成了现场回归：

- `PJZ110` 上的 Android App 冷开后可重新扫描到：
  - `Classic Bluetooth scan | devices=8 | nameMatch=1`
  - `Lumelo T4`
- 重新 `CONNECT` 后，`hello / device_info` 能正常返回
- 说明经典蓝牙在冷启动后无需手工拉服务即可重新被手机发现并连接

重启后自动回连与双 IP 页面展示也补齐：

- 冷启动后板子先自动回连到热点 `isee_test`
- 当时现场可见：
  - `end0 = 192.168.1.120`
  - `wlan0 = 192.168.43.170`
- `/provisioning-status`、`/healthz` 与首页都正确显示：
  - `state = connected`
  - `ssid = isee_test`
  - `wifi_ip = 192.168.43.170`
  - `wired_ip = 192.168.1.120`
  - `all_ips = [192.168.1.120, 192.168.43.170]`

家庭路由器场景也已完成现场验证：

- 手机切到正式家庭路由器 `iSee`
- 现场先通过手机分享二维码导出精确的 Wi‑Fi 参数，再用于板端验证
- 通过经典蓝牙下发家庭路由器凭据时，一度出现：
  - `state = waiting_for_ip`
  - 随后 `dhcp_timeout`
- 继续排查后确认：
  - 经典蓝牙加密下发链本身是通的
  - 板端最终使用精确 PSK 后，`wpa_cli -i wlan0 status` 返回：
    - `wpa_state = COMPLETED`
    - `ssid = iSee`
    - `ip_address = 192.168.1.121`
- 重启 `lumelo-wifi-provisiond.service` 刷新状态后，`/provisioning-status` 与首页正确显示：
  - `ssid = iSee`
  - `wifi_ip = 192.168.1.121`
  - `wired_ip = 192.168.1.120`
  - `all_ips = [192.168.1.120, 192.168.1.121]`
- 现场还额外确认：
  - Mac 可访问 `http://192.168.1.120:18080/` 与 `http://192.168.1.121:18080/`
  - 手机可 `curl` 与 `ping` `192.168.1.121`

阶段结论：

- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
  里原先 `9.1 板子侧` 想补的几项，本轮已经全部补齐：
  - 冷启动后的蓝牙自动起来
  - 手机冷启动后可重新扫描连接
  - 重启后 Wi‑Fi 自动回连
  - 家庭路由器场景
  - 双网卡 / 双 IP 页面与状态展示
- 当前仅剩的硬件边界是：
  - 这块板子若不按键，默认仍会进入 `eMMC`
  - 因此“完全无人工按键地冷启动进入调试 `sd` 系统”不属于本轮已解决项

### 8.5 2026-04-13：补齐真实曲库与 `ALSA hw` 最小 smoke，恢复 `/library` 读库与 `playbackd` socket

这轮先修了两个会挡住后续业务回归的基础问题：

- `controld` 已从 `github.com/mattn/go-sqlite3` 切到纯 Go 的 `modernc.org/sqlite`
  - 目标是让板端交叉构建后的 `/library` 不再因为 `CGO_ENABLED=0` 落到 sqlite stub
- `lumelo-wifi-provisiond.service` 已补 `RuntimeDirectoryPreserve=yes`
  - 目标是避免它重启时把共享的 `/run/lumelo` 目录清空，连带删掉 `playbackd` 的 socket

本地验证：

- `services/controld` 已通过：
  - `go mod tidy -go=1.22`
  - `go test ./...`
- 新增板端 helper：
  - [lumelo-media-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-smoke)
  - 用于在板上重复执行：
    - 生成 demo `WAV`
    - `media-indexd scan-dir`
    - 列出已索引轨道
    - `aplay` 播放第一条 `WAV`

现场板端验证：

- 热更新新版 `controld` 与 `lumelo-wifi-provisiond.service` 后：
  - `/run/lumelo/playback_cmd.sock`
  - `/run/lumelo/playback_evt.sock`
  已重新出现
- `http://192.168.1.121:18080/library` 已恢复：
  - `Database: online`
  - 不再出现 sqlite `CGO_ENABLED=0` stub 错误
- 板上生成真实媒体并索引成功：
  - `media-indexd scan-dir /var/lib/lumelo/test-media`
  - 当前可见：
    - `1 volume`
    - `1 album`
    - `1 track`
  - 页面条目为：
    - `Warmup Tone`
    - `Blue Room Sessions/01 - Warmup Tone.wav`
- `ALSA hw` 最小真实播放已通过：
  - `aplay -D default /var/lib/lumelo/test-media/Blue Room Sessions/01 - Warmup Tone.wav`
  - 退出码 `0`
- 新 helper 也已现场通过：
  - `lumelo-media-smoke list --first-wav`
  - `lumelo-media-smoke play --first-wav`
  - `lumelo-media-smoke smoke --skip-play`

阶段结论：

- “真实曲库索引 + `controld` 读库 + `ALSA hw` 最小播放链”已经不是空白项
- 目前真正还没闭环的是：
  - `playbackd` 仍是队列/状态 authority
  - 还没有把真实媒体解码与 ALSA 输出接进 `playbackd`

### 8.6 2026-04-13：将 `playbackd` 接入真机真实输出，完成 `wav` 轨道的 `play / pause / resume / stop`

这轮继续沿着 `8.5` 往下推进，把 `playbackd` 从“只有队列状态权威”推进到“已经能在真机上真的出声”。

代码侧本轮落地：

- [playbackd/Cargo.toml](/Volumes/SeeDisk/Codex/Lumelo/services/rust/crates/playbackd/Cargo.toml)
  新增 `rusqlite`
- [playbackd/main.rs](/Volumes/SeeDisk/Codex/Lumelo/services/rust/crates/playbackd/src/main.rs)
  已新增：
  - `library.db` 的 `track_uid -> mount_path + relative_path` 解析
  - 最小输出控制器：
    - `start`
    - `pause`
    - `resume`
    - `stop`
  - 运行时进程监控：
    - 自然播完后自动把状态拉回 `stopped`
  - 当前板端真实输出路径：
    - `playbackd -> aplay -D default -> ALSA hw`
  - 当前策略：
    - `wav` 允许真实输出
    - 其他格式返回 `unsupported_format`
- [services/controld/internal/api/server.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/internal/api/server.go)
  首页默认 `track id` 已改为：
  - 优先当前播放曲目
  - 否则曲库第一首已索引轨道
- [services/controld/web/templates/index.html](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/templates/index.html)
  已修复播放事件 `EventSource` 路径的双重引号问题

本地与 Linux ARM 编译验证：

- 本机：
  - `cargo test --manifest-path services/rust/Cargo.toml -p playbackd`
  - `go test ./...`
  都通过
- `lumelo-dev` OrbStack Linux arm64：
  - `cargo test --manifest-path services/rust/Cargo.toml -p playbackd`
  - `cargo build --manifest-path services/rust/Cargo.toml --release -p playbackd`
  都通过

现场板端验证：

- 已热更新：
  - `/usr/bin/playbackd`
  - `/usr/bin/controld`
- `playbackd.service` 启动日志已新增：
  - `library db: /var/lib/lumelo/library.db`
  - `audio device: default`
- 为了拉长验证窗口，板上重新生成了 `12` 秒 demo `WAV`：
  - `track_uid = 63bd597223448ebb`
- 首页默认输入已自动指向这条真实轨道

真机回归结果：

- `Play`
  - 首页返回：
    - `PLAY -> state=quiet_active current=63bd597223448ebb`
  - 板端可见：
    - `aplay -D default /var/lib/lumelo/test-media/Blue Room Sessions/01 - Warmup Tone.wav`
- `Pause`
  - 首页返回：
    - `PAUSE -> state=paused current=63bd597223448ebb`
  - 板端 `ps` 可见：
    - `aplay` 进程状态为 `T`
- `Resume`
  - 再次对同一 `track id` 执行 `Play`
  - 首页返回：
    - `PLAY -> state=quiet_active current=63bd597223448ebb`
  - 板端 `aplay` 进程状态恢复为运行态
- `Stop`
  - 首页返回：
    - `STOP -> state=stopped current=63bd597223448ebb`
  - 板端 `pgrep aplay` 为空
- 自然播完
  - 等待 `12` 秒后
  - `/healthz` 返回：
    - `playback_state = stopped`
- `TrackChanged`
  - 追加同一轨道后执行 `Next`
  - 首页返回：
    - `NEXT -> state=quiet_active current=63bd597223448ebb`
  - 板端再次出现真实 `aplay`

附带结果：

- 队列条目已经不再只显示手工 `track_id`
- 当前 `/` 页队列里可见：
  - `Warmup Tone`
  - `Blue Room Sessions/01 - Warmup Tone.wav`

阶段结论：

- `playbackd` 现在已经不是“只改状态不出声”
- 在当前板子镜像上，`wav` 轨道已经能由 `playbackd` 真正驱动 `ALSA hw`
- 剩下还没闭环的重点已经收敛为：
  - 非 `wav` 格式的真实解码链
  - `prev / play_history / 更长队列自动切歌`

### 8.7 2026-04-13：为 `playbackd` 接入第一版非 `WAV` 解码，并完成 `m4a/aac` 真机回归

在 `8.6` 的基础上继续推进，这轮目标是把 `playbackd` 从“`wav` 真机可播”推进到“第一类非 `wav` 格式也能真机播放”，并顺手补掉切歌竞态。

代码侧本轮落地：

- [playbackd/Cargo.toml](/Volumes/SeeDisk/Codex/Lumelo/services/rust/crates/playbackd/Cargo.toml)
  - 新增 `symphonia`
- [playbackd/main.rs](/Volumes/SeeDisk/Codex/Lumelo/services/rust/crates/playbackd/src/main.rs)
  - 已新增：
    - `symphonia` 解码探测与首包 `PCM` 规格解析
    - 非 `wav` 路径：
      - `playbackd -> symphonia decode -> aplay -t raw -f S16_LE -c <channels> -r <sample_rate>`
    - `wav` 仍保持直接文件路径播放：
      - `playbackd -> aplay -D default <file>`
    - 切歌时旧 `aplay` 进程退出等待：
      - 避免 `Prev/Next/TrackChanged` 时新旧进程抢占同一 `ALSA` 设备导致 `exit 1`
- [lumelo-media-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-smoke)
  - 已新增：
    - `regress-playback`
    - `--decoded-format`
  - 用来在板端本地自动执行：
    - `wav` 的 `play / pause / resume / stop`
    - 已解码格式的 `play / pause / resume / stop`
    - `decoded -> wav` 的自动切歌

本地与 Linux ARM 编译验证：

- 本机：
  - `cargo test --manifest-path services/rust/Cargo.toml -p playbackd`
  - 通过
- `lumelo-dev` OrbStack Linux arm64：
  - 需要显式设置：
    - `CARGO_HOME=/var/tmp/lumelo-cargo-home`
    - `CARGO_TARGET_DIR=/var/tmp/lumelo-cargo-target`
    - `TMPDIR=/var/tmp`
  - 然后执行：
    - `cargo test --manifest-path services/rust/Cargo.toml -p playbackd`
    - `cargo build --manifest-path services/rust/Cargo.toml --release -p playbackd`
  - 都通过

测试素材准备：

- 本机用 `python3` 生成了一条 `decoder-check.wav`
- 再用系统自带 `afconvert` 转成：
  - `03 - Decoder Check.m4a`
- 已推送到板子：
  - `/var/lib/lumelo/test-media/Blue Room Sessions/03 - Decoder Check.m4a`
- 另外又生成并推送了一条：
  - `/var/lib/lumelo/test-media/Blue Room Sessions/04 - Decoder Check.flac`
- 重新扫描后新增曲目：
  - `track_uid = c6c2c99784330e4a`
  - `format = m4a`
  - `track_uid = f1d26abfb3ef8eca`
  - `format = flac`

真机回归结果：

- `m4a Play`
  - `STATUS -> state=quiet_active current=c6c2c99784330e4a`
  - 板端可见：
    - `aplay -D default -t raw -f S16_LE -c 2 -r 44100`
- `m4a Pause`
  - `STATUS -> state=paused`
  - 板端 `ps` 可见：
    - `aplay` 进程状态为 `T`
- `m4a Resume`
  - 再次对同一 `track id` 执行 `Play`
  - `STATUS -> state=quiet_active`
  - 同一 `aplay` 进程恢复运行
- `m4a Stop`
  - `STATUS -> state=stopped`
  - 板端 `pgrep aplay` 为空
- 混合队列自动切歌：
  - 先 `PLAY m4a`
  - 再 `QUEUE_APPEND` 一首真实 `wav`
  - 等 `m4a` 自然播完后：
    - `current_track` 自动切到 `63bd597223448ebb`
    - `last_command = auto_next:63bd597223448ebb`
    - 板端 `aplay` 命令从：
      - `-t raw ...`
      切回：
      - 真实 `wav` 文件路径播放
- `Prev`
  - 之前会因为旧 `aplay` 未退出完毕而进入：
    - `quiet_error_hold`
  - 本轮修复后，真机已恢复正常：
    - `prev:c17e54ca8ee2754e`
    - 板端重新拉起对应 `aplay`
- `Play History`
  - 真机已验证可重新拉起目标轨道：
    - `play_history:63bd597223448ebb`
- `flac Play`
  - `STATUS -> state=quiet_active current=f1d26abfb3ef8eca`
  - 板端同样可见：
    - `aplay -D default -t raw -f S16_LE -c 2 -r 44100`
- 自动化回归 helper 现场已通过：
  - `lumelo-media-smoke regress-playback --timeout 8`
    - 自动命中：
      - `wav + m4a`
  - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format flac`
    - 自动命中：
      - `wav + flac`

阶段结论：

- `playbackd` 已不再是“`wav` only”
- 第一版非 `wav` 真实解码链已经在真机上落地并通过
- 当前明确已验证的格式覆盖：
  - `wav`
  - `m4a/aac`
  - `flac`
- 当前剩余重点已经收敛为：
  - `mp3 / ogg` 等其他常见格式的真实文件回归
  - 更长队列与更接近真实用户曲库的批量回归

### 8.8 2026-04-13：补齐 `mp3 / ogg` 真机回归，并落地批量曲库扫描回归 helper

这轮沿着 `8.7` 继续收口两件事：

- 把剩余的常见压缩格式再补掉一轮真机验证
- 把“更接近真实用户曲库的批量扫描回归”固化成板端 helper

测试素材来源：

- 本机通过网络拉取了两条小样本并推到板子：
  - `05 - Remote Sample.mp3`
  - `06 - Remote Sample.ogg`
- 重新扫描后新增：
  - `track_uid = c66533534c922bbc`
  - `format = mp3`
  - `duration_ms = 105822`
  - `track_uid = b667e4dd48a9fc1d`
  - `format = ogg`
  - `duration_ms = 105773`

代码侧本轮落地：

- [lumelo-media-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-smoke)
  - `regress-playback` 已新增：
    - `--skip-mixed`
      - 用于长时长 `mp3 / ogg` 回归时跳过自动切歌等待
    - `--mount-root`
      - 用于在存在多个 indexed volume 时锁定回归目标根目录
  - 新增：
    - `regress-library-scan`
      - 会在独立根目录下生成多目录、多格式批量 fixture
      - 然后执行：
        - `media-indexd scan-dir <fixture_root>`
      - 最后校验：
        - track 数
        - format 集合
        - 目录层级数量

真机回归结果：

- `mp3`
  - 已通过：
    - `play / pause / resume / stop`
  - 板端可见：
    - `aplay -D default -t raw -f S16_LE -c 2 -r 44100`
  - 由于样本时长约 `106` 秒，本轮用：
    - `--skip-mixed`
    - 不等待它自然播完再切歌
  - 之后又单独补跑：
    - `lumelo-media-smoke regress-playback --timeout 140 --decoded-format mp3`
  - 真机已确认：
    - 长时长 `mp3` 自然播完后可自动切到 `wav`
- `ogg`
  - 已通过：
    - `play / pause / resume / stop`
  - 板端同样可见：
    - `aplay -D default -t raw -f S16_LE -c 2 -r 44100`
  - 同样因样本较长，本轮跳过自动切歌等待

批量曲库扫描回归：

- 板端 helper 已成功生成独立回归树：
  - `/var/lib/lumelo/test-media-batch`
- 本轮 fixture 覆盖格式：
  - `wav`
  - `m4a`
  - `flac`
  - `mp3`
  - `ogg`
- 现场执行：
  - `lumelo-media-smoke regress-library-scan`
  - `lumelo-media-smoke regress-playback --mount-root /var/lib/lumelo/test-media --timeout 8 --decoded-format ogg --skip-mixed`
- 真机结果：
  - `mount_path = /var/lib/lumelo/test-media-batch`
  - `tracks = 5`
  - `directories = 3`
  - `formats = ["flac", "m4a", "mp3", "ogg", "wav"]`
  - helper 返回：
    - `Library scan regression passed`

阶段结论：

- `playbackd` 当前真机已验证覆盖：
  - `wav`
  - `m4a/aac`
  - `flac`
  - `mp3`
  - `ogg`
- 板端验证路径也已经固化成两条 helper：
  - `lumelo-media-smoke regress-playback`
  - `lumelo-media-smoke regress-library-scan`
- 当前剩余重点进一步收敛为：
  - 更接近真实用户素材的艺术家/专辑/年份元数据回归
  - 后续可再补真实外部 TF/USB 媒体批量导入回归

### 8.9 2026-04-13：补齐长时长 `ogg -> wav` 自动切歌，并让 `/library` 真渲染封面缩略图

这轮继续把 `8.8` 里最后两个明显缺口收掉：

- 长时长 `ogg` 在真实混合队列下的自动切歌长跑
- `media-indexd` 已产出的 `thumb_rel_path` 真正接入 `controld` 页面，而不是只显示文本路径

代码改动：

- [lumelo-media-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-smoke)
  - `regress-library-scan` 现在会额外生成：
    - `Album Alpha/folder.jpg`
    - `Album Alpha/cover.jpg`
    - `Album Beta/cover.jpg`
  - 并新增验证：
    - `albums` 数量
    - `covered_tracks` 数量
    - `artwork` source/thumb 文件都已落入 `/var/cache/lumelo/artwork`
    - `Album Alpha` 确实优先命中 `folder.jpg`
    - `Album Gamma` 维持无封面
- [main.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/cmd/controld/main.go)
  - 新增 `CONTROLD_ARTWORK_CACHE_DIR`
  - 默认指向：
    - `/var/cache/lumelo/artwork`
- [server.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/internal/api/server.go)
  - 新增只读路由：
    - `/artwork/...`
  - `/library` 的专辑项现在会把 `thumb_rel_path` 转成可访问 URL
- [library.html](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/templates/library.html)
  - 专辑列表现在直接渲染缩略图 `<img>`
- [server_test.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/internal/api/server_test.go)
  - 新增 `/artwork` 路由测试

本地验证：

- `python3 -m py_compile base/rootfs/overlay/usr/bin/lumelo-media-smoke`
- `go test ./...`
  - `services/controld` 全通过

真机验证：

- 板端长时长 `ogg` 自动切歌：
  - `lumelo-media-smoke regress-playback --mount-root /var/lib/lumelo/test-media --timeout 140 --decoded-format ogg`
  - 返回：
    - `Playback regression passed`
  - 并且 `ogg` 自然播完后，已自动切到：
    - `01 - Warmup Tone.wav`
- 板端批量扫描 + artwork 回归：
  - `lumelo-media-smoke regress-library-scan`
  - 返回：
    - `albums = 3`
    - `covered_tracks = 4`
    - `formats = ["flac", "m4a", "mp3", "ogg", "wav"]`
    - `Library scan regression passed`
  - `media-indexd` 现场汇总：
    - `artwork refs = 2`
- `controld` 新二进制已热更新到板子，并重启：
  - `controld.service`
  - 状态：
    - `active`
- 页面级验证：
  - `curl http://192.168.1.121:18080/library`
  - 已出现：
    - `Album Alpha`
    - `Album Beta`
    - `<img src="/artwork/thumb/320/...jpg" class="library-cover-art">`
  - `curl -I http://192.168.1.121:18080/artwork/thumb/320/...jpg`
    - 返回：
      - `200 OK`
      - `Content-Type: image/jpeg`

阶段结论：

- `playbackd` 真实输出这条线目前真机已完成：
  - `wav`
  - `m4a/aac`
  - `flac`
  - `mp3`
  - `ogg`
  - 以及长时长 `mp3 -> wav` / `ogg -> wav` 混合队列自动切歌
- `media-indexd` 批量扫描回归也已从“多目录多格式”推进到：
  - 多目录
  - 多格式
  - 专辑封面发现
  - `thumb/320` 缩略图生成
  - `controld /library` 页面真实渲染
- 当前业务主线剩余重点进一步收敛为：
  - 更接近真实用户素材的艺术家/专辑/年份等元数据回归
  - 外部 TF / USB 媒体的批量导入与长稳回归

### 8.10 2026-04-13：完成 tagged 元数据真机回归，确认真实专辑信息已贯通到 `/library`

这轮把 `8.9` 里剩下的“更像真实用户素材的专辑元数据”补成了真机闭环。

本地先生成了一套 tagged fixture，随后打包推到 T4：

- 根目录：
  - `/var/lib/lumelo/test-media-tagged`
- 专辑一：
  - `Lena March / Northern Signals / Disc 1`
  - `Lena March / Northern Signals / Disc 2`
- 专辑二：
  - `Northline / Transit Lines`
- 覆盖格式：
  - `m4a`
  - `mp3`
  - `flac`
  - `ogg`
- 额外封面：
  - `Northern Signals/folder.jpg`
  - `Transit Lines/cover.jpg`

写入的核心标签：

- 专辑名
- 专辑艺人
- 曲目艺人
- 年份
- 曲目标题
- 流派
- `disc_no`
- `track_no`

现场执行：

- `media-indexd scan-dir /var/lib/lumelo/test-media-tagged`

现场索引汇总：

- `volumes = 3`
- `albums = 6`
- `tracks = 15`
- `artwork refs = 4`

现场数据库结果：

- 专辑：
  - `Northern Signals`
    - `album_artist = Lena March`
    - `year = 2024`
    - `track_count = 3`
    - `thumb_rel_path = thumb/320/1c/da/1cda4703f3bb3115.jpg`
  - `Transit Lines`
    - `album_artist = Northline`
    - `year = 2021`
    - `track_count = 1`
    - `thumb_rel_path = thumb/320/2b/8e/2b8e0866423d8c77.jpg`
- 曲目：
  - `Ember Coast`
    - `artist = Lena March`
    - `disc_no = 1`
    - `track_no = 1`
    - `format = m4a`
  - `Glass Harbor`
    - `artist = Lena March feat. Northline`
    - `disc_no = 1`
    - `track_no = 2`
    - `format = mp3`
  - `Quiet Current`
    - `artist = Northline`
    - `disc_no = 2`
    - `track_no = 1`
    - `format = flac`
  - `Night Platform`
    - `artist = Northline`
    - `disc_no = 1`
    - `track_no = 1`
    - `format = ogg`
- 关联结果：
  - `artists = ["Lena March", "Lena March feat. Northline", "Northline"]`
  - `genres = ["Ambient", "Downtempo"]`

页面级验证：

- `curl http://192.168.1.121:18080/library`
  - 已出现：
    - `Northern Signals`
    - `Transit Lines`
    - `Night Platform`
    - `Glass Harbor`
    - `Ember Coast`
    - `Quiet Current`
- `/library` 页面也已显示：
  - 专辑艺人
  - 年份
  - 真封面缩略图

播放链补充验证：

- 直接从 tagged 曲库播放：
  - `Glass Harbor.mp3`
- `playbackd` 返回：
  - `state = quiet_active`
  - `current_track = 580b1cb448e6afa8`

阶段结论：

- “真实用户曲库元数据回归”这一项现在可以视为已通过：
  - 专辑名
  - 专辑艺人
  - 曲目艺人
  - 年份
  - 流派
  - 分碟目录
  - 封面缩略图
  - 点播
- 当前里程碑剩余重点进一步收敛为：
  - 外部 TF / USB 媒体的批量导入与长稳回归
~~~

## `docs/Development_Progress_Log.md` (4/4)

- bytes: 119711
- segment: 4/4

~~~md
  - 扫描/播放互斥、服务恢复、重启状态等稳定性回归

补充观察：

- 当前板子运行态没有现成的外部媒体管理服务在跑
- `media-indexd.service` 只是静态索引 worker
- 本轮也没有外接 TF / USB 介质插在板子上，因此：
  - 第 2 项已开始探路
  - 但还没进入真正的“插入 -> 自动挂载 -> 扫描 -> 播放 -> 拔出”真机闭环

### 8.11 2026-04-13：补上外部媒体最小入口命令，先收住“已挂载介质如何安全入库”

由于当前板子现场没有插着外部 TF / USB 介质，这轮先把第 2 项推进到“最小可执行入口”。

新增：

- [lumelo-media-import](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-import)

当前能力：

- `list-mounted`
  - 列出当前已挂载的可移动介质候选
  - 过滤目标：
    - `lsblk` 识别为 removable，或 `tran=usb`
    - 挂载点位于 `/media/*` 或 `/mnt/*`
- `scan-path <path>`
  - 对一个显式目录执行：
    - `media-indexd scan-dir <path>`
- `scan-mounted`
  - 依次扫描当前所有已挂载的可移动介质候选
- 所有扫描命令默认都会先检查：
  - `/run/lumelo/quiet_mode`
  - 若当前处于播放期，默认拒绝扫描
  - 只有显式加 `--force` 才会越过

本地验证：

- `python3 -m py_compile base/rootfs/overlay/usr/bin/lumelo-media-import`

现场验证：

- 板端当前无外部介质时：
  - `/usr/bin/lumelo-media-import list-mounted`
  - 返回：
    - `[]`
- 对显式目录执行：
  - `/usr/bin/lumelo-media-import scan-path /var/lib/lumelo/test-media-tagged`
  - 能成功触发：
    - `media-indexd scan-dir /var/lib/lumelo/test-media-tagged`
- 播放期安全边界：
  - 先播放 tagged 曲库中的 `Glass Harbor.mp3`
  - 再执行：
    - `/usr/bin/lumelo-media-import scan-path /var/lib/lumelo/test-media-tagged`
  - 返回：
    - `playback quiet mode is active; refusing media scan unless --force is used`

阶段结论：

- 外部媒体这条线还没有完成“真机插入 -> 自动挂载 -> 自动/手动扫描 -> 播放 -> 拔出”的整条闭环
- 但现在至少已经有了一个可落地的入口：
  - 已挂载介质如何安全入库
- 当前剩余重点进一步收敛为：
  - 真外部 TF / USB 介质在场时，补完整插拔闭环
  - 系统层是否需要继续补自动挂载或自动触发扫描

### 8.12 2026-04-13：把外部媒体入口推进到“模拟块设备导入 -> 入库 -> 播放 -> 下线”

为了在当前没有真 TF / USB 介质插在板子上的情况下，先验证更接近真实块设备的链路，这轮补了一次 loop ISO 模拟回归。

现场步骤：

- 本机用 tagged fixture 目录生成：
  - `lumelo-metadata-suite.iso`
- 上传到板子：
  - `/tmp/lumelo-metadata-suite.iso`
- 板端执行：
  - `losetup -f --show /tmp/lumelo-metadata-suite.iso`
  - `lumelo-media-import import-device /dev/loop0`

现场结果：

- `import-device` 已能完成：
  - 识别 `iso9660`
  - 在 `/media/metadata-suite` 挂载
  - 触发 `media-indexd scan-dir /media/metadata-suite`
  - 使用稳定 `volume_uuid = media-uuid-2026-04-13-07-12-25-00`
- 本轮导入后，在该挂载点下已索引到：
  - `track_count = 4`
  - `formats = ["flac", "m4a", "mp3", "ogg"]`
- 随后直接从这块模拟外部介质里点播 `mp3`：
  - `PLAY 744048cc3fd684c2`
  - `STATUS`
  - 返回：
    - `state = quiet_active`
    - `current_track = 744048cc3fd684c2`
- 最后执行：
  - `umount /media/metadata-suite`
  - `losetup -d /dev/loop0`
  - `lumelo-media-import reconcile-volumes`

阶段结论：

- 即使当前现场没有真 TF / USB 卡在板子上，外部媒体主链也已经推进到了：
  - “模拟块设备 -> 挂载 -> 入库 -> 点播 -> 卸载 -> reconcile”
- 当前还没补上的只剩真正的硬件插拔触发：
  - udev 自动命中
  - 真 TF / USB 在场下的热插入与热拔出

### 8.13 2026-04-13：把稳定性回归固化成板端命令，验证 `playbackd` 重启恢复与坏文件边界

为了把第 2 组“稳定性回归”从临时脚本收成可重复命令，这轮给：

- [lumelo-media-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-media-smoke)

新增了两个正式回归入口：

- `regress-playbackd-restart`
- `regress-bad-media`

本地验证：

- `python3 -m py_compile base/rootfs/overlay/usr/bin/lumelo-media-smoke`

现场验证 1：`playbackd` 重启恢复

- 执行：
  - `lumelo-media-smoke regress-playbackd-restart --mount-root /var/lib/lumelo/test-media-tagged --timeout 8`
- 本轮使用的轨道：
  - `Ember Coast | m4a`
  - `Glass Harbor | mp3`
- 结果：
  - `systemctl restart playbackd.service` 后
  - `current_track` 仍保持：
    - `6cdd17ab60c8b980`
  - `queue_entries = 2`
  - `state_after_restart = stopped`

现场验证 2：坏文件边界

- 执行：
  - `lumelo-media-smoke regress-bad-media --timeout 8`
- helper 会生成：
  - 1 个有效轨道
  - 3 个伪装成：
    - `Broken.mp3`
    - `Broken.flac`
    - `Broken.ogg`
      的坏文件
- `media-indexd` 当前行为：
  - 这些坏文件仍会被索引成轨道
- 但 `playbackd` 当前行为是可恢复的：
  - 对 `Broken.mp3` 执行 `PLAY`
  - 立即返回：
    - `ERR`
  - `STATUS` 进入：
    - `quiet_error_hold`
  - 随后立刻播放有效轨道：
    - 能恢复为 `quiet_active`
  - `playbackd.service` 仍保持 `active`

阶段结论：

- 稳定性这条线里，以下几项现在可以视为已通过：
  - 扫描/播放互斥
  - `playbackd` 服务重启恢复
  - 坏文件播放失败后的服务存活与恢复
- 当前剩余的稳定性重点进一步收敛为：
  - 整机重启后的状态回归
  - 坏文件是否还要在索引层直接过滤，避免出现在用户曲库里
~~~

