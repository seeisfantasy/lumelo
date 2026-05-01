# 里程碑进度文档

## 1. 文档定位

本文件用于把 `Lumelo V1 Local Mode` 的后续开发收口成可执行里程碑。

权威边界：

- 长期产品原则以 [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 为准。
- 当前阶段事实以 [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md) 和 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md) 为准。
- 本文件负责：
  - 版本规划
  - 需求池
  - bug 池
  - 每个条目的功能描述、骨架框架、实现方向和验收口径

约定：

- `P0`：阻塞 V1 架构正确性、基础安全或核心播放语义。
- `P1`：V1 必须闭环，但可在 P0 之后推进。
- `P2`：V1 产品化、扩展性或长期维护质量。
- `bug`：现有实现与手册/已确认产品语义不一致。
- `requirement`：手册已有但尚未完整落地，或为下一阶段必须补齐的能力。

## 2. 版本规划

### 2.1 M1：V1 Architecture Recovery

目标：

- 把当前工程重新拉回 `Product_Development_Manual` 定义的 V1 基础架构。
- 范围必须包含：
  - 所有 `P0` requirement
  - 所有 `P1` requirement
  - 所有优先级的 bug
- 不包含：
  - Bridge Mode 真功能
  - Android 主播放器化
  - 多解码器选择的 V2 真功能
  - 桌面端复杂 UI

M1 完成定义：

- WebUI 控制面具备最低安全闭环。
- 控制面具备 HTTP / UDS / SSE 资源上限和 dev/release endpoint profile。
- `playbackd` 事件语义与 Quiet Mode 契约一致。
- `sessiond` 能实际执行 Quiet Mode 服务切换。
- `sessiond` 在正式播放 Quiet Mode 中能关闭或抑制 mDNS/DNS-SD 广播，停止后恢复。
- `media-indexd` 不再被当作播放期高活跃常驻服务。
- `mode` / `interface_mode` / `ssh_enabled` 的状态不再只是展示字段。
- queue/order/repeat 的基础控制面可由 WebUI/API 稳定调用。
- Android APK 能判断当前手机是否支持浏览器/WebView 直接访问 `http://lumelo.local/`，并自动选择 `.local` 或 IP 入口。
- 已知 bug 池全部关闭或明确降级到非阻塞并写明原因。

M1 不再拆分成新的正式里程碑；执行顺序建议是：

1. 先处理 P0 安全边界和 runtime 语义。
2. 再补 P1 状态 contract 和控制面。
3. 最后做镜像、测试和文档收口。

建议验证：

- `cargo test -p ipc-proto -p playbackd -p sessiond`
- `go test ./...` in `services/controld`
- `systemd-analyze verify` on rootfs units
- `lumelo-media-smoke smoke --skip-play`
- live T4 WebUI 手动播放、pause/resume/stop/next/prev、history、settings smoke
- Quiet Mode live check：播放时 Bluetooth provisioning / media scan domain 不持续活动
- Quiet Mode live check：播放态 mDNS/DNS-SD 不持续广播，停止后 `lumelo.local` 能恢复发布
- Android real-device check：同一台手机能记录 `.local` resolver 探测结果，并按结果打开 `lumelo.local` 或 IP URL

### 2.2 M2：V1 Playback Core Completion

目标：

- 完成 V1 播放体验的核心差异化和错误恢复。
- 重点包括：
  - RAM Window Playback MVP
  - 内容错误 6 秒 auto-skip
  - 大曲库 API 分页和播放能力字段
  - 正式镜像 systemd hardening

M2 完成定义：

- 播放内核不再只是 `aplay path` 的薄包装。
- 内容错误、介质离线、输出链错误都有稳定状态机。
- 大曲库不会强制一次性返回全部 tracks/albums。
- 正式镜像默认安全边界可验收。

### 2.3 M3：V1 Product Release Candidate

目标：

- 出正式候选镜像前的产品化收尾。
- 重点包括：
  - 设置页完整产品交互
  - release/dev 镜像差异固定
  - 文档验收矩阵
  - 长稳测试

M3 完成定义：

- 开发镜像与正式镜像的差异清晰可验证。
- T4 真实播放链完成长时间稳定测试。
- 用户不需要阅读开发文档即可完成基础 setup 和本地播放。

### 2.4 V2：Post-V1 Expansion

目标：

- 只在 V1 收口后进入。
- 候选内容：
  - 多 USB DAC 选择
  - Bridge Mode 真功能
  - Bluetooth Control
  - 更完整的手机端控制体验

V2 前置条件：

- V1 Local Mode 已稳定。
- `mode manager` 已能保证 Bridge 不污染 Local Mode。
- `Audio_Output_Device_Plan.md` 中 V2 语义已确认。

## 3. M1 需求池

### REQ-P0-001：WebUI 认证、首次密码和 session

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- V1 必须提供基础登录能力。
- 首次启动时必须设置单管理员密码，不允许跳过。
- 登录后通过 session cookie 访问 WebUI。
- 播放控制、曲库命令、设置、日志、provisioning 状态等敏感接口必须鉴权。
- `/healthz` 可保留匿名。
- 忘记密码只能通过物理恢复介质触发 reset。

当前偏差：

- `services/controld/internal/auth/auth.go` 只有 `passwordConfigured bool`。
- `services/controld/cmd/controld/main.go` 固定 `auth.NewService(false)`。
- 控制 API 和日志 API 当前未鉴权。
- `auth-recovery` 仍是 placeholder。

骨架框架：

- `services/controld/internal/auth`
  - `Store`：读取/写入 password hash、session secret、first-boot state。
  - `Service`：密码设置、登录校验、session 创建、session 校验、登出。
  - `Middleware`：保护 Web routes 和 `/api/v1/...` routes。
- `services/controld/internal/api`
  - 新增 `/setup-admin`
  - 新增 `/login`
  - 新增 `/logout`
  - 修改 route registration，把敏感 route 包进 auth middleware。
- `base/rootfs/overlay/usr/libexec/lumelo/auth-recovery`
  - 从 physical recovery media 检测 marker。
  - 删除 auth state。
  - 禁用 SSH。
  - 让下次进入 first-boot setup。

实现方向：

1. 密码存储放 `/var/lib/lumelo/auth.json`。
2. password hash 使用强 hash，优先 `argon2id`；若当前 Go 依赖暂不引入，至少保留可迁移 schema。
3. session cookie：
   - `HttpOnly`
   - `SameSite=Lax`
   - 局域网 HTTP 阶段不强制 `Secure`，但 schema 要支持正式 HTTPS。
   - 明确 idle timeout、absolute timeout 和 logout 后 session invalidation。
4. 登录失败需要限速。
5. 所有 state-changing POST 加 CSRF token 或严格 Origin/Referer 校验。
6. auth reset marker 的路径、格式和处理结果必须固定。
7. endpoint 权限矩阵统一由 `REQ-P0-006` 固化。

验收：

- 未设置密码时访问首页跳转到 first setup。
- 设置密码后必须登录才能控制播放。
- 未登录 POST `/api/v1/playback/commands` 返回 401/403。
- 登录后 playback command 正常。
- auth reset marker 生效后重新进入 first setup。

### REQ-P0-002：修正 playback event timing

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- `PLAY_REQUEST_ACCEPTED` 表示用户播放请求已被播放核心接受，系统进入 `pre_quiet`。
- `PLAYBACK_STARTED` 必须表示第一帧音频已经成功写入 ALSA。
- 准备阶段失败时只能发 `PLAYBACK_FAILED`，不能提前发 `PLAYBACK_STARTED`。
- `sessiond` 和 WebUI 不应把“已创建 aplay 子进程”误判为“正式播放开始”。

当前偏差：

- `playbackd` 当前在 command outcome 中同时发 `PLAY_REQUEST_ACCEPTED` 和 `PLAYBACK_STARTED`。
- `derive_output_action()` 又根据 `PLAYBACK_STARTED` 才启动输出。
- 因此 `PLAYBACK_STARTED` 的真实时机早于 ALSA 第一帧。

骨架框架：

- `playbackd`
  - 新增明确阶段：
    - command accepted
    - output preparing
    - output opened
    - first frame written
    - started
  - `OutputController::start()` 不再由 `PLAYBACK_STARTED` 驱动。
  - output thread 在首帧成功后广播 `PLAYBACK_STARTED`。
- `ipc-proto`
  - 保持现有 event 名称，必要时新增 `OUTPUT_OPENED` / `FIRST_FRAME_WRITTEN` 内部事件。
- `sessiond`
  - 保持 `PlayRequestAccepted -> prepare`
  - 保持 `PlaybackStarted -> active`

实现方向：

1. command outcome 对 `PLAY` / `QUEUE_PLAY` / `PLAY_HISTORY` 只发 `PLAY_REQUEST_ACCEPTED`。
2. `derive_output_action()` 改为对 `PLAY_REQUEST_ACCEPTED` 触发 `OutputAction::Start`.
3. `OutputController` 在不同 transport 中确认第一帧写入：
   - WAV direct `aplay path` 不能直接确认 write；M1 必须选择 pipe 化、test/fake output 先验状态机，或把 direct path 明确标成临时 transport 且不伪装 first-frame 语义。
   - decoded/DSD pipe 可在第一次 `stdin.write_all()` 成功后发。
4. 输出打开或首帧失败时发 `PLAYBACK_FAILED`。

验收：

- 使用 fake output test 验证 `PLAYBACK_STARTED` 在 first write 之后。
- ALSA 打不开时不发 `PLAYBACK_STARTED`。
- `sessiond` quiet flag 先进入 `prepare`，首帧后进入 `active`。

### REQ-P0-003：Quiet Mode 服务切换闭环

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- 播放期间停止或冻结非关键服务。
- 播放期间不持续保留手机配网用 Bluetooth provisioning / advertising。
- 媒体扫描、封面处理、缩略图生成等工作在 Quiet Mode 中不主动运行。

当前偏差：

- `sessiond.env` 有 `SESSIOND_FREEZABLE_SERVICES="media-indexd.service"`，但 `sessiond` 当前只读取并打印，没有实际 stop/freeze。
- `SESSIOND_QUIET_STOP_UNITS` 会 stop Bluetooth provisioning 相关 unit，但需要 live 验证和失败处理。

骨架框架：

- `sessiond`
  - `quiet_stop_units`
  - `quiet_start_units`
  - `freezable_units`
  - protected units deny-list
- rootfs
  - `sessiond.env`
  - systemd unit dependency
- WebUI
  - 展示 Quiet Mode 状态，不做高频轮询。

实现方向：

1. `SESSIOND_FREEZABLE_SERVICES` 改为真实参与 reconcile。
2. 对 freezable 服务优先 `systemctl stop`，后续再讨论 cgroup freeze。
3. Protected services 不允许被配置进 stop/freeze 列表；发现冲突 fail-fast。
4. 进入 Quiet Mode 前记录 `QuietReconcileSnapshot`：
   - 原来 active 的 unit 才在退出后恢复
   - 原来 inactive 的 unit 不擅自启动
   - 原来 failed 的 unit 不误恢复成 active
5. Quiet Mode 进入/退出都记录 journal。
6. 对 systemctl 失败采用可诊断失败，不静默吞掉。

验收：

- 播放开始后：
  - `lumelo-wifi-provisiond.service` inactive
  - `lumelo-bluetooth-provisioning.service` inactive
  - `media-indexd.service` 不运行
- 停止播放后 provisioning service 恢复到可配网状态。
- `controld` 和 SSH 不被停止。

### REQ-P0-004：mode manager 最小闭环

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- `mode = local | bridge` 不能只是配置展示字段。
- V1 默认进入 `local`。
- `bridge-mode.target` 在 V1 只作为占位，不启动真实桥接功能。
- 设置页可显示当前 mode；切换 mode 必须经过确认并重启生效。

当前偏差：

- postbuild 静态启用 `local-mode.target`。
- `bridge-mode.target` 存在，但没有按 config 选择 target 的启动逻辑。
- `controld` 只读取 mode，不提供保存/确认/重启路径。

骨架框架：

- `lumelo-mode-manager.service`
  - 启动早期读取 `/etc/lumelo/config.toml`
  - 决定进入 `local-mode.target` 或 `bridge-mode.target`
  - 坏配置默认回 `local`
- `controld settings API`
  - read current settings
  - validate pending settings
  - commit settings
  - request reboot

实现方向：

1. M1 先实现最小 local-only manager：
   - config 缺失/错误时进入 local。
   - bridge config 只进入 bridge placeholder target。
2. `bridge-mode.target` 必须保留基础网络、`controld`、设置页和 `healthz`，让用户能切回 `local`。
3. 设置页先只显示和保存 mode，不实现 Bridge 功能。
4. 切换 mode 写入 config 前要求用户确认 reboot；取消则不写 committed config。
5. 不在 runtime 隐式 isolate target，避免误断连接。

验收：

- `mode=local` 启动 local target。
- `mode=bridge` 启动 bridge placeholder，保留 settings 回退入口，不启动 playback stack。
- 坏 config 回 local，并在 UI 告警。

### REQ-P0-005：控制面资源边界

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- WebUI / API / UDS / SSE 都必须有资源上限。
- 资源超限时返回明确错误，不允许拖挂 `controld` 或 `playbackd`。
- 这是认证之外的基础安全边界。

当前偏差：

- HTTP request body、form post、SSE subscriber、UDS command line、candidate list 长度等边界尚未形成统一 contract。

骨架框架：

- `controld`
  - HTTP server timeout config
  - request body limit middleware
  - form parse size limit
  - SSE subscriber cap
- `ipc-proto` / `playbackd`
  - UDS command line length limit
  - command connection cap
  - track id / queue entry id length validation
- `controld playback command mapping`
  - candidate list max length

实现方向：

1. 设置 HTTP `ReadHeaderTimeout`、`ReadTimeout`、`IdleTimeout`。
2. 限制所有 POST body 和 form size。
3. 限制 `QUEUE_PLAY` / `play_context` candidate list。
4. 限制单个 track id、queue entry id、context id 长度。
5. 限制 SSE subscriber 数量和 UDS command line 长度。
6. 所有超限路径返回明确错误码和日志。

验收：

- 超大 HTTP body 被拒绝。
- 超长 UDS command 被拒绝。
- 超长 candidate list 不会进入 `playbackd`。
- SSE subscriber 超限时返回明确错误。

### REQ-P0-006：dev/release 安全 profile 与 endpoint 矩阵

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- 开发 / bring-up 镜像和正式镜像允许有不同暴露面，但差异必须写成 profile 并可验证。
- release image 默认收紧；dev image 保留无头排障能力。

当前偏差：

- `/healthz`、`/provisioning-status`、`/logs`、`/logs.txt`、SSH 的 dev/release 暴露策略尚未形成统一配置。

骨架框架：

- `config.toml`
  - `profile = "dev" | "release"` 或等价 image build profile
- `controld`
  - route-level auth policy table
  - setup-phase exception
- rootfs
  - dev/release SSH default

实现方向：

1. `/healthz` 始终匿名。
2. `/provisioning-status` 在 dev 可匿名或 setup-token，release 默认登录，setup 阶段可例外。
3. `/logs` / `/logs.txt` 在 dev 可匿名、本地网段限定或 setup-token，release 必须登录。
4. playback commands 和 settings 在 dev/release 都必须登录。
5. SSH dev 默认可开，release 默认关闭。

验收：

- dev profile 可无头排障。
- release profile 未登录不能读 logs 或发控制命令。
- profile 差异在 WebUI 设置页或 diagnostics 中可识别。

### REQ-P0-007：Quiet Mode 中关闭或抑制 mDNS/DNS-SD 广播

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- `lumelo.local` 和 DNS-SD 是 setup / stopped / control 阶段的增强入口。
- Lumelo 的播放目标是极限纯净、极少后台扰动。mDNS 通过已有 `systemd-resolved` 提供，live T4 观测为 `1 thread / RSS ~14 MB`，新增开销很小但不是零。
- 进入正式 `Playback Quiet Mode active` 后，应关闭或抑制 mDNS/DNS-SD 广播；退出 Quiet Mode 后恢复之前状态。
- 该要求不允许通过新增 `avahi-daemon` 等常驻服务实现。当前方向继续使用 `systemd-resolved` / `systemd-networkd`。

当前偏差：

- rootfs 已启用 `MulticastDNS=yes` 并发布 `_http._tcp:80`。
- `sessiond` 还没有在 Quiet Mode 中管理 mDNS/DNS-SD。

骨架框架：

- `sessiond`
  - 扩展 `QuietReconcileSnapshot`，记录每个 active interface 的 mDNS 原始状态。
  - 进入 `pre_quiet` / `active` 时对当前网络 interface 执行 mDNS disable。
  - 退出 Quiet Mode 时恢复进入前状态。
- rootfs / systemd
  - 保留 `systemd-resolved.service` 与 `systemd-networkd.service` 为 protected services，不 stop 核心网络栈。
  - 只切 mDNS/DNS-SD 行为，不切断 WebUI 控制 IP。
- diagnostics
  - `lumelo-t4-report` 输出当前 mDNS 状态。
  - journal 记录进入/退出时的 mDNS reconcile 行为。

实现方向：

1. 先确认 `systemd-resolved` 在 T4 上可用的 runtime toggle，优先使用 `resolvectl mdns <ifname> no/yes` 或等价 D-Bus API。
2. `sessiond` 在收到 `PLAY_REQUEST_ACCEPTED` 后进入 `pre_quiet` 时即可关闭 mDNS；若后续播放失败，按 snapshot 恢复。
3. `PLAYBACK_STARTED` 后保持 mDNS disabled。
4. `PLAYBACK_STOPPED`、输出链 failure、退出 `error_hold` 后恢复 snapshot。
5. 若 toggle 失败，必须记录可诊断错误，不允许静默忽略。

验收：

- stopped/setup 状态下：
  - `dns-sd -G v4 lumelo.local` 能发现 T4。
  - `_http._tcp` service 可发现 `port=80`。
- 播放 Quiet Mode active 状态下：
  - T4 不持续发布 `lumelo.local` / `_http._tcp`。
  - WebUI IP 控制路径仍可访问。
- 停止播放后：
  - `lumelo.local` / `_http._tcp` 能恢复。
  - journal 能看到 disable/restore 记录。

### REQ-P0-008：Android APK 检测 `.local` resolver 支持并自动选择入口

类型：requirement
优先级：P0
所属里程碑：M1

功能描述：

- 产品入口策略固定为：
  - default entry：`http://lumelo.local/`
  - reliable entry：`http://<T4_IP>/`
  - 不开发、不承诺、不验收：`http://lumelo/`
- APK 在蓝牙连接 T4 并发送 Wi-Fi 密码后，必须判断当前手机、当前网络、当前 resolver/WebView 是否能访问 `http://lumelo.local/`。
- 若 `.local` 可用，APK 默认打开 `http://lumelo.local/`。
- 若 `.local` 不可用，APK 自动打开 provisioning status 返回的 `web_url=http://<T4_IP>/`。

当前偏差：

- APK 当前主要信任 provisioning status 的 IP URL。
- 尚未记录“当前手机是否支持浏览器/WebView 直接访问 `.local`”这个能力位。

骨架框架：

- Android APK
  - `LocalHostnameProbe`
    - 对 `http://lumelo.local/healthz` 做短超时真实 HTTP probe。
    - 建议 timeout：2-3 秒。
    - 只在手机已连上目标 Wi-Fi，且 Bluetooth provisioning 已返回 T4 `hostname=lumelo` 后执行。
  - `EntryUrlSelector`
    - probe 成功：选择 `http://lumelo.local/`。
    - probe 失败：选择 `web_url` / IP URL。
  - UI / diagnostics
    - 展示 `.local supported: yes/no/unknown`。
    - 失败时说明“当前手机或网络不支持 `.local` 解析，已使用 IP 入口”。
- T4 / provisioning status
  - 保持返回 `hostname=lumelo`、`web_url=http://<T4_IP>/`、`web_port=80`。
  - 继续发布 `lumelo.local` 与 `_http._tcp.local`。

实现方向：

1. 在 APK provisioning 成功后先读取 `device_info` / `/provisioning-status`。
2. 若有 `wifi_ip` / `web_url`，并且 active Wi-Fi 与发送凭据时 SSID 一致，启动 `.local` probe。
3. 使用 Android app 自己的 HTTP client 做真实请求，不只依赖 `NsdManager` discovery。
4. 后续可追加 `NsdManager` 搜索 `_http._tcp.local` / `_lumelo._tcp.local` 作为发现增强，但不把 NSD 成功等同于浏览器/WebView 可解析 `.local`。
5. 保存最近一次 probe 结果到 app diagnostics，便于现场判断机型差异。

验收：

- 支持 `.local` 的手机：APK 默认打开 `http://lumelo.local/`。
- 不支持 `.local` 的手机：APK 自动打开 `http://<T4_IP>/`，且 UI 明确说明 fallback 原因。
- 开飞行模式、VPN、移动数据干扰等场景时，probe 失败不能阻塞 IP 入口。
- `http://lumelo/` 不出现在 APK 文案、代码路径或验收用例中。

### REQ-P1-001：queue/order/repeat 控制面补齐

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- V1 的播放模式拆分为：
  - `order_mode = sequential | shuffle`
  - `repeat_mode = off | one | all`
- `play_order` 是实际播放顺序，由 `playbackd` 维护。
- WebUI/API 必须能设置 order/repeat。
- queue 应支持基础调序能力。

当前偏差：

- model 有字段，但 IPC 缺少：
  - `SET_ORDER_MODE`
  - `SET_REPEAT_MODE`
  - `QUEUE_MOVE`
- `QUEUE_PLAY` 当前重置为 sequential/off。
- WebUI 没有稳定的随机/重复控制。

骨架框架：

- `ipc-proto`
  - `SetOrderMode(OrderMode)`
  - `SetRepeatMode(RepeatMode)`
  - `QueueMove { queue_entry_id, target_index }`
- `playbackd`
  - 状态变更和持久化
  - shuffle play_order 构建
  - repeat behavior
- `controld`
  - API command mapping
  - WebUI controls

实现方向：

1. 先补 IPC command 和 tests。
2. `SET_ORDER_MODE shuffle` 由 `playbackd` 重建 play_order。
3. `SET_ORDER_MODE sequential` 回到队列自然顺序。
4. `repeat_mode` 不因 ordinary play 自动重置，除非产品明确要求。
5. `shuffle` 下禁用 queue move，返回明确错误。
6. `controld` 不生成随机顺序。

验收：

- 设置 shuffle 后 queue snapshot 的 `play_order` 改变且可持久化。
- repeat one/all 行为符合手册。
- 重启后 order/repeat 恢复但不自动播放。

最小行为矩阵：

| order | repeat | next 行为 | previous 行为 | 队尾行为 |
| --- | --- | --- | --- | --- |
| `sequential` | `off` | 下一首 | 上一首 | 停止 |
| `sequential` | `one` | 当前曲 | 当前曲 | 当前曲 |
| `sequential` | `all` | 下一首 | 上一首 | 回队首 |
| `shuffle` | `off` | `play_order` 下一项 | `play_order` 上一项 | 停止 |
| `shuffle` | `one` | 当前曲 | 当前曲 | 当前曲 |
| `shuffle` | `all` | `play_order` 下一项 | `play_order` 上一项 | 回 `play_order` 首项 |

### REQ-P1-002：内容错误恢复状态机

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- 内容错误进入 `quiet_error_hold`。
- UI 静态显示错误原因和“6 秒之后切换到下一首”。
- 6 秒等待由 `playbackd` 内部维护。
- 连续 auto-skip 上限为 3。
- 任一用户显式操作取消本次 auto-skip。
- 失败项只在运行期存在，不写入 queue/history/library。

当前偏差：

- 现有代码有 `QuietErrorHold` 和错误分类，但缺少完整 6 秒 timer、auto-skip limit、runtime failed set 和用户操作取消机制。

骨架框架：

- `playbackd RuntimeState`
  - `quiet_error_hold_until`
  - `auto_skip_count`
  - `failed_queue_entries_runtime`
  - `last_failure_reason`
- `ipc-proto`
  - 扩展 `PLAYBACK_FAILED` fields：
    - `reason`
    - `class`
    - `recoverable`
    - `keep_quiet`
    - `auto_skip_after_ms`
    - `queue_entry_id`
- WebUI
  - 静态错误提示，不做倒计时动画。

实现方向：

1. 内容错误时标记当前 queue entry failed runtime。
2. spawn timer thread 或 central event loop 在 6 秒后尝试下一首。
3. 用户 `STOP/PLAY/NEXT/PREV/PAUSE/QUEUE_*` 取消 pending auto-skip。
4. 连续 3 次后停止并退出 Quiet Mode。
5. output 错误仍 fail-stop，不 auto next。

验收：

- 坏文件后进入 `quiet_error_hold`。
- 6 秒后自动下一首。
- 连续坏文件最多跳 3 次。
- 用户点击 stop 后不会再自动切歌。

### REQ-P1-003：设置系统写入和重启生效契约

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- 设置系统不只展示 config，而要支持读取、校验、保存和提示。
- `mode`、`interface_mode`、`dsd_output_policy`、`ssh_enabled` 要有明确生效语义。
- 配置解析失败时回滚到 default，并在 UI 明确告警。

当前偏差：

- `controld` 只读 config。
- config parse failure 只写 log，不进入 UI。
- `ssh_enabled` 更多是展示字段，不驱动服务开关。

骨架框架：

- `services/controld/internal/settings`
  - `LoadCurrent`
  - `LoadDefault`
  - `Validate`
  - `SaveAtomic`
  - `Warning`
- `services/controld/internal/api`
  - `GET /api/v1/settings`
  - `POST /api/v1/settings`
  - `POST /api/v1/system/reboot-request`
- rootfs helper
  - SSH enable/disable action

实现方向：

1. config parser 返回 `ConfigWithWarning`。
2. UI 首页/设置页显示 config warning。
3. 保存配置用 atomic write。
4. `ssh_enabled` commit 时调用 systemd enable/disable SSH unit。
5. mode/interface/dsd 采用 pending -> confirm -> commit 流程：
   - 用户修改后先 validate
   - 返回 `requires_reboot=true`
   - 用户确认后才写 committed config
   - 用户取消则不写 committed config

验收：

- 写入合法 config 后 UI 和 API 更新。
- 写入坏 config 后服务使用 default 且 UI 告警。
- SSH 开关能实际改变 service enablement。

### REQ-P1-004：library command 不越过 playbackd 队列权威

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- `playbackd` 是队列和播放顺序权威。
- `controld` 可传递用户上下文，但不应持有随机、repeat、current pointer 等队列语义。

当前偏差：

- `controld` 读取 library snapshot 后从点击 track 开始拼 track list，再发 `QUEUE_PLAY`。
- 该行为目前可作为“上下文播放”，但需要边界和长度上限。

骨架框架：

- 可选方向 A：
  - `controld` 传 `PLAY_CONTEXT {context_type, context_id, start_track_id}`。
  - `playbackd` 或独立 library resolver 生成 queue。
- 可选方向 B：
  - 保持 `QUEUE_PLAY []track_uid`，但明确它是 candidate list。
  - 限制最大长度。
  - shuffle/repeat/play_order 仍由 `playbackd` 决定。

实现方向：

1. M1 优先采用方向 B，改动小。
2. `controld` candidate list 加上限，例如 500 或按页面分页后的可见范围。
3. `QUEUE_PLAY` 不强制重置 order/repeat，除非 command 显式要求。
4. 文档写清楚 `controld` 不构建 shuffle 顺序。

验收：

- library play 不提前暴露 shuffle 后续顺序。
- candidate list 过长时有明确截断或分页策略。
- repeat/order 设置由 `playbackd` 决定。

### REQ-P1-005：持久化对象 atomic write + fsync contract

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- V1 稳定持久化对象必须具备断电友好的写入规则。
- 不能只做普通 write，也不能在解析失败时静默伪装成功。

覆盖对象：

- `/etc/lumelo/config.toml`
- `/var/lib/lumelo/queue.json`
- `/var/lib/lumelo/history.json`
- `/var/lib/lumelo/auth.json`

骨架框架：

- `services/controld/internal/settings`
  - config atomic writer
- `playbackd`
  - queue/history atomic writer
- `controld auth`
  - auth state atomic writer
- common helper
  - `write_tmp_fsync_rename_fsync_dir`

实现方向：

1. 写临时文件。
2. `fsync(file)`。
3. `rename` 到目标路径。
4. `fsync(parent directory)`。
5. 读取时限制文件大小。
6. 解析失败时返回 warning 或显式错误。

验收：

- queue/history/config/auth 写入都走同一 atomic contract。
- 坏文件或超大文件不会拖挂服务。
- fallback 状态能被 UI 或 diagnostics 看见。

### REQ-P1-006：media offline 状态机

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- TF / USB 离线不是普通 content error。
- UI 必须明确提示介质不可访问。
- 默认不自动重扫、不自动重建队列、不自动切下一首。
- 若 current/next 已进入 RAM Window，可由 `playbackd` 决定继续播放。

骨架框架：

- `ipc-proto`
  - `PLAYBACK_FAILED failure_class=media_offline`
  - `MediaOffline` status field
- `playbackd`
  - media offline detection
  - queue item availability state
- `controld` / WebUI
  - offline media badge
  - disabled play action with reason

实现方向：

1. 文件不存在但 media root 离线时归类为 `media_offline`，不要归为普通内容损坏。
2. 保留 queue 和 history，不自动重建。
3. 若用户显式点击离线曲目，返回明确 `media_offline` 错误。
4. RAM Window 设计锁定后，再决定已缓存 current/next 的继续播放行为。

验收：

- 拔掉 TF/USB 后 UI 显示介质离线。
- 离线曲目不可播放且提示明确。
- 不触发自动重扫或队列重建。

### REQ-P1-007：RAM Window Playback design lock

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- RAM Window 是 V1 产品核心差异化能力，不应被误判成可选优化。
- M1 不要求完成 MVP 实现，但必须锁定实现 contract，避免后续 playback event timing 和 media offline 设计互相打架。

骨架框架：

- `playbackd`
  - `ram_window` module boundary
  - cache source type
  - memory budget
  - media offline behavior
- transport
  - current source abstraction
  - next preload abstraction

实现方向：

1. 决定缓存原始文件还是 decoded PCM。
2. 决定单曲最大缓存大小和总内存预算。
3. 决定超大文件降级策略。
4. 决定 DSD 是否进入 RAM Window。
5. 决定介质拔出后 current/next 的继续播放语义。
6. 明确 `aplay path` 是否只是临时 transport。

验收：

- `RAM Window MVP` 的 M2 实现范围清晰。
- `PLAYBACK_STARTED` 首帧语义不再依赖无法验证的 direct path 假设。
- media offline 状态机与 RAM Window 行为一致。

### REQ-P1-008：最低 runtime 权限边界

类型：requirement
优先级：P1
所属里程碑：M1

功能描述：

- 完整 systemd hardening 可以放到 M3，但 M1 不能继续没有最低 runtime 边界。
- 目录权限、UDS socket 权限和服务间访问边界要先明确。

骨架框架：

- rootfs directory ownership
  - `/run/lumelo`
  - `/var/lib/lumelo`
  - `/var/cache/lumelo`
- UDS socket mode
  - command socket
  - event socket
- systemd tmpfiles
  - runtime dirs
  - state dirs

实现方向：

1. `/run/lumelo` 不应 world-writable。
2. UDS command socket 只允许授权服务用户/组访问。
3. event socket 可读范围要明确，慢订阅者断开。
4. dev/release 权限差异必须写进 image profile。
5. 完整 `User=lumelo` 和 `ProtectSystem` 仍可放 M3。

验收：

- 非授权本地用户不能直接写 playback command socket。
- core services 可正常互通。
- runtime/state/cache 目录权限可被离线检查脚本验证。

### REQ-P1-009：曲库本地介质管理入口

类型：requirement
优先级：P1
所属里程碑：M1
状态：部分已修复，2026-05-02；真实扫描按钮尚未人工触发验证

功能描述：

- V1 曲库必须能表达当前 TF / USB 介质状态，而不是只显示已经进入 `library.db` 的专辑和曲目。
- 用户需要看到设备路径、挂载路径、文件系统和已索引 volume 的对应关系。
- 用户需要有明确入口执行挂载、扫描当前介质、扫描所有已挂载介质、选择目录扫描和同步挂载状态。
- 扫描必须是手动按需动作；播放期不得因为打开曲库页而启动 `media-indexd`。

骨架框架：

- `lumelo-media-import`
  - `list-devices`
  - `import-device --mount-only`
  - `import-device`
  - `scan-mounted`
  - `scan-path`
  - `reconcile-volumes`
- `controld`
  - media import client
  - `GET /api/v1/library/media`
  - `POST /api/v1/library/media/commands`
  - SSR form path `/library/media/commands`
- WebUI
  - Library page `本地介质 / 挂载与扫描` section
  - Quiet Mode scan lock warning
  - device row actions

实现方向：

1. `list-devices` 使用 `lsblk` 只列出可操作的 removable USB / TF device。
2. 有 partition children 的父级 disk 不暴露给 UI，避免用户误点 `/dev/sdX`。
3. `controld` 对 `scan_device / scan_mounted / scan_path` 做 Quiet Mode gate。
4. `mount_device / refresh / reconcile_volumes` 不触发扫描，可作为轻量管理动作。
5. 真正 scan 仍委托 `lumelo-media-import` 和 `media-indexd`，不把索引逻辑塞进 `controld`。

验收：

- 插入 U 盘 / TF 后，曲库页显示设备路径和挂载路径。
- 未播放时，用户可以手动扫描当前介质。
- 播放时，扫描按钮 disabled 或 API 返回明确 `playback_quiet_mode_active`。
- 打开曲库页本身不会触发扫描。
- 离线或拔出介质后，状态能由 `reconcile_volumes` 更新。

## 4. M2/M3 需求池

### REQ-P2-001：RAM Window Playback MVP

类型：requirement
优先级：P2
所属里程碑：M2

功能描述：

- 实现 V1 差异化播放内核：播放期间尽量避免持续读取 TF/USB。
- M1 已通过 `REQ-P1-007` 锁定设计；M2 负责 MVP 实现。
- MVP 先做 current + next RAM preload。
- 后续扩展为 prev/current/next 三曲窗口。

骨架框架：

- `playbackd`
  - `ram_window` module
  - `MemoryBudget`
  - `PreloadTask`
  - `WindowEntry`
- transport
  - WAV/FLAC/常见 PCM 先支持
  - DSD 和超大文件后续处理

实现方向：

1. 明确缓存原始文件还是 decoded PCM。
2. 设置单曲最大缓存大小和总内存预算。
3. 当前曲开始前 preload 到 RAM。
4. 下一曲在后台 preload，但 Quiet Mode 中不得高活跃扰动播放。
5. 介质拔出后，如果 current/next 已在 RAM，可继续由 `playbackd` 决定播放。

验收：

- 当前曲从 RAM source 播放。
- 下一曲可预加载。
- 超大文件 graceful fail 或降级策略明确。

### REQ-P2-002：大曲库 API 分页和能力字段

类型：requirement
优先级：P2
所属里程碑：M2

功能描述：

- 曲库 API 不应无上限返回全部 albums/tracks。
- 每个 track 应明确给出 playback support，而不是 WebUI 自己猜扩展名。

骨架框架：

- `libraryclient.Query`
  - `limit`
  - `cursor`
  - `album_uid`
  - `directory`
- API response
  - `next_cursor`
  - `playback_supported`
  - `unsupported_reason`

实现方向：

1. `GET /api/v1/library/snapshot` 增加分页参数。
2. Track support 逻辑集中到后端。
3. WebUI 只消费 support 字段。
4. directory filter 对 `%` / `_` 做 LIKE escape。

验收：

- 1 万 tracks 曲库不会一次性渲染全部。
- unsupported format UI 与 API 一致。

### REQ-P2-003：正式镜像 systemd hardening

类型：requirement
优先级：P2
所属里程碑：M3

功能描述：

- 正式镜像中的长期服务不应默认 root 裸跑。
- systemd unit 需要基础 hardening。

骨架框架：

- 新增 `lumelo` system user/group。
- 调整 runtime/state/cache directory 权限。
- unit 添加 hardening：
  - `User=lumelo`
  - `Group=lumelo`
  - `NoNewPrivileges=true`
  - `ProtectSystem=strict`
  - `ProtectHome=true`
  - `PrivateTmp=true`
  - `ReadWritePaths=/run/lumelo /var/lib/lumelo /var/cache/lumelo`

实现方向：

1. 先在 dev image 验证目录权限和 UDS 权限。
2. `playbackd` 访问 ALSA 可能需要 audio group。
3. `sessiond` 调 systemctl 可能仍需特权，需单独设计 sudoers/polkit 或拆 helper。
4. `controld` 不应拥有系统级特权。
5. 当 `controld` 切到 `User=lumelo` 后，监听 `80/tcp` 需要显式授予 `CAP_NET_BIND_SERVICE` 或改为前置 proxy/socket activation。

验收：

- 服务以非 root 跑通。
- `controld` 非 root 状态下仍可绑定或接收 `80/tcp`。
- WebUI playback command 正常。
- sessiond Quiet Mode helper 权限可控。

### REQ-P2-004：WebUI legacy port compatibility

类型：requirement
优先级：P2
所属里程碑：M2/M3

功能描述：

- 2026-04-26 起，正式 WebUI 入口切到 `80/tcp`，primary URL 为 `http://lumelo.local/` 和 `http://<T4_IP>/`。
- 为减少已安装 APK、旧书签、旧文档和现场调试脚本的断裂，`18080/tcp` 应保留一段兼容窗口。

骨架框架：

- `controld`
  - 可选同时监听 `80` 与 `18080`。
  - 或由轻量 redirect/proxy unit 把 `18080` redirect 到 `80`。
- `rootfs/systemd`
  - 兼容服务必须明确 dev/release profile。
- docs
  - 标明 `18080` 是 legacy/debug fallback，不是新用户入口。

实现方向：

1. 优先实现 `18080 -> 80` HTTP redirect，避免两套入口状态分叉。
2. 若 redirect 复杂，短期至少保持 `18080` 可访问同一 WebUI。
3. APK 新版本默认使用 `80`，但可在兼容期探测旧 `18080`。

验收：

- `http://<T4_IP>/healthz` 可用。
- 兼容期内 `http://<T4_IP>:18080/healthz` 也可用或返回清晰 redirect。
- 新文档不再把 `18080` 作为 primary URL。

## 5. Bug 池

### BUG-P0-001：`PLAYBACK_STARTED` 早发

优先级：P0
所属里程碑：M1
关联需求：REQ-P0-002

现象：

- `playbackd` 在输出启动前就发 `PLAYBACK_STARTED`。

影响：

- `sessiond` 过早进入 active Quiet Mode。
- UI 状态可能显示正在播放，但 ALSA 实际尚未成功写入第一帧。

修复方向：

- 只在 first frame write 成功后广播 `PLAYBACK_STARTED`。

### BUG-P0-002：控制 API 无认证和 CSRF

优先级：P0
所属里程碑：M1
关联需求：REQ-P0-001

现象：

- `/api/v1/playback/commands`
- `/api/v1/library/commands`
- `/commands`
- `/library/commands`
- `/logs`
- `/logs.txt`
- `/provisioning-status`
当前缺少登录保护。

影响：

- 同网段设备可直接控制播放或读取诊断信息。
- 浏览器跨站请求可能触发控制命令。

修复方向：

- auth middleware + session + CSRF/Origin check。

### BUG-P0-003：`sessiond` 未执行 freezable services

优先级：P0
所属里程碑：M1
关联需求：REQ-P0-003

现象：

- `SESSIOND_FREEZABLE_SERVICES` 只被读取打印，没有参与 Quiet Mode reconcile。

影响：

- media worker 域不受 Quiet Mode 管控。

修复方向：

- 将 freezable units 纳入 `reconcile_quiet_services()`。

### BUG-P0-004：auth recovery 是 placeholder

优先级：P0
所属里程碑：M1
关联需求：REQ-P0-001

现象：

- `auth-recovery` 只打印 placeholder 信息。

影响：

- 忘记密码恢复路径不存在。
- 首次密码/重置密码闭环无法验收。

修复方向：

- 实现 physical recovery marker scan 和 auth state reset。

### BUG-P0-005：生产环境 remote API 可触发绝对路径播放

优先级：P0
所属里程碑：M1
关联需求：REQ-P0-001 / REQ-P0-005
状态：已修复，2026-04-26

验证补充：

- 2026-04-26 已完成 live `T4 192.168.71.243` runtime update。
- 已验证 remote `/api/v1/playback/commands` 的 `play` / `queue_play` 绝对路径请求会被 `controld` 拒绝。
- 已验证 `playbackd` 默认启动为 `absolute paths: disabled`，UDS 直连绝对路径播放返回 `absolute_path_playback_disabled`。
- 已完成 Safari 真机 WebUI 手动回归，最终 playback state 回到 `stopped`。

现象：

- 若播放命令允许从 Web/API 传入任意绝对路径，认证之前风险尤其高；认证之后也不应成为 release 行为。

影响：

- remote control 面可能绕过 library track UID 边界。
- 容易把 dev debug 能力误带入正式镜像。

修复方向：

- release profile 禁止 remote API 触发任意绝对路径播放。
- track target 只允许 library track UID、queue entry 或受控 media root 内的显式 dev path。
- dev 绝对路径播放必须由环境变量或 dev profile 显式开启。
- API 层禁止裸路径直通 `playbackd`。

### BUG-P0-006：`playbackd.service` 写死 ALSA output device

优先级：P0
所属里程碑：M1
状态：已修复，2026-05-02

现象：

- `playbackd.service` 固定设置 `LUMELO_AUDIO_DEVICE=plughw:CARD=Audio,DEV=0`。
- 现场 USB DAC 的 ALSA card id 可能不是 `Audio`，例如 `iBassoDC04Pro`。
- 点击播放后 `aplay` 退出，UI 回到 `stopped`，log 只看到 `Broken pipe`。

影响：

- V1 插了 DAC 也可能不能播放。
- 没插 DAC 时错误不够明确，容易被看成“假装播放后停止”。

修复方向：

- V1 不提供多 DAC 选择。
- `playbackd` 每次开始播放前读取当前 `/proc/asound/cards`。
- 发现当前唯一 USB Audio DAC 时自动使用：
  - `plughw:CARD=<detected>,DEV=0`
- 未发现 USB Audio DAC 时返回：
  - `audio_output_unavailable`
- 同时发现多个 USB Audio DAC 时返回：
  - `audio_output_ambiguous`
- 不做隐式 fallback。

验证补充：

- 2026-05-02 已完成 live `T4 192.168.71.12` runtime update。
- 当前 iBasso DAC 下，`lumelo-media-smoke regress-playback --timeout 8 --skip-mixed` 通过。
- WAV 与 FLAC decoded path 均确认使用 `plughw:CARD=iBassoDC04Pro,DEV=0`。

### BUG-P1-001：`media-indexd` 被 local-mode target 直接 Wants

优先级：P1
所属里程碑：M1

现象：

- `local-mode.target` 直接 `Wants=media-indexd.service`。
- verify 脚本还把这个作为期望。

影响：

- 与“按需 indexing worker”语义冲突。
- 容易让未来 media-indexd 变成长驻高活跃服务。

修复方向：

- 移除 local-mode 对 `media-indexd.service` 的直接 Wants。
- 若需要开机 schema 初始化，拆独立 oneshot，例如 `lumelo-library-init.service`。

### BUG-P1-002：`QUEUE_PLAY` 重置 order/repeat

优先级：P1
所属里程碑：M1
关联需求：REQ-P1-001

现象：

- `QUEUE_PLAY` 当前将 `order_mode` 重置为 sequential，将 `repeat_mode` 重置为 off。

影响：

- 用户设置的 repeat/shuffle 状态会被普通播放上下文覆盖。

修复方向：

- 明确产品语义。
- 默认保留 order/repeat；若需要重置，使用显式 command 参数。

### BUG-P1-003：config fallback 无 UI 告警

优先级：P1
所属里程碑：M1
关联需求：REQ-P1-003

现象：

- config parse failure 只写 log，然后使用 defaults。

影响：

- 用户看到系统“正常”，但实际配置已失效。

修复方向：

- `settings.Load()` 返回 warning。
- `/api/v1/system/summary` 和设置页展示 warning。

### BUG-P1-004：rootfs overlay 会打入 ignored artifacts

优先级：P1
所属里程碑：M1

现象：

- overlay 下存在 `__pycache__/*.pyc`。
- build 脚本 `rsync -a overlay/ rootfs/` 会复制进 image/img。

影响：

- ignored artifact 仍可能进入正式镜像。

修复方向：

- 清理 overlay 下 artifact。
- build rsync 排除：
  - `__pycache__/`
  - `*.pyc`
  - `.DS_Store`
  - 其他 host-only artifacts

### BUG-P1-005：内容错误 auto-skip 未实现

优先级：P1
所属里程碑：M1
关联需求：REQ-P1-002

现象：

- 有 `QuietErrorHold` 雏形，但没有 6 秒 auto-skip、连续上限 3、用户操作取消。

影响：

- 内容坏文件恢复行为与手册不一致。

修复方向：

- 在 `playbackd` 内补 runtime error hold state machine。

### BUG-P1-006：WebUI 动态表单拼接存在 DOM XSS 风险

优先级：P1
所属里程碑：M1

现象：

- 浏览器端若从 URL 参数读取 context 并用 `innerHTML` 拼 hidden input，用户可控参数可能进入 DOM。

影响：

- 破坏 WebUI 安全边界。
- 与 M1 auth/CSRF 收口目标冲突。

修复方向：

- 不用 `innerHTML` 拼用户可控参数。
- 改用 `document.createElement("input")`。
- `context_type` 做枚举白名单。
- `context_id` / `context_name` 限长。

### BUG-P1-007：失败或秒切路径可能污染 history

优先级：P1
所属里程碑：M1
关联需求：REQ-P1-002 / REQ-P1-005

现象：

- 若 track 尚未真正到达播放态，失败、秒切或内容错误路径不应写入播放历史。

影响：

- WebUI 播放历史会出现用户没有真正听到的曲目。
- `play_history` 会把错误状态重新暴露成可播放历史。

修复方向：

- history write 只发生在曲目进入真实播放态后。
- content error failed item 只保留运行期标记，不写入 history。
- `PLAYBACK_STARTED` 首帧语义修复后，以该事件作为历史写入的最低门槛之一。

### BUG-P2-001：曲库 snapshot 无分页

优先级：P2
所属里程碑：M1

现象：

- `QuerySnapshot()` 一次性返回 stats、volumes、directories、albums、tracks。

影响：

- 大曲库性能和页面渲染风险。

修复方向：

- M1 至少限制 library play candidate list。
- M2 实现完整分页/cursor。

### BUG-P2-002：directory LIKE 未 escape `%` / `_`

优先级：P2
所属里程碑：M1

现象：

- directory filter 使用 `tracks.relative_path LIKE ?3`。
- 若路径含 `%` 或 `_`，LIKE 语义会被放大。

影响：

- 不是 SQL 注入，但目录过滤可能错误。

修复方向：

- 增加 LIKE escape helper。
- SQL 使用 `LIKE ? ESCAPE '\\'`。

### BUG-P2-003：systemd unit 缺少 hardening

优先级：P2
所属里程碑：M1

现象：

- core services 缺少 `User=`, `NoNewPrivileges`, `ProtectSystem` 等。

影响：

- 正式镜像安全边界不足。

修复方向：

- M1 先记录 dev/release 差异。
- M3 完成正式 hardening。

## 6. 当前不纳入 M1 的事项

### V2 多解码器选择

- V1 只展示当前连接的 USB Audio 解码器，并在播放时自动选择当前唯一 USB Audio DAC。
- 多解码器列表、选择、持久化、udev sound event scan 放入 V2。
- 规划继续维护在 [Audio_Output_Device_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Audio_Output_Device_Plan.md)。

### Bridge Mode 真功能

- M1 只要求 mode manager 不污染 Local Mode。
- Bridge target 仍是占位。

### Android 主播放器

- Android APK 仍只作为 provisioning / WebView shell / diagnostics。
- 不进入主播放器或主曲库浏览职责。

## 7. 开发读取顺序

后续开发一个 M1 条目时，建议按这个顺序读：

1. 本文件对应 `REQ-*` / `BUG-*` 条目。
2. [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md) 对应章节。
3. 当前实现文件：
   - `services/rust/crates/playbackd/src/main.rs`
   - `services/rust/crates/sessiond/src/main.rs`
   - `services/rust/crates/ipc-proto/src/lib.rs`
   - `services/controld/internal/api/server.go`
   - `services/controld/internal/settings/config.go`
   - `base/rootfs/overlay/etc/systemd/system/`
4. 修改后把验证结果写回 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)。
