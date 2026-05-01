# WebUI API Contract Plan

## 1. 文档用途

本文件只负责维护 `V1` 阶段的 WebUI / API 解耦计划。

目标只有一个：

- 未来多次 WebUI 重构尽量只影响 `Web` 层，不影响底层功能实现

它不替代：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [WebUI_Design_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/WebUI_Design_Plan.md)

边界：

- 长期产品原则仍看产品手册
- 当前做到哪一步、这轮是否已验证，仍看开发日志

## 2. 当前真实状态

当前 `WebUI` 不是一个独立 frontend app。

当前真实结构是：

- `services/controld/internal/api/server.go`
  - route
  - SSR page data
  - 一部分页面专用 command wiring
- `services/controld/web/templates/*.html`
  - 页面模板
- `services/controld/web/static/*`
  - 静态资源
- `services/rust/*`
  - 真正的播放、队列、Quiet Mode、索引权威

这意味着：

- 视觉改动通常已经能只动模板和 CSS
- 但页面重排、功能入口迁移、整页重构，当前仍经常要碰 `controld` 的 page-specific wiring

一句话：

- 当前“核心功能实现”和“页面样式”已经基本分层
- 但“页面结构”和“浏览器消费的数据 contract”还没完全稳定

## 3. 这次改造要解决什么

这次不是做 SPA，也不是重写整套 WebUI。

这次要解决的是：

- 给浏览器补一层稳定的 `/api/v1/...` contract
- 让页面优先消费 domain-oriented JSON / SSE
- 让 UI 以后换布局、换按钮样式、跨页面挪功能时，不需要改 Rust 服务语义

目标场景包括：

- 播放按钮从文字改成图片
- 功能入口从 `Library` 挪到后续 `Settings`
- 按另一套播放器的视觉风格重做整个布局

## 4. 长期约束

- 不新增单独 frontend daemon
- 不新增第二个监听端口
- 继续由同一个 `controld` 进程监听 `:18080`
- 继续保持：
  - `SSR + 少量原生 JS`
- 不把队列逻辑、播放顺序、当前播放指针权威搬进 `controld`
- 不把页面布局细节写进 API 字段语义
- 不用高频轮询替代现有播放事件流

## 5. 分阶段计划

### 5.1 Phase 1：先补稳定 read-only contract

先做只读接口，不先改 command 语义。

第一批 endpoint：

- `GET /api/v1/system/summary`
- `GET /api/v1/system/health`
- `GET /api/v1/system/audio-output`
- `GET /api/v1/provisioning/status`
- `GET /api/v1/playback/status`
- `GET /api/v1/playback/queue`
- `GET /api/v1/playback/events`
- `GET /api/v1/library/snapshot`

这一阶段的目标是：

- 新 UI 可以只靠 API 取状态
- 旧 SSR 页面继续工作
- 不因为第一轮解耦就把现有控制链打断

当前进度：

- 第一批只读 endpoint 已落地
- 设置页当前已接入 `audio-output` 只读状态，用于显示 V1 当前解码器。
- 首页 `/` 已开始消费：
  - `system summary`
  - `provisioning status`
  - `playback status`
  - `playback queue`
  - `playback events`
- `Library` 页的动态区块也已开始消费：
  - `library snapshot`
  - `playback status`
  - `playback queue`
  - `playback events`
- `Library` 的列表本体现在也已开始优先按 `library snapshot` 做 browser-side render：
  - `Volumes`
  - `Folders`
  - `Albums`
  - `Tracks`
  - 当前仍保留 SSR first paint，JS 缺失或 API refresh 失败时继续可用
- `Library` 页里的 album / folder browse 当前也已开始优先走 browser-side navigation：
  - `pushState`
  - `popstate`
  - 按当前 URL query 重新请求 `library snapshot`
  - API 失败时再退回整页导航
- `Library` 页当前不再依赖 server 预拼 query-specific snapshot URL
  - server 只提供稳定 `/api/v1/library/snapshot` base path
  - query source 由浏览器当前 URL / history 决定
- 页面表单当前已开始优先走：
  - `POST /api/v1/playback/commands`
  - `POST /api/v1/library/commands`
- 旧 form action 主要保留为 no-JS fallback

### 5.2 Phase 2：收口 command contract

在只读 contract 稳定后，再把 command 路径也收成稳定 API。

目标方向：

- `POST /api/v1/playback/commands`
- `POST /api/v1/library/commands`

这阶段完成后：

- SSR form
- 后续新 UI

都应复用同一套 command contract，而不是继续绑定旧页面路径。

当前进度：

- `POST /api/v1/playback/commands` 已落地
- `POST /api/v1/library/commands` 已落地
- 旧：
  - `/commands`
  - `/library/commands`
  现在已复用同一套底层执行逻辑
- 首页与 `Library` 页当前已开始优先走新 command API
- 旧 form action 继续保留为 fallback

### 5.3 Phase 3：让页面逐步改成 API consumer

这阶段不要求一次性推倒重来。

按页面逐步迁移即可：

- 首页
- `Library`
- 后续 `Settings`

原则是：

- 页面只是 API consumer
- 页面布局怎么变，不应牵动 `playbackd / sessiond / media-indexd`

## 6. Phase 1 当前字段策略

Phase 1 优先输出 domain data，而不是页面 HTML 专用字段。

例如：

- 返回 `track_uid / album_uid / volume_uuid / relative_path`
- 返回 `order_mode / repeat_mode / current_order_index`
- 返回 `mode / interface_mode / dsd_policy`

而不是优先返回：

- 旧页面的按钮文案
- 旧页面的跳转 URL
- 旧模板专用的展示句子

## 7. 运行时开销约束

这次改造默认不新增：

- 新 daemon
- 新常驻监听线程
- 新独立 Web 服务

运行时变化主要只应是：

- 同一个 `controld` 内多几个 route / handler
- 浏览器多几个受控的 JSON 请求

实时播放状态继续优先复用：

- `SSE`

不要改成高频多接口 polling。

## 8. 当前验收标准

Phase 1 先以这些为准：

- `go test ./internal/api ./internal/provisioningclient ./internal/settings`
- 新 `/api/v1/...` endpoint 有最小测试覆盖
- 首页 `/` 已开始真实消费第一批只读 contract
- 旧 `/`、`/library`、`/healthz`、`/provisioning-status` 不回归
- 不改变 `playbackd`、`sessiond`、`media-indexd` 的现有职责边界

## 9. 当前文档关系

- 长期边界：
  - [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- 当前推进和待办：
  - [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- 新窗口快速交接：
  - [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
