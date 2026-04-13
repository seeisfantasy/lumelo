# Lumelo repository AGENTS.md

## 1. 先看文档，不要猜
新窗口进入仓库后，先按这个顺序路由：

1. `docs/README.md`
2. `docs/AI_Handoff_Memory.md`
3. `docs/Development_Progress_Log.md`
4. `docs/Product_Development_Manual.md`
5. `docs/T4_Bringup_Checklist.md`

专项场景再补读：
- 开发环境、出包、工作区：`docs/Development_Environment_README.md`
- 配网协议：`docs/Provisioning_Protocol.md`
- T4 无线金样：`docs/T4_WiFi_Golden_Baseline.md`
- 给外部 AI 的静态审查入口：`docs/review/`

文档权威边界：
- 长期产品边界：`docs/Product_Development_Manual.md`
- 当前阶段、已验证事实、未闭环事项：`docs/AI_Handoff_Memory.md` + `docs/Development_Progress_Log.md`
- 真机 bring-up / 烧录后核查：`docs/T4_Bringup_Checklist.md`
- 外部 AI 静态审查：`docs/review/`
  - 若与仓库真实文件或主文档冲突，以仓库真实文件和主文档为准。
- 不要把同一条规则复制进 3 份文档。

## 2. 当前产品边界：不要越界
当前正式目标是：
- `V1`
- `Local Mode`
- `headless` 本地音频系统

当前 steady-state 主交互仍然是：
- Ethernet / Wi-Fi 下的 WebUI

手机 APK 当前只是：
- BLE / Wi-Fi provisioning 工具
- 联网成功后的 WebView 壳
- 板端异常时的诊断入口

不是：
- 主播放器 App
- 主曲库浏览 App
- steady-state 主控制端

不要主动扩展到：
- Bridge Mode 真功能
- AirPlay / UPnP
- 蓝牙控制 App 正式形态
- 桌面端复杂 UI
- 不在当前阶段的“顺手大升级”

## 3. 先守服务边界
- `playbackd`
  - 播放状态、队列状态、切歌逻辑唯一权威
- `sessiond`
  - 只负责 Quiet Mode 与系统环境切换
- `controld`
  - 负责 WebUI / API / 设置 / 认证
  - 不是队列逻辑权威
- `media-indexd`
  - 是按需索引 worker
  - 不应成为播放期高活跃后台

因此：
- 不要把顺序播放、shuffle、当前播放指针逻辑塞进 `controld`
- 不要让 `sessiond` 参与播放决策
- 不要让 `media-indexd` 在播放期间主动高频工作
- `controld` 崩溃不能影响 `playbackd`

## 4. 当前阶段路由
- 当前阶段优先级以：
  - `docs/AI_Handoff_Memory.md`
  - `docs/Development_Progress_Log.md`
    中的现行未闭环事项为准。
- 不要因为看到历史 BLE / Wi-Fi bring-up 条目，就回到过期主线。
- 用户如果明确改优先级，以用户为准。

## 5. 双层开发策略
默认开发策略：

- `OrbStack / lumelo-dev`
  - 服务逻辑
  - 持久化
  - IPC
  - WebUI
  - `systemd` 基础验证
- `NanoPC-T4`
  - 真实 `ALSA hw`
  - DAC
  - 板级
  - 启动链
  - 热插拔
  - 长稳验证

约束：
- 在 `OrbStack` 中执行 `PLAY`，只代表逻辑链路成功。
- 不代表真实音频已经通过 ALSA 输出到 DAC。

## 6. 变更类型与默认验证
### A. 只改 WebUI / Go API / 文案
- 至少跑受影响的 `go test`
- 做页面或接口最小自测
- 不要触碰播放权威语义

### B. 改 `playbackd / sessiond / media-indexd`
- 至少补相关单元或最小本地验证
- 补 `queue` / `history` / `library.db` 边界检查
- 补 Quiet Mode 状态语义检查
- 若影响真实媒体链，再安排真机 smoke

### C. 改 rootfs / image / `systemd` / firmware / Wi-Fi / 蓝牙 / SSH / 启动链
- 先做离线验证，再让用户上板
- 真机步骤以：
  - `docs/Development_Environment_README.md`
  - `docs/T4_Bringup_Checklist.md`
    为准

### D. 改 Android APK
- 至少跑构建
- 跑关键按钮路径自测
- 确认与当前板端主链一致
- 不要把 APK 误做成 steady-state 主控制端

## 7. 真机与高风险动作
- 涉及 rootfs、镜像、`systemd`、firmware、无线链、SSH、启动链时，不要跳过离线 gate 就催用户上板。
- 真机 bring-up 时，不要只看“服务自称正常”。
- 至少交叉确认：
  - 外部可达性
  - SSH
  - `systemctl` / `journalctl`
  - `rfkill`
  - 蓝牙控制器状态
  - 手机实扫 / 真播放结果

## 8. 真实媒体链 gate
- 触及媒体扫描、真实曲库、ALSA、解码链、`playbackd` 真机输出、坏文件处理、重启恢复时：
  - 优先用板端 helper
  - 不要手工临时拼一堆命令
- 当前 canonical helper：
  - `lumelo-media-smoke`
  - `lumelo-media-import`

## 9. 当前已知长期稳定原则
- `Playback Quiet Mode` 是一等公民，不是附属优化。
- 输出链错误按 fail-stop 处理，不做隐式输出回退。
- 内容错误允许受控恢复，但不能拖挂 `playbackd`。
- 队列恢复入口只有 `queue.json`。
- 重启后统一进入 `stopped`，不自动恢复播放。
- 运行时轻量对象放 `/run/lumelo/`
- 持久化状态放 `/var/lib/lumelo/`

## 10. 文档维护边界
- 长期边界改 `docs/Product_Development_Manual.md`
- 当前阶段改 `docs/Development_Progress_Log.md`
- 烧录核查改 `docs/T4_Bringup_Checklist.md`
- 新窗口路由优先维护 `docs/README.md`
- 外部 AI 静态审查入口维护在 `docs/review/`

## 11. 何时用 subagent
可以显式要求 spawn subagents 的典型场景：
- 一次问题同时跨 Rust / Go / `systemd` / Android / docs
- 需要并行比对“文档现状 / 代码现状 / 板级风险 / 回归覆盖”
- 需要预发布 review：正确性 / 板级风险 / 文档一致性 / 回归覆盖

不要用 subagent 的场景：
- 一两个文件的小修
- 会同时改同一批文件的任务
- 正在连板、看日志、出一次很窄的补丁

默认原则：
- 多 agent 主要做读、比、查、审
- 真正改代码由主 agent 收口
- subagent 默认不负责 SSH 改状态、runtime update、烧录、磁盘操作或其他高副作用写动作，这些统一由主 agent 收口
