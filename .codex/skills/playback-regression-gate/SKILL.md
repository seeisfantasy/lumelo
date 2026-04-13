---
name: playback-regression-gate
description: 当改动涉及 playbackd、sessiond、media-indexd、lumelo-media-import、真实媒体链、Quiet Mode、坏文件、外部介质或重启恢复时，按 Lumelo 当前阶段的媒体回归方式组织验证。
---

## 触发时机
- 改动 `playbackd`
- 改动 `sessiond`
- 改动 `media-indexd`
- 改动 `lumelo-media-import`
- 改动队列 / 恢复 / history / `library.db`
- 改动 ALSA / 解码 / 真实曲库路径
- 改动外部 TF / USB 导入
- 改动坏文件处理
- 改动重启恢复

## 固定检查点
- `playbackd` 仍是播放状态和队列权威
- `sessiond` 只负责 Quiet Mode
- `controld` 不持有队列权威
- `media-indexd` 不在播放期间主动高频工作
- 外部媒体导入不应绕过 `volume` / `mount_path` / `is_available` 语义

## 最小 smoke 顺序
```sh
lumelo-media-smoke smoke --skip-play
lumelo-media-smoke list --first-wav
lumelo-media-smoke play --first-wav
```

## 按需继续的 canonical helper
```sh
lumelo-media-smoke regress-playback --timeout 8
lumelo-media-smoke regress-library-scan
lumelo-media-smoke regress-playbackd-restart --mount-root /var/lib/lumelo/test-media-tagged --timeout 8
lumelo-media-smoke regress-bad-media --timeout 8
lumelo-media-import list-mounted
lumelo-media-import scan-mounted
lumelo-media-import import-device <DEVICE>
lumelo-media-import reconcile-volumes
```

## 外部媒体验证要求
- 没有真 TF / USB 时，可以先用：
  - loop device / ISO
    做“模拟块设备导入”回归
- 有真介质时，必须再补：
  - 热插入
  - 挂载
  - 扫描入库
  - 点播
  - 拔出后下线

## 必须守住的语义
- `PLAYBACK_STARTED` 才表示第一帧真正写入 ALSA
- 输出链错误按 fail-stop
- 内容错误不能拖挂 `playbackd`
- 当前坏文件即使仍被索引，也必须是：
  - 播放失败可恢复
  - 服务不崩
  - 之后仍能播放有效轨道
- 重启后统一进入 `stopped`
- 不自动恢复播放

## 输出要求
- 明确写出：
  - 跑了哪些 helper
  - 是本地逻辑验证还是真机 ALSA 验证
  - 外部媒体是模拟块设备还是真 TF / USB
- 若坏文件仍被索引，要单独说明：
  - 这是当前现状
  - 还是这轮改动引入的新问题
