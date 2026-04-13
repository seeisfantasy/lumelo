---
name: lumelo-doc-router
description: 在 Lumelo 仓库中先路由到现行主文档，再按任务类型跳到正确专项文档。适用于任何“先读什么文档”“当前阶段是什么”“哪份规则是权威”的任务。
---

## 触发时机
- 刚进入新窗口，需要快速接手
- 用户问当前主线、当前阶段、权威边界
- 用户要你改代码，但你还不知道先看哪份文档
- 用户引用了旧文档或过期结论，需要澄清

## 首选路由顺序
1. `docs/README.md`
2. `docs/AI_Handoff_Memory.md`
3. `docs/Development_Progress_Log.md`
4. `docs/Product_Development_Manual.md`
5. `docs/T4_Bringup_Checklist.md`

专项补充：
- 环境 / 出包 / runtime update：`docs/Development_Environment_README.md`
- 配网协议：`docs/Provisioning_Protocol.md`
- 无线金样：`docs/T4_WiFi_Golden_Baseline.md`
- 外部 AI 静态审查：`docs/review/`

## 文档边界
- 长期产品边界看：
  - `docs/Product_Development_Manual.md`
- 当前阶段、已验证事实、未闭环事项看：
  - `docs/AI_Handoff_Memory.md`
  - `docs/Development_Progress_Log.md`
- 真机 bring-up / 烧录后核查看：
  - `docs/T4_Bringup_Checklist.md`
- 外部 AI 静态审查入口看：
  - `docs/review/`
  - 但它只是 static snapshot，不是唯一真相
  - 若与仓库真实文件冲突，以仓库真实文件和主文档为准

## 使用要求
- 不要一上来整本通读 `Development_Progress_Log.md`
- 先定位当前阶段相关段落，再补历史上下文
- 若用户引用了旧结论，要明确指出：
  - 哪部分还有效
  - 哪部分已经过期
  - 现在应该以哪份文档为准

## 输出要求
- 明确告诉主 agent：
  - 哪份文档是长期边界
  - 哪份是当前进度
  - 哪份是操作清单
- 标出当前阶段真正未闭环的事项
- 标出用户引用内容是否已过期
