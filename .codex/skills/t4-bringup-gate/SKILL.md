---
name: t4-bringup-gate
description: 当改动涉及 rootfs、firmware、systemd、蓝牙、Wi-Fi、SSH、启动链、board bring-up 时，按 Lumelo 当前阶段的离线验收、runtime update 与真机 bring-up gate 组织验证。
---

## 触发时机
- 改动触及：
  - rootfs
  - image packaging
  - firmware / patch
  - `systemd`
  - Wi-Fi / 蓝牙 / SSH
  - 启动链
  - board bring-up
- 需要判断：
  - 这轮先做 runtime update 验证是否足够
  - 还是已经进入最终镜像 / 交付级 gate

## 先分流：runtime update 还是 final image
### 可以先走 runtime update 的场景
- 主要改动在：
  - overlay 下的 user-space 文件
  - helper 脚本
  - daemon 逻辑
  - service unit
- 当前目标是：
  - 快速真机验证
  - 缩短迭代周期

### 必须走 final image gate 的场景
- 涉及：
  - firmware / patch
  - boot / 启动链
  - kernel / dtb
  - 分区 / first-boot
  - 无线底座
  - 最终镜像交付

## 固定流程
1. 先结合现场现象分析 bug 和根因
2. 尽量合并多个待验点后统一验收
3. 若是开发期快速验证，优先考虑 runtime update
4. 若进入 final image / 高风险链路，出包前必须跑：
   ```sh
   ./scripts/verify-t4-lumelo-rootfs-image.sh <IMAGE>
   ```
5. 若涉及无线链路，再补：
   ```sh
   ./scripts/compare-t4-wireless-golden.sh \
     --board-base-image <BASE_IMAGE> \
     --image <IMAGE>
   ```
6. 只有 `0 failure(s), 0 warning(s)` 才允许进入上板建议
7. 上板后按 `docs/T4_Bringup_Checklist.md` 分层核查

## 真机检查重点
- `/healthz`
- `/provisioning-status`
- `/logs`
- `/logs.txt`
- `ssh root@<T4_IP>`
- `systemctl`
- `journalctl -b`
- `rfkill list`
- `bluetoothctl show`
- `hciconfig -a`

## 特别关注
- `hciattach.rk`
- `BCM4356A2.hcd`
- `bcmdhd.conf`
- `lumelo-bluetooth-uart-attach.service`
- SSH host keys 自动生成链
- FriendlyELEC 无线金样差异

## 实操边界
- `active` 不等于真的可用
- `advertising = true` 不等于手机一定能扫到
- `ssh enabled = true` 不等于 22 端口真的能用
- 若整机 `reboot` 后板子失联，优先怀疑：
  - 没有重新进入当前调试 `sd` 系统
  - 而不是先怪网络

## 禁止
- 只看服务 active 就宣布成功
- 没过离线验收就催用户上板
- 把手机问题当第一嫌疑，跳过板端日志
- 在 final image gate 未完成时，把开发期 runtime update 结果当成最终交付结论
