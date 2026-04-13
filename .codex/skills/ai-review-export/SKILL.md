---
name: ai-review-export
description: 当任务需要给外部 AI 或 reviewer 做大规模静态审查时，先导出 Lumelo 的 static review bundle，再基于 `docs/review/` 组织审查。
---

## 触发时机
- 用户要做大规模静态 code review
- 需要给外部 AI / reviewer 一份不依赖直接挂载仓库的工程快照
- 任务跨 Go / Rust / Android / rootfs / docs，需要统一静态入口

## 固定流程
1. 先运行：
   ```sh
   python3 scripts/build-ai-review-docs.py
   ```
2. 确认已经生成：
   - `docs/review/README.md`
   - `docs/review/AI_Review_File_Index.md`
   - `docs/review/AI_Review_Part_*.md`
3. 确认单个 review 文档大小不超过项目要求
4. 审查时优先从：
   - `docs/review/README.md`
   - `docs/review/AI_Review_File_Index.md`
     进入

## 使用原则
- 若源码或文档已变化，先 regenerate，再 review
- 不手工拼超长 code excerpt 代替完整 review bundle
- `docs/review/` 是 static snapshot，不是仓库唯一真相
- 若 review bundle 与仓库现状冲突，以仓库真实文件为准

## 输出要求
- 明确说明：
  - review bundle 是否刚重新生成
  - 入口文件位置
  - 共拆成多少 part
  - 是否有 binary / 非 UTF-8 文件被省略
