# Lumelo 仓库目录改名前清单

## 1. 改名目标

本轮改名前的仓库根目录位于：

`/Volumes/SeeDisk/Codex/NanoPC-T4 `

当前统一后的仓库根目录为：

`/Volumes/SeeDisk/Codex/Lumelo`

这次改名不只是品牌统一，也顺带消除了旧目录名末尾的尾随空格。

## 2. 当前已确认的影响面

### A. 仓库内硬编码路径

以下文件包含写死的旧仓库根路径，改名时必须同步修改：

- `scripts/orbstack-bootstrap-lumelo-dev.sh`
  - 默认 `REPO_HOST_PATH` 曾指向 `/Volumes/SeeDisk/Codex/NanoPC-T4 `

### B. 文档中的绝对路径链接

以下文档包含基于旧仓库根路径的绝对链接，仓库目录改名后需要统一替换：

- `docs/AI_Handoff_Memory.md`
- `docs/Development_Progress_Log.md`
- `docs/archive/T4_moode_port_blueprint.md`

这些链接主要指向：

- `docs/*.md`
- `docs/archive/*.md`
- `examples/sysctl/*.sample`
- `examples/systemd/*.sample`

### C. 文档中的状态说明

以下文档里明确写着“仓库目录名当前仍然是 `NanoPC-T4`”，改名完成后应更新结论：

- `docs/AI_Handoff_Memory.md`
- `docs/Development_Progress_Log.md`

## 3. 当前不应误改的内容

以下内容即使在仓库目录改名后，通常也不应该批量替换：

- `RK3399 / NanoPC-T4` 作为当前 `V1` 硬件平台的描述
- `NanoPC-T4` 作为板卡名、Wiki 名、bring-up 说明中的硬件名
- 与 `T4` 板级、设备树、USB/I2S、DAC 相关的硬件文档语义

也就是说，这次应改的是“仓库路径/项目目录名”，不是“硬件平台名”。

## 4. 仓库外需要同步检查的项目

这些项不一定在仓库文件里，但改名时很容易受影响：

- Codex 当前工作区路径和新窗口打开路径
- Finder / IDE / 编辑器工作区收藏
- OrbStack 里引用宿主机仓库路径的命令或临时脚本
- shell 历史、别名、手工保存过的本地命令

## 5. 推荐执行顺序

1. 停掉所有仍在使用旧仓库路径的长运行进程。
2. 将仓库目录从 `/Volumes/SeeDisk/Codex/NanoPC-T4 ` 改到 `/Volumes/SeeDisk/Codex/Lumelo`。
3. 重新用新路径打开 Codex 工作区。
4. 统一替换仓库内的旧绝对路径引用。
5. 更新 `orbstack-bootstrap-lumelo-dev.sh` 的默认 `REPO_HOST_PATH`。
6. 更新交接文档里“仓库目录仍是 `NanoPC-T4`”的旧结论。
7. 跑一轮最小校验，确认没有明显残留。

## 6. 改名后建议校验

建议至少做这几项检查：

- 搜索旧绝对路径是否还有残留
- 检查 OrbStack bootstrap 脚本是否仍指向旧路径
- 检查交接文档中的可点击链接是否还能打开
- 重新跑一轮：
  - `cargo test --manifest-path services/rust/Cargo.toml`
  - `GOCACHE=/tmp/lumelo-go-build-cache GOPATH=/tmp/lumelo-go go test ./...`

## 7. 当前结论

从当前仓库扫描结果看，真正需要修改的内容并不多，难度属于中等偏低。

最主要的风险不是“改不动”，而是：

- 漏改文档里的绝对路径链接
- 忘记改 OrbStack bootstrap 脚本默认路径
- 把硬件名 `NanoPC-T4` 误当成仓库名一起替换

因此，仓库目录改名是值得做的，但建议单开一轮、一次性收口。
