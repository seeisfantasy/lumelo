# Handoff 2026-04-19 WebUI Player First

## 1. 用途

这份文件只服务本轮 `WebUI / controld / live T4 runtime update` 交接。

长期边界和完整时间线仍以这些主文档为准：

- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [WebUI_API_Contract_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/WebUI_API_Contract_Plan.md)
- [WebUI_Design_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/WebUI_Design_Plan.md)

## 2. 今天已经落地的事实

- 最新 `WebUI` 已通过 `runtime update` 部署到 live `T4`
- 当前 live 板地址：
  - `192.168.1.110`
- 当前 live URL：
  - `http://192.168.1.110:18080/`
  - `http://192.168.1.110:18080/library`
  - `http://192.168.1.110:18080/provisioning`
  - `http://192.168.1.110:18080/logs`
- 当前 live `controld` 已是新版：
  - `Home` 会显示 `Quiet HiFi Console`
  - `Library` 会显示 `Library Console`
  - `Provisioning` 会显示 `Provisioning Monitor`
  - `Logs` 会显示 `Runtime Logs`

## 3. 用户已明确给出的反馈

用户已经明确否定了当前视觉方向的一个关键点：

- 现在这版虽然更统一了
- 但看起来仍然不像音乐播放器
- 更像：
  - dashboard
  - diagnostics console

所以后续不要继续沿着“control console / diagnostics shell”加料。

下一轮必须改成：

- `music player first, diagnostics second`

## 4. 下一轮必须遵守的边界

只做 `Web layer` 重构：

- `templates`
- `CSS`
- 浏览器侧 JS
- `controld` 的 page composition / stable API consumer

不要因为这轮视觉纠偏去改：

- `playbackd`
- `sessiond`
- `media-indexd`
- 播放语义

继续保持当前功能边界，不要凭空加：

- 进度条
- `radio`
- 歌词
- 全屏 `Now Playing`
- 尚不存在的 `Settings`

## 5. 下一轮应该怎么改

### `Home`

目标：

- 第一眼必须像进入音乐播放器

主视觉优先级：

- `Now Playing`
- artwork
- current track / artist / album hierarchy
- queue
- recent albums / recent listening 这类音乐入口

弱化到次级：

- defaults
- runtime paths
- diagnostics overview

### `Library`

目标：

- 更像 album wall + album detail

主视觉优先级：

- album art
- album / artist / year
- 当前专辑 tracklist
- 右侧 queue

保留但降级：

- folder / volume context
- debug / technical metadata
- 路径类信息

### `Provisioning / Logs`

保持：

- diagnostics page
- copy-friendly
- bring-up 可读性

不要再让这两页的气质反向定义整个播放器首页。

## 6. 关键文件

- [services/controld/internal/api/server.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/internal/api/server.go)
- [services/controld/internal/api/contracts.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/internal/api/contracts.go)
- [services/controld/internal/api/server_test.go](/Volumes/SeeDisk/Codex/Lumelo/services/controld/internal/api/server_test.go)
- [services/controld/web/static/css/app.css](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/static/css/app.css)
- [services/controld/web/templates/index.html](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/templates/index.html)
- [services/controld/web/templates/library.html](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/templates/library.html)
- [services/controld/web/templates/provisioning.html](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/templates/provisioning.html)
- [services/controld/web/templates/logs.html](/Volumes/SeeDisk/Codex/Lumelo/services/controld/web/templates/logs.html)

## 7. 今天已验证

- `go test ./internal/api ./internal/provisioningclient ./internal/settings`
- `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build ./cmd/controld`
- live `T4` 上 `controld.service = active`
- live `T4` 上 `curl` 已确认：
  - `/`
  - `/library`
  - `/provisioning`
  - `/logs`
  都已返回新版页面骨架

## 8. 今天还没验证

- live 浏览器里把新版页面完整点一遍
- 移动端观感
- 用户真正认可的 `music player` 视觉气质

## 9. 额外说明

- 今天为了本地验收起过一个临时 preview helper
- 这个 helper 和本地 preview server 已经删掉
- 工作树里不应再残留 `.tmp/ui_preview` 这类临时文件
