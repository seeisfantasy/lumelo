# Audio Output Device Plan

## 1. 文档用途

本文件维护 `Lumelo` 输出解码器 / DAC 的 WebUI 展示与后续选择能力。

边界：

- 长期播放原则看 [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- 当前进展看 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- WebUI / API 解耦看 [WebUI_API_Contract_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/WebUI_API_Contract_Plan.md)

## 2. V1 范围

V1 只做当前连接解码器展示，不做输出切换。

UI：

- 设置页显示一个 `当前解码器` 下拉框。
- 未发现 USB Audio 解码器时显示：
  - `未连接`
- 发现 USB Audio 解码器时显示：
  - 当前解码器名称

实现边界：

- `controld` 只读 `/proc/asound/cards`。
- 只把 `USB-Audio` ALSA card 当作 V1 解码器。
- 设置页渲染时读取一次。
- 不持续监听 USB 口。
- `playbackd` 在每次开始播放前读取当前 `/proc/asound/cards`。
- V1 自动选择当前唯一 USB Audio DAC：
  - 例如 `plughw:CARD=<detected>,DEV=0`
- 未发现 USB Audio DAC 时：
  - 播放命令必须 fail-fast，返回 `audio_output_unavailable`
  - 不允许先假装播放再自动变成 stopped
- 同时发现多个 USB Audio DAC 时：
  - V1 不做隐式选择
  - 播放命令必须 fail-fast，返回 `audio_output_ambiguous`
- 不做隐式 fallback。

当前只读 API：

- `GET /api/v1/system/audio-output`

## 3. V2 待讨论范围

V2 目标是支持多个 USB 解码器并允许用户选择输出设备。

目标场景：

- 用户通过 USB 同时连接 2 个解码器。
- 设置页下拉框列出：
  - 解码器 1
  - 解码器 2
- 用户选择其中一个作为播放输出。

需要先确认的产品语义：

- 选择是否必须在 `stopped` 状态下才允许。
- 播放中切换时是拒绝、停止后切换，还是先暂停再切换。
- 选择保存位置：
  - `/var/lib/lumelo/` 作为持久状态
  - 或 `/etc/lumelo/config.toml` 作为配置
- 解码器身份使用什么稳定 key：
  - USB vendor / product / serial
  - ALSA card id
  - sysfs path
- 断开当前选中解码器时 UI 如何提示：
  - 保留选择但标记不可用
  - 自动选择另一个
  - 进入 `未连接`

当前倾向：

- 当前选中解码器断开时进入 `未连接 / 输出不可用`。
- 不自动切到另一个解码器。
- 输出链错误继续按 fail-stop 处理。
- 用户重新选择后再恢复播放。

## 4. V2 事件监听方案

不要用高频轮询 USB 口。

推荐方向：

- 用 `udev` 只在 `sound` 设备 `add/remove` 时触发一次扫描。
- `udev` 触发一个 oneshot service：
  - `lumelo-audio-device-scan.service`
- oneshot helper 扫描当前 ALSA USB Audio card。
- helper 写入运行时状态：
  - `/run/lumelo/audio-output-status.json`
- `controld` 读取这个 runtime JSON。

浏览器更新方式待定：

- 页面打开时读取一次。
- 若需要页面停留时自动变化，优先用 SSE / WebSocket 订阅 runtime 状态变化。
- 不用浏览器高频轮询替代 USB 事件。

## 5. V2 待开发清单

- 新增 `lumelo-audio-device-scan` helper。
- 新增 `udev` rule，只在 `SUBSYSTEM=="sound"` 的 `add/remove` 触发。
- 新增 `lumelo-audio-device-scan.service` oneshot unit。
- 定义 `/run/lumelo/audio-output-status.json` schema。
- 扩展 `GET /api/v1/system/audio-output`：
  - 当前选中设备
  - 当前可用设备列表
  - 选中设备是否在线
  - 最近一次事件时间
- 新增输出选择 command API。
- 明确 `playbackd` 如何接收新的输出设备。
- 设置页下拉框启用选择。
- 增加单元测试：
  - 单解码器
  - 双解码器
  - 当前选中设备拔出
  - 无解码器
- 增加真机验证：
  - 单 USB DAC 插入 / 拔出
  - 双 USB DAC 插入 / 选择 / 拔出
  - 播放中拔出当前 DAC 时 fail-stop
