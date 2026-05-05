# Lumelo Docs Index

## Quick Paths

- 新窗口接手：
  - [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
  - [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- 长期产品边界、服务权威、状态机、API / 服务 contract：
  - [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- 里程碑版本规划、需求池和 bug 池：
  - [Milestone_Progress_Document.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Milestone_Progress_Document.md)
- 环境、出包、在线更新、T4 无线金样：
  - [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- T4 真机 bring-up、烧录后核查、真实媒体链验证：
  - [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
- 手机 APK、经典蓝牙配网、协议与安全传输：
  - [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
  - [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - [apps/android-provisioning/README.md](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/README.md)
- Win11 RKDevTool USB-to-eMMC 固件包：
  - [T4_USB_eMMC_Firmware_Requirements.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_USB_eMMC_Firmware_Requirements.md)

## Active Docs

- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
  - 当前交接入口、最新进展、未闭环事项
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
  - 产品原则、长期边界、服务权威、状态机、API / 服务 contract、验收矩阵
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
  - 时间线开发日志
- [Milestone_Progress_Document.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Milestone_Progress_Document.md)
  - 版本规划、M1/M2/M3 开发大纲、需求池与 bug 池
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 开发环境、出包、在线更新、T4 无线金样与操作约定
- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
  - 手机 APK 当前状态、结构、验收重点与后续计划
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - T4 与手机 APK 当前配网协议和传输契约
- [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
  - T4 真机 bring-up 与烧录后核查清单
- [T4_USB_eMMC_Firmware_Requirements.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_USB_eMMC_Firmware_Requirements.md)
  - Win11 RKDevTool USB-to-eMMC package 需求与验收

## Generated Docs

外部 AI 静态审查包不再提交到 git。

需要时运行：

```sh
python3 scripts/build-ai-review-docs.py
```

脚本会生成 `docs/review/`。该目录是 generated bundle，不是权威文档；若和主文档或仓库真实文件冲突，以主文档和仓库真实文件为准。

## Removed Old Docs

以下内容已合并或过期，不再保留单独文件：

- 旧 `docs/archive/` 历史提案、旧 MVP、一次性 checklist
- 旧 `WebUI_API_Contract_Plan.md` / `WebUI_Design_Plan.md`
  - 有效规则已合并进 [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- 旧 `Audio_Output_Device_Plan.md`
  - V1 / V2 DAC 语义已合并进 `Product_Development_Manual.md` 的 `Audio Output Contract`
- 旧 `T4_WiFi_Golden_Baseline.md`
  - 金样基线已合并进 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- 旧 `Handoff_20260419_WebUI_Player_First.md`
  - 一次性 session handoff 已由进度日志和产品手册覆盖

## Current Rule

- 顶层 `docs/` 只保留当前仍在使用的主文档。
- 一次性 handoff、旧提案和 generated review bundle 不长期留在仓库。
- 如果某条规则会影响后续开发，合并进 `Product_Development_Manual.md`、`Development_Environment_README.md` 或 `T4_Bringup_Checklist.md`，不要另开孤立文档。
