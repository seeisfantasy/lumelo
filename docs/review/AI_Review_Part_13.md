# AI Review Part 13

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `docs/Product_Development_Manual.md`

- bytes: 23608
- segment: 1/1

~~~md
# Lumelo 本地音频系统产品开发说明书

> 文档边界：
> - 本文件只维护产品原则、版本边界、服务权威、恢复语义和长期运行规则。
> - 环境、出包、在线更新和真机操作约定看 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)。
> - 每天真实发生的开发过程和阶段变化看 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)。
> - 手机 APK 当前状态、结构和后续计划看 [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)。
> - 经典蓝牙配网协议、安全传输和板端交互契约看 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)。
> - T4 板级无线金样与当前差异看 [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)。

## 1. 产品定义

`Lumelo` 是本产品的正式名称。

本系统当前基于 `RK3399 / NanoPC-T4` 平台构建，面向本地音乐播放，目标是成为一个极简音频系统。

它的核心目标不是做一个通用 Linux 发行版，也不是做一个大而全的网络音频平台，而是做一个：

- `本地播放优先`
- `headless`
- `极简可控`
- `播放时进入静默态`
- `mobile-first WebUI`
- `后续可扩展到 bridge 模式`

的专用音频 appliance。

## 2. 产品要解决的问题

本产品面向的不是普通桌面使用场景，而是以下需求：

- 开机后即可进入音乐控制主界面
- 不依赖联网即可播放本机 `TF 卡` 或 `USB 存储` 中的歌曲
- 播放时尽量减少系统噪声、后台服务和非必要线程活动
- 用远程页面进行控制，而不是依赖本地接屏显示
- 后续允许扩展为网络桥接设备，但不污染本地播放模式

## 3. 当前产品阶段

当前定义的正式版本是：

- `V1 = Local Mode`

`Bridge Mode` 仅保留模式接口和系统框架占位，不在 V1 中开发具体桥接功能。

## 4. 产品形态

### V1 形态

V1 是一个 `headless 本地播放系统`：

- 设备本身不需要本地接屏
- 主交互通过 `有线网络` 或 `Wi-Fi` 下的远程 WebUI 完成
- V1 的 WebUI 先以手机版基本功能完整为目标
- V1 不优先做桌面专用复杂布局和视觉美化
- 蓝牙在未来用于 `控制能力`，不用于 V1 的主要交互入口
- 本地播放时，系统进入 `Playback Quiet Mode`

### V2 预留形态

V2 才讨论并开发：

- `Bridge Mode`
- AirPlay
- UPnP
- 其他网络桥接能力
- 蓝牙 App 控制能力
- `Convenience Mode` 播放交互模式

## 5. 模式设计

系统采用互斥模式设计。

支持的模式枚举：

- `mode=local`
- `mode=bridge`

### 模式规则

- 新机器首次启动默认进入 `local`
- 后续每次开机默认进入上一次成功保存的模式
- 模式切换只允许在设置页中进行
- 模式切换不支持热切换
- 切换后必须立即重启才生效
- 如果用户取消重启，则本次切换不生效

### V1 对模式的实际实现

- `local`：完整实现
- `bridge`：仅保留模式壳和占位页面，不开发业务功能

## 6. 用户交互定义

### 主控制方式

V1 采用远程 WebUI 控制：

- 手机
- 平板
- 电脑浏览器

访问方式来自：

- 有线网络
- Wi-Fi

### 模式切换交互

设置页中提供一个模式选择控件：

- 下拉菜单
- 或两个互斥按钮

要求：

- 当前模式必须在界面上清楚显示
- 当前模式选项应高亮或处于选中状态

用户操作流程：

1. 用户修改模式
2. 点击“保存设置”
3. 系统弹窗提示“切换系统模式需要立即重启，取消则不保存此次切换”
4. 用户确认后，系统写入新模式并立即重启
5. 用户取消后，系统不保存新模式，界面恢复到当前模式

## 7. V1 功能范围

### 7.1 本地播放

V1 必须支持：

- 播放 `TF 卡` 中的本地音乐
- 播放 `USB 存储` 中的本地音乐
- 本地媒体浏览
- 本地媒体库
- 默认专辑封面平铺视图
- 文件夹视图兜底
- 基础搜索
- 播放 / 暂停
- 上一首 / 下一首
- 播放队列
- 顺序 / 随机播放
- 重复关闭 / 单曲循环 / 列表循环
- 基础设置页

本地媒体范围补充：

- 专辑聚合以 `album artist` 为主
- `ARTIST` 作为详情展示和搜索补充字段
- 专辑聚合采用保守合并策略
- 同专辑不同目录默认分成两张，不做激进合并
- 缺失 tag 时允许按目录回退聚合，并在 UI 中提示“目录聚合”
- 搜索包含目录名和文件名
- 大曲库依赖增强索引，但不污染播放内核
- 主索引库不存封面 blob 或缩略图 blob，只存资源引用
- 图片缓存属于可重建派生层，不属于内容真相源
- 介质离线后允许继续显示已缓存封面，但不可播放
- `shuffle` 采用固定 `play_order`
- `shuffle + repeat-all` 重复同一份 `play_order`

### 7.2 网络

V1 需要提供联网能力，但网络是外围能力，不是系统主核心。

V1 支持：

- 有线网络
- Wi-Fi

### 7.3 蓝牙

V1 只保留蓝牙控制方向的产品规划，不将其作为核心功能落地。

当前共识：

- 蓝牙未来更适合通过手机 App 进行控制
- V1 可使用 BLE 承担初次联网 / Wi-Fi provisioning 的 setup path
- 一旦完成联网，steady-state 主交互仍回到 Ethernet / Wi-Fi 下的 WebUI
- 蓝牙不作为 V1 的主要 WebUI 承载方式
- 蓝牙不参与 V1 的音频输入输出链路

### 7.4 手机 APK

V1 当前保留一个独立手机 APK，但它的角色需要明确限定。

当前定位：

- BLE / Wi-Fi provisioning 工具
- 首次联网 setup path
- 联网成功后的 APK 内 `WebView` 外壳
- 板端 BLE / 配网异常时的诊断入口

当前不是：

- 主播放器 App
- 主曲库浏览 App
- steady-state 主控制端

V1 的 steady-state 主交互仍然是：

- Ethernet / Wi-Fi 下的 WebUI

当前 APK 的功能结构按产品层划分为：

- `Setup Shell`
  - 权限、环境状态、输入与主流程按钮
- `BLE 扫描层`
  - Lumelo 扫描、通用 BLE 扫描、自检入口
- `GATT 会话层`
  - 连接、MTU、服务发现、读写、通知
- `Provisioning 流程层`
  - Wi-Fi 凭据发送、apply、状态推进
- `APK 内 WebView 壳层`
  - 联网成功后的 Home / Library / Provisioning / Logs 入口

更细的结构、当前进度和后续分阶段计划统一维护在：

- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)

## 8. 单接口运行原则

系统设计为单接口运行。

即任一时刻只启用一种连接模式：

- `Ethernet`
- `Wi-Fi`
- `Bluetooth Control`（未来）

### 设计原则

- 同一时间不同时监听多种控制接口
- 修改连接模式后重启生效
- 未被选中的接口和相关服务应关闭
- 尽量减少扫描、重连、守护进程和无关中断

### 当前版本范围

V1 重点实现：

- Ethernet
- Wi-Fi

Bluetooth Control 为后续能力。

## 9. Playback Quiet Mode

`Playback Quiet Mode` 是本系统的一等公民，不是附属优化项。

### 设计目标

在本地播放期间，尽量减少所有与播放无关的系统活动，使系统进入尽可能纯净、安静、可预测的运行状态。

### 可以做的事情

播放时可执行以下动作：

- 停止媒体扫描
- 停止封面抓取
- 停止媒体库重建
- 停止缩略图处理
- 降低或停止非必要日志写入
- 禁止不必要的 WebUI 高频轮询
- 禁止高频进度刷新
- 让后台非关键服务进入休眠、冻结、降权或不启动状态
- 将关键播放路径与外围服务分离

### 产品层面的表现

- 播放时不要求可拖动进度条
- 播放时可以只保留极简控制
- 播放中允许只监听：
  - `track start`
  - `track end`
  - 用户显式控制命令

### 出错时的产品行为

- 输出链错误优先直接停止，并明确提示原因
- 不做隐式输出路径回退
- 对“当前文件损坏 / 读不到”这类内容错误，只允许一次性的 6 秒后自动切到下一首
- 该等待由播放核心内部维护，界面只做静态提示，不做高频倒计时刷新

### 边界说明

Linux 系统不可能做到绝对“零后台线程”，仍会存在：

- 内核线程
- IRQ
- 定时器
- 必要系统管理线程

本系统的目标不是假设完全归零，而是将非关键活动压到最低。

## 10. 播放策略

V1 的本地播放以“纯净播放优先”为原则。

### 建议方向

- 优先考虑 `RAM Playback` 或强缓冲策略
- 播放时尽量避免持续读取本地存储
- 将播放路径与管理路径分离
- 不提供数字音量
- 不提供软件音量

### 播放控制取向

V1 不追求复杂可视化和高频动态反馈。

更重要的是：

- 稳定
- 可预测
- 对音频播放路径干扰小

## 11. 系统特点

本产品的主要特点如下：

- 本地播放优先
- 极简 headless 架构
- 远程 WebUI 控制
- 互斥模式设计
- 单接口运行原则
- Playback Quiet Mode 为一等公民
- Bridge 功能与 Local 功能在架构上严格隔离

## 11.1 本产品相对于通用播放器方案的核心优势

本产品不是传统意义上的：

- `播放器前端 + MPD 后端`

也不是以通用 Linux 音乐播放系统为目标的方案。

本产品的核心优势在于，它从一开始就按“自定义 transport 内核”的思路设计，并围绕以下非标准但高价值的能力展开：

- 点击 `play` 后立即进入静默准备，并在第一帧音频写入 ALSA 后进入正式 `Playback Quiet Mode`
- 使用 `prev/current/next` 三曲 `RAM Window Playback`

这两点是本产品的重要差异化能力，也是相对于 `moOde` 一类通用播放器方案的关键优势。

它们带来的价值包括：

- 播放期间更少后台干扰
- 对本地存储的持续访问更少
- 切歌行为更可控
- 播放状态和缓存策略由产品自身掌控，而不是依赖通用播放器守护进程的默认行为

## 12. 系统由什么搭建

当前技术方向如下：

### 硬件平台

- `NanoPC-T4`
- `RK3399`

### 底座方向

采用 `RK3399/T4 板级稳定底座 + 自定义极简 rootfs + 音频层 overlay` 的路线。

不是直接硬改 Raspberry Pi 发行版。

### 板级线与 rootfs 的关系

本项目不将“整套现成发行版镜像”作为长期产品底座。

V1 的正式路线采用解耦设计：

- `rootfs` 由项目自行定义和维护
- `kernel / dtb / u-boot` 初期优先采用 FriendlyELEC 的稳定 T4 板级线
- 上层音频功能与板级镜像发行版解耦

这意味着：

- 项目不是在选择某个完整镜像作为最终产品
- 项目真正选择的是：
  - 哪条 `kernel / dtb / u-boot` 板级线
  - 哪套自定义 `rootfs`

这样设计的目的在于：

- 先利用 FriendlyELEC 的 T4 板级稳定性完成 bring-up
- 同时保留项目对 rootfs、服务和音频层的完全控制
- 后续允许在不推翻上层系统的情况下迁移到更主线的 `kernel / dtb` 方案

### 推荐的软件分层

- `common base`
- `mode manager`
- `media`
- `playback`
- `session`
- `control`
- `connectivity`

### 模块说明

#### common base

负责：

- rootfs
- systemd
- 启动链
- 板级支持
- 基础网络与系统配置

#### mode manager

负责：

- 持久模式读取
- 首次启动默认模式
- 按模式选择启动目标

#### media

负责：

- TF 卡和 USB 存储挂载
- 本地媒体发现
- 本地索引

#### playback

负责：

- 播放引擎
- ALSA 路径
- RAM Playback 或缓冲策略
- 本地播放状态机

#### session

负责：

- Playback Quiet Mode
- 播放开始时切换系统状态
- 播放结束后恢复系统状态

#### control

负责：

- 远程 WebUI
- 基础设置页
- 控制接口

#### connectivity

负责：

- Ethernet
- Wi-Fi
- 未来 Bluetooth Control

### 升级与维护原则

- V1 不做在线自动更新
- V1 不启用后台自动检查更新
- V1 与后续版本都应长期保留两种明确的人工触发维护路径：
  - 在线更新
  - 整包重刷
- 在线更新用于：
  - 用户态服务
  - Web 资源
  - 配置模板
  - 其他不需要改动 boot 链、分区布局、内核 / DTB 的版本更新
- 整包重刷用于：
  - bootloader / kernel / DTB 变更
  - 分区布局调整
  - 底座损坏恢复
  - 在线更新失败后的兜底恢复
- 在线更新必须是显式人工触发：
  - 不做后台静默安装
  - 不做无人值守自动重启
- 整包重刷应继续保留“保留数据重刷”和“清空重刷 / 恢复出厂”的设计空间
- 配置与关键运行数据应在升级时优先保留
- `library.db` 如遇不兼容，可重建而不强行维护复杂迁移链

### 安全边界原则

- WebUI 面向局域网使用，不以公网暴露为目标
- V1 提供基础登录能力
- V1 采用单管理员密码模型，不提供用户名
- 首次启动必须先设置管理密码，不提供跳过入口
- 忘记密码时，通过物理恢复介质重置，并重新进入首次设置
- V1 不做多用户和复杂角色系统
- 正式发布镜像中 SSH 默认关闭
- 正式发布镜像里，SSH 只在设置中显式开启，用于 PC 调试
- 开发 / bring-up 镜像可默认开启 SSH，以便板级调试和命令行排障

### 启动与服务编排原则

- `playbackd`、`sessiond`、`controld` 为长期常驻核心服务
- `media-indexd` 为按需启动的索引 worker
- 启动顺序优先保证播放核心，再挂控制层
- `playbackd` 不依赖网络启动
- `controld` 崩溃不应影响 `playbackd`

## 13. V1 与 V2 的边界

### V1

- 只做 `local mode`
- `bridge mode` 仅保留占位
- 实现本地播放、远程 WebUI、网络配置、Playback Quiet Mode

### V2

再讨论：

- Bridge 功能
- AirPlay
- UPnP
- 其他网桥服务
- 蓝牙 App 控制
- `Convenience Mode`

### V2 对播放交互模式的预留

V2 预留两种播放交互模式：

- `Pure Mode`
- `Convenience Mode`

规则：

- 默认模式始终为 `Pure Mode`
- `Pure Mode` 不显示进度条，不支持 seek
- `Convenience Mode` 才允许进度条与后续 seek 能力
- 该开关属于控制层 / UI 层设置
- 该开关不改变音频输出策略
- 该开关预计立即生效，不需要重启

设计原则：

- `Pure Mode` 是产品默认人格，不会被 `Convenience Mode` 取代
- `Convenience Mode` 只增加交互便利性，不改变 transport 的底层纯净输出路径

## 14. 当前产品原则总结

这不是一个“功能尽量多”的系统，而是一个：

- 只做必要功能
- 严格控制后台活动
- 明确区分产品模式
- 明确区分当前版本与未来扩展

的本地音频系统。

V1 的目标不是全能，而是把 `local mode` 做稳、做干净、做安静。

## 15. 工程组织原则

V1 的实现建议遵循以下工程组织方式：

- 采用单仓 `monorepo`
- `base`、`services`、`docs`、`packaging` 明确分层
- Rust 侧集中实现：
  - `playbackd`
  - `sessiond`
  - `media-indexd`
- Go 侧集中实现：
  - `controld`
- WebUI 保持 `SSR + 少量原生 JS`
- 不引入 SPA 构建链
- 页面模板和静态资源可直接打包进 `controld` 二进制
- 运行时临时对象统一放入 `/run/lumelo/`
- 持久化状态统一放入 `/var/lib/lumelo/`

设计目标：

- 保持构建链简单
- 保持播放核心与 UI 层解耦
- 保持运行时状态边界清晰
- 方便后续镜像打包、恢复和交接

## 16. 当前推荐开发环境

当前活跃开发环境仍是：

- `macOS 主机 + OrbStack Linux arm64 + NanoPC-T4 真机 + Android 真机`

但从本轮起，产品手册不再展开维护：

- 宿主机文件系统与工作区方案
- 虚拟机与 SDK 版本
- 出包脚本、在线更新与真机操作细节

这些内容统一转到：

- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)

后续原则只有一条：

- 产品手册只保留长期边界，操作性细节不再在这里重复维护

## 17. 服务边界与状态权威

V1 的稳定服务边界如下：

- `playbackd` 是播放状态、队列状态和切歌逻辑的唯一权威
- `sessiond` 是独立的薄服务，只负责 `Playback Quiet Mode` 与系统环境切换
- `controld` 负责 WebUI、API、设置和认证，不作为队列逻辑权威
- `media-indexd` 是按需启动的索引 worker，不作为播放期高活跃常驻服务

### `playbackd`

负责：

- 播放状态机
- `prev/current/next` 与 `RAM Window Playback`
- 队列与 `play_order`
- 历史写入
- 切歌与错误分类
- 对外命令 socket 与事件 socket

### `sessiond`

负责：

- 订阅 `playbackd` 事件
- 维护 Quiet Mode 状态
- freeze / unfreeze 非关键服务
- 维护 `/run/lumelo/quiet_mode`
- 将 Quiet Mode 状态变化传递给 `controld`

边界：

- 不参与音频处理
- 不参与 RAM Window 管理
- 不参与播放决策
- 不要求 `playbackd` 等待它反馈

### `controld`

负责：

- WebUI 页面
- 控制命令转发
- 设置读写
- 登录、会话与认证

边界：

- 不自行计算顺序播放逻辑
- 不自行重建 `shuffle` 顺序
- 不自行更新当前播放指针
- 不持有队列状态权威

### `media-indexd`

负责：

- 本地曲库索引
- 标签解析
- 封面发现与缩略图派生层构建

边界：

- 不在播放期间主动工作
- 不感知 `playbackd` 内部状态
- 主要通过 `library.db` 与控制层共享结果

### 事件与 Quiet Mode 约定

- `playbackd -> sessiond` 采用单向事件依赖
- 事件传输采用独立 UDS 事件 socket
- 事件采用 fire-and-forget，慢订阅者直接断开

V1 关键事件：

- `PLAY_REQUEST_ACCEPTED`
- `PLAYBACK_STARTED`
- `PLAYBACK_PAUSED`
- `PLAYBACK_RESUMED`
- `PLAYBACK_STOPPED`
- `TRACK_CHANGED`
- `PLAYBACK_FAILED`

Quiet Mode 关键语义：

- `PLAY_REQUEST_ACCEPTED` 进入 `pre-quiet`
- `PLAYBACK_STARTED` 表示第一帧真正写入 ALSA，进入正式 Quiet Mode
- `TRACK_CHANGED` 且仍处于播放态时，Quiet Mode 保持不变
- `PLAYBACK_STOPPED` 退出 Quiet Mode
- `PLAYBACK_FAILED` 必须恢复到非静默态

## 18. 配置、队列与恢复边界

### 配置系统

- 静态配置与运行时状态分离
- 当前配置与默认配置分离
- 推荐路径为：
  - `/etc/lumelo/config.toml`
  - `/usr/share/lumelo/default_config.toml`
- 配置解析失败时自动回滚到默认配置
- 回滚后必须在 UI 中明确告警

立即生效的设置：

- WebUI 皮肤切换
- Quiet Mode 的非核心行为微调

必须重启生效的设置：

- `Local / Bridge` 模式切换
- 网络接口模式切换
- DSD 输出策略切换

### 队列与播放顺序

- V1 的播放模式拆分为：
  - `order_mode = sequential | shuffle`
  - `repeat_mode = off | one | all`
- `play_order` 是实际播放顺序
- 当前曲目由 `play_order + current_order_index` 推导
- `shuffle + repeat_mode=all` 重复同一份 `play_order`
- 只有队列增删或用户重新开关随机时，才重建随机顺序
- `shuffle` 下不允许手动调序
- 若需手动调序，用户需先切回 `sequential`
- 队列项应区分：
  - `track_uid`
  - `queue_entry_id`

### 持久化边界

V1 保留的稳定持久化对象：

- `/etc/lumelo/config.toml`
- `/var/lib/lumelo/queue.json`
- `/var/lib/lumelo/history.json`
- `/var/lib/lumelo/library.db`

V1 明确不持久化：

- `prev / current / next` 的 RAM 内容
- Quiet Mode 状态
- 跨重启秒级播放进度
- 独立 `session.json`

### 重启恢复语义

- `queue.json` 是唯一队列恢复入口
- 重启后只恢复：
  - 队列内容
  - `play_order`
  - `current_order_index`
  - `order_mode`
  - `repeat_mode`
- 系统启动后统一进入 `stopped`
- 不自动恢复播放
- 不自动恢复暂停点

### 历史记录

- 历史记录采用轻量 JSON
- 历史记录只保留最近 `100` 首
- 历史记录由 `playbackd` 单写
- 历史记录采用原子写入
- 历史记录文件保留 `version` 与 `updated_at`

## 19. 错误处理与恢复原则

### 输出链错误

下列错误按 `fail-stop` 处理：

- DAC 不可用
- DAC 被拔掉
- ALSA 打不开
- `Strict Native` 不支持

处理原则：

- 立即停止当前输出
- 发出 `PLAYBACK_FAILED`
- 退出 Quiet Mode
- UI 明确提示错误原因
- 不自动切到 `DoP`
- 不自动切到 PCM
- 不自动切下一首

### 内容错误

当“当前文件损坏 / 读不到”时：

- 进入 `quiet_error_hold`
- UI 静态提示错误原因
- UI 显示“6 秒之后切换到下一首”
- 不做可视化倒计时刷新
- 6 秒等待由 `playbackd` 内部维护
- 6 秒后自动切到下一首可播放曲目
- 连续自动跳过上限为 `3`
- 任一用户显式操作都会取消该次自动切歌等待

失败项标记原则：

- 只在本次运行期存在
- 不写入 `queue.json`
- 不写入 `history.json`
- 不写入 `library.db`
- 自动遍历时跳过失败项
- 用户显式点击失败曲目时，允许手动重试一次
- `repeat_mode=one` 遇到内容错误时，不重复尝试同一失败项

### 介质离线

- TF / USB 被拔掉时，UI 必须明确提示介质不可访问
- 若当前曲目或下一曲已完整驻留于 RAM，可由 `playbackd` 决定继续播放
- 不默认自动切下一首
- 不自动重扫
- 不自动重建队列

## 20. 启动编排与运行路径

### 启动顺序

- `local-mode.target` 为 V1 主目标
- `bridge-mode.target` 在 V1 仅作占位
- `playback-quiet` 不强制独立 target

建议顺序：

1. 基础系统、挂载和必要设备
2. `auth-recovery.service`
3. 当前启用接口所需的基础网络服务
4. `playbackd`
5. `sessiond`
6. `controld`
7. `media-indexd` 按需启动

补充约束：

- `auth-recovery.service` 只在启动阶段检查一次，并在 `controld` 之前完成
- `playbackd` 不等待网络
- `sessiond` 依赖 `playbackd`，不依赖 `controld`
- `controld` 崩溃不应影响 `playbackd`
- `media-indexd` 崩溃不应影响当前播放

### 运行路径

- 运行时临时对象统一放入 `/run/lumelo/`
- 持久化状态统一放入 `/var/lib/lumelo/`
- `/run/lumelo/` 只放 socket 与轻量运行时标志
- 不在 `/run/lumelo/` 中引入每卷运行时状态文件

## 21. T4 Bring-up 稳定约束

下面这些约束已经从最近几轮真机 bring-up 中收口为长期原则。

- `NanoPC-T4` 的开发 / bring-up 图必须保留板级蓝牙 `UART attach` 链，不能假设“只有 `bluez + bluetoothd` 就足够”。
- 若官方 FriendlyELEC 底图对蓝牙 bring-up 依赖专用 helper 或等价链路，Lumelo 开发图必须保留同等能力。
- 开发 / bring-up 图默认应表现为保守的 appliance DHCP client；在没有明确产品需求前，不默认开启 `LinkLocalAddressing`、`LLMNR`、`MulticastDNS`。
- Wi-Fi 配置应用流程优先重配置目标无线接口，不优先重启整套网络栈。
- 开发图必须支持无头排障：SSH host keys 需要可自动补齐，Web 侧默认保留 `/healthz`、`/provisioning-status`、`/logs`、`/logs.txt`。

更细的现场核查步骤、无线金样差异和配网协议细节，统一分别维护在：

- [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
- [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
~~~

## `docs/Provisioning_Protocol.md`

- bytes: 5718
- segment: 1/1

~~~md
# Provisioning Protocol

This document defines the current Bluetooth / Wi-Fi provisioning protocol between
the T4 and the Android provisioning app.

Historical product-scope notes for the first APK MVP are archived in
[archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md).

## Current T4-Side Foundation

The next rebuilt image is expected to include:

- `bluez`
- `wpasupplicant`
- `iw`
- `rfkill`
- `wireless-regdb`
- `/usr/bin/lumelo-bluetooth-provisioning-mode`
- `/usr/bin/lumelo-wifi-apply`
- `/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond`
- `/etc/systemd/network/30-wireless-dhcp.network`

This foundation is enough to validate whether the NanoPC-T4 exposes a usable
Bluetooth controller and Wi-Fi interface under the Lumelo rootfs.

## Provisioning User Flow

1. T4 boots without known Wi-Fi credentials.
2. T4 starts Bluetooth provisioning mode.
3. Phone app discovers `Lumelo T4`.
4. User pairs or connects over classic Bluetooth.
5. Phone app sends SSID and password to the T4.
6. T4 writes credentials and restarts Wi-Fi.
7. Phone app shows success once the T4 has an IP address.
8. User opens the normal Lumelo WebUI on Wi-Fi.

## Transport Decision

当前 bring-up 已经确认：

- 经典蓝牙在接好天线后可被手机系统蓝牙设置页发现
- BLE 广播在当前 `T4` 板上仍不稳定

因此配网主通道调整为：

- 经典蓝牙 `RFCOMM / SPP` 作为主传输层
- `Raw BLE Scan` 保留为诊断工具，不再作为主配网发现路径

## Provisioning Message Shape

经典蓝牙主通道不再依赖 GATT characteristic。

但上层业务语义继续沿用当前定义：

- `device_info`: JSON with hostname, build id, and current IP state
- `wifi_credentials_encrypted`: encrypted credential payload carrying the same `ssid/password` semantics
- `apply`: trigger that asks the T4 to apply the last credentials
- `status`: JSON with `advertising`, `credentials_ready`, `applying`, `waiting_for_ip`, `connected`, or `failed`

首版经典蓝牙协议采用逐行 JSON：

- `{"type":"device_info"}`
- `{"type":"wifi_credentials_encrypted","payload":{...}}`
- `{"type":"apply"}`
- `{"type":"status"}`

当前协议已经扩展为协商式安全传输：

- `hello` 现在会额外携带 `security` 字段
- 当前实现中：
  - App 只发送 `wifi_credentials_encrypted`
- 当前实现采用：
  - `scheme = dh-hmac-sha256-stream-v1`
  - `dh_group = modp14-sha256`
- 板端会在 `hello.security` 中提供：
  - `session_id`
  - `server_nonce`
  - `server_public_key`
- 手机端在发送加密凭据时提供：
  - `client_public_key`
  - `client_nonce`
  - `message_nonce`
  - `ciphertext`
  - `mac`

设计边界：

- 这一轮先解决“蓝牙传输链路不再明文暴露 Wi-Fi 密码”
- 板端“非明文持久化存储”不在当前改动范围内，后续在固件改造时一并处理
- 板端当前会拒绝旧的明文 `wifi_credentials` 命令，并返回：
  - `code = plaintext_credentials_disabled`

板端响应：

- `{"type":"device_info","payload":{...}}`
- `{"type":"status","payload":{...}}`
- `{"type":"ack","message":"..."}`
- `{"type":"error","message":"...","code":"..."}`

The first implementation should accept only WPA-PSK credentials. Open networks
and enterprise Wi-Fi can stay out of scope.

## T4 Implementation Notes

当前主实现调整为经典蓝牙 RFCOMM 服务：

- `/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond`

它负责：

- 让 `T4` 进入经典蓝牙 discoverable / pairable 模式
- 通过 `sdptool` 注册 `SPP` 服务
- 接受手机端 RFCOMM JSON 指令
- 调用 `/usr/bin/lumelo-wifi-apply`
- 持续输出 `/run/lumelo/provisioning-status.json`

The current bring-up iteration also writes the latest status snapshot to
`/run/lumelo/provisioning-status.json` so `controld`, SSH, and the T4 report
script can all inspect the same runtime state.

That snapshot should now also carry:

- `error_code`
- `apply_output`
- `diagnostic_hint`
- `wpa_unit`
- `ip_wait_seconds`

`/usr/bin/lumelo-bluetooth-provisioning-mode` 仍负责在服务启动前把控制器拉到
经典蓝牙可发现 / 可连接状态。

`/usr/bin/lumelo-wifi-apply` should no longer assume `wlan0`; it should prefer
`LUMELO_WIFI_IFACE`, then `WIFI_INTERFACE`, then auto-detect the first wireless
interface via `iw dev` or `/sys/class/net/*/wireless`.

## App Role

Start with Android only unless iOS becomes a hard requirement. The app should
only do:

- scan for `Lumelo T4` over classic Bluetooth
- connect/pair over classic Bluetooth
- send SSID/password
- prefill the current phone Wi-Fi SSID when available
- show connection result
- show the WebUI URL after success
- automatically enter the APK-hosted main interface after `connected`
- allow manual status refresh and disconnect during bring-up
- automatically poll status for a short window after apply
- expose the board-side `/provisioning`, `/logs`, and `/healthz` pages once an IP is known

`Raw BLE Scan` 作为诊断能力保留，用来判断板子是否还有 BLE 广播，但它不再
承担主配网职责。

The WebUI home page should also expose a compact provisioning summary so the
operator can see the latest Bluetooth / Wi-Fi state without leaving `/`.

The log page remains part of the WebUI, not the mobile provisioning app.

The first Android-only MVP scope is archived in
[archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md).

The current APK structure, status, and follow-up roadmap are maintained in
[Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md).
~~~

## `docs/README.md`

- bytes: 3604
- segment: 1/1

~~~md
# Lumelo Docs Index

## Quick Paths

- 新窗口进入状态：
  - 先看 [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
  - 再看 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- 环境、出包、在线更新：
  - 看 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- 手机 APK、经典蓝牙配网、协议与安全传输：
  - 看 [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
  - 看 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - 看 [apps/android-provisioning/README.md](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning/README.md)
- 给外部 AI 做静态工程审查：
  - 看 `docs/review/`
- T4 真机 bring-up、无线金样、板级排障：
  - 看 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
  - 看 [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)
- 历史提案、旧 MVP、阶段性问题清单：
  - 看 `docs/archive/`

## Active Docs

- [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md)
  - 当前交接入口、最新进展、未闭环事项
- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
  - 产品原则、长期边界、升级维护原则
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
  - 时间线开发日志
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
  - 开发环境、出包、在线更新与操作约定
- [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)
  - 手机 APK 当前状态、结构、验收重点与后续计划
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)
  - T4 与手机 APK 当前配网协议和传输契约
- [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
  - T4 真机 bring-up 与烧录后核查清单
- [T4_WiFi_Golden_Baseline.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_WiFi_Golden_Baseline.md)
  - T4 无线金样与 Lumelo 当前差异
- `docs/review/`
  - 给外部 AI 使用的静态审查文档包

## Archived Docs

- [archive/Real_Device_Findings_20260412_v15.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Real_Device_Findings_20260412_v15.md)
  - `v15` 阶段性真机问题原始清单
- [archive/Repo_Rename_To_Lumelo_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Repo_Rename_To_Lumelo_Checklist.md)
  - 仓库改名历史清单
- [archive/T4_moode_port_blueprint.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/T4_moode_port_blueprint.md)
  - 历史参考蓝图
- [archive/Android_Provisioning_App_MVP.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/Android_Provisioning_App_MVP.md)
  - APK 初版 MVP 目标定义，现已由进度文档取代
- [archive/V1_Local_Mode_Function_and_Service_Spec.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/V1_Local_Mode_Function_and_Service_Spec.md)
  - V1 功能规格历史稿
- [archive/V1_Technical_Architecture_Proposal.md](/Volumes/SeeDisk/Codex/Lumelo/docs/archive/V1_Technical_Architecture_Proposal.md)
  - V1 技术方案历史稿

## Current Rule

- 顶层 `docs/` 只保留当前仍在使用的主文档
- 阶段性清单、旧提案、历史 MVP 与一次性 checklist 进入 `docs/archive/`
~~~

## `docs/T4_Bringup_Checklist.md`

- bytes: 13635
- segment: 1/1

~~~md
# NanoPC-T4 Bring-up Checklist

## 1. 文档用途

本文件是 `NanoPC-T4` 真机 bring-up 的最小核查清单。

用途不是替代开发日志，而是把“每次烧录后都该核什么”固定下来，避免再次出现：

- 镜像离线验收通过
- 但板级无线 firmware / patch 仍缺失
- 或 SSH / 蓝牙只是上层看起来正常，底层其实没真正起来

## 2. 适用时机

以下任一情况，烧录后都应跑完本清单：

- 新出一张 `lumelo-t4-rootfs-YYYYMMDD-vN.img`
- 改了蓝牙、Wi-Fi、SSH、`systemd`、board bring-up、firmware、启动链
- 现场再次出现“`/healthz` 看起来正常，但手机扫不到 / SSH 进不去 / Wi-Fi 不稳”

## 3. 官方依据

本清单不是凭经验拼出来的，主要依据以下官方资料整理：

- [FriendlyELEC NanoPC-T4 产品页](https://www.friendlyelec.com/index.php?product_id=225&route=product%2Fproduct)
  - 明确板子是 `Bluetooth 4.1`
- [FriendlyELEC NanoPC-T4 Wiki](https://wiki.friendlyelec.com/wiki/index.php/NanoPC-T4)
  - 明确官方系统支持蓝牙
  - 明确曾有 `Bluetooth BLE enabled`
  - 明确曾修复过 Bluetooth firmware 加载问题
- [Murata Wi-Fi/BT Linux Quick Start Guide](https://www.murata.com/-/media/webrenewal/products/connectivitymodule/asset/pub/rfm/data/murata_quick_start_guide_linux.ashx?cvid=20210615064818000000&la=en)
  - 明确 Wi-Fi firmware、NVRAM 与 `/etc/firmware/*.hcd` 是 bring-up 必需文件
  - 明确蓝牙验证路径是：
    - `hciattach`
    - `hciconfig hci0 up`
    - `hcitool scan`
- [BlueZ hciattach_bcm43xx.c](https://sources.debian.org/src/bluez/5.43-2%2Bdeb9u2/tools/hciattach_bcm43xx.c)
  - 明确 Broadcom 蓝牙 patch 会从 `/etc/firmware/` 查找

## 4. 核查原则

- 先过离线镜像验收，再上板
- 上板后同时看：
  - 上层状态
  - 板级日志
  - 真实接口可用性
- 不接受“服务说自己起来了”作为唯一成功标准

一句话说：

- `advertising = true` 不等于手机一定能扫到
- `ssh enabled = true` 不等于 `22` 端口一定真的能进

当前 `NanoPC-T4` 默认以 FriendlyELEC 官方运行态为无线金样：

- Wi-Fi 驱动走 `bcmdhd`
- 蓝牙控制器挂在 `ttyS0`
- 蓝牙 patch 文件是 `/etc/firmware/BCM4356A2.hcd`
- 模块策略文件是 `/etc/modprobe.d/bcmdhd.conf`
- 蓝牙 attach 先等 `/sys/module/bcmdhd`，再执行 `hciattach.rk /dev/ttyS0 bcm43xx 1500000`

## 5. 烧录前离线核查

### 5.1 必跑验收

先对镜像执行：

```sh
./scripts/verify-t4-lumelo-rootfs-image.sh /absolute/path/to/lumelo-t4-rootfs-YYYYMMDD-vN.img
```

通过标准：

- `0 failure(s), 0 warning(s)`

若本轮改动涉及 `NanoPC-T4` 无线链路，再补跑：

```sh
./scripts/compare-t4-wireless-golden.sh \
  --board-base-image /absolute/path/to/rk3399-sd-*.img.gz \
  --image /absolute/path/to/lumelo-t4-rootfs-YYYYMMDD-vN.img
```

通过标准：

- `0 failure(s), 0 warning(s)`
- `BCM4356A2.hcd`
- `hciattach.rk`
- `bcmdhd.conf`
  与官方底图一致

### 5.2 本轮必须特别确认的板级文件

离线验收之外，重点确认这些文件已被纳入镜像：

- `/usr/bin/hciattach.rk`
- `/etc/firmware/BCM4356A2.hcd`
- `/etc/modprobe.d/bcmdhd.conf`
- `lumelo-bluetooth-uart-attach.service`
- `lumelo-ssh-hostkeys.service`

## 6. 烧录后真机核查

### 6.1 第一层：最小外部可见性

先确认：

- `http://<T4_IP>:18080/` 可打开
- `http://<T4_IP>:18080/healthz` 可返回
- `http://<T4_IP>:18080/provisioning-status` 可返回
- `http://<T4_IP>:18080/logs.txt` 可返回
- `ssh root@<T4_IP>` 可登录

如果这里任何一项失败，不要先假定是手机或 APK 问题。

### 6.2 第二层：SSH 登录后必查

SSH 进入板子后，默认先跑：

```sh
systemctl --no-pager --full status \
  ssh.service \
  lumelo-ssh-hostkeys.service \
  lumelo-bluetooth-uart-attach.service \
  bluetooth.service \
  lumelo-wifi-provisiond.service
```

```sh
journalctl -b --no-pager | grep -Ei 'brcm|bcmdhd|dhd|bluetooth|hci|firmware|sshd'
```

```sh
rfkill list
bluetoothctl show
hciconfig -a
```

### 6.3 第三层：官方板级蓝牙 smoke test

如果板端 `hci0` 已存在，继续做最朴素的板级测试：

```sh
hciconfig hci0 up
hcitool scan
```

判定方式：

- `hciconfig hci0 up` 不应报 firmware / patch / UART attach 错误
- `hcitool scan` 至少应能开始 classic Bluetooth 扫描

注意：

- 这一步是“控制器最小存活测试”
- 它不能替代 BLE 配网验证
- 但如果连这一步都过不了，就不该先怀疑手机 APK

### 6.4 第四层：BLE 配网链核查

再看 BLE 配网链本身：

- `/healthz` 中 `provisioning_available` 应为 `true`
- `provisioning_state` 不应长期卡在明显异常状态
- `logs.txt` 中不应出现：
  - `Patch not found`
  - `Cannot open directory '/etc/firmware'`
  - `Direct firmware load ... failed`
  - `no hostkeys available`
- 手机端应至少能在：
  - `BLE TEST SCAN`
  - 或 Lumelo 专用扫描
  中看到设备

### 6.5 第五层：Wi-Fi 与 WebUI 闭环

如果手机已能发现并连接 T4，再继续看：

- Wi-Fi 凭据是否成功写入
- 板子是否拿到正确 IP
- WebUI 是否可打开
- `/library`、首页、播放控制页是否正常

### 6.6 第六层：真实曲库与 ALSA smoke

若本轮需要继续验证真实媒体链，优先先跑最小 smoke，而不是一开始就上整套业务逻辑：

```sh
lumelo-media-smoke smoke --skip-play
lumelo-media-smoke list --first-wav
lumelo-media-smoke play --first-wav
```

判定方式：

- `smoke --skip-play` 应能生成 demo `WAV` 并让 `media-indexd` 成功写入 `library.db`
- `list --first-wav` 应至少返回一条可解析的真实轨道
- `play --first-wav` 应调用 `aplay -D default` 并正常退出
- `http://<T4_IP>:18080/library` 应能看到真实条目，而不是全 `0`

注意：

- 这一步验证的是：
  - 真实媒体文件
  - 索引写库
  - `controld` 读库
  - `ALSA hw` 最小播放链
- 它当前还不等于：
  - `playbackd` 已经接入真实解码与真实输出
  - 首页上的播放控制已经具备最终用户态能力

### 6.7 第七层：`playbackd` 真机输出回归

若本轮要继续验证首页播放控制是否已具备真实用户态能力，至少准备：

- 一首真实 `wav`
- 一首非 `wav` 轨道
  - 当前优先用 `m4a/aac`

最低回归集：

- 也可以直接先跑板端 helper：
  - `lumelo-media-smoke regress-playback --timeout 8`
  - 如果板子上已经有多个 indexed volume：
    - `lumelo-media-smoke regress-playback --mount-root /var/lib/lumelo/test-media --timeout 8`
  - 若要强制指定解码格式：
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format flac`
  - 若是长时长压缩格式：
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format mp3 --skip-mixed`
    - `lumelo-media-smoke regress-playback --timeout 8 --decoded-format ogg --skip-mixed`
  - 若要真的等待长 `mp3` 自然播完并验证自动切歌：
    - `lumelo-media-smoke regress-playback --timeout 140 --decoded-format mp3`
  - 若要真的等待长 `ogg` 自然播完并验证自动切歌：
    - `lumelo-media-smoke regress-playback --mount-root /var/lib/lumelo/test-media --timeout 140 --decoded-format ogg`
- 对 `wav` 执行：
  - `play / pause / resume / stop`
- 对 `m4a/aac` 执行：
  - `play / pause / resume / stop`
- 做一次混合队列：
  - `m4a -> wav`
  - 等第一首自然播完，看是否自动切到第二首
- 再补一次：
  - `prev`
  - `play_history`

判定方式：

- `wav` 路径应看到：
  - `aplay -D default <real file>`
- 已解码非 `wav` 路径应看到：
  - `aplay -D default -t raw -f S16_LE -c <channels> -r <sample_rate>`
- `STATUS` 与队列当前项应和真机输出保持一致
- 长时长 `ogg` 回归通过时，应看到：
  - 第一首为：
    - `aplay -D default -t raw -f S16_LE ...`
  - 自然播完后自动切到：
    - `aplay -D default /var/lib/lumelo/test-media/Blue Room Sessions/01 - Warmup Tone.wav`

### 6.8 第八层：批量曲库扫描回归

若本轮要确认曲库扫描已经不只是“单首 smoke”，可直接在板子上执行：

```sh
lumelo-media-smoke regress-library-scan
```

判定方式：

- helper 会在独立目录下生成多目录、多格式 fixture
- 也会额外生成专辑封面 fixture：
  - `Album Alpha/folder.jpg`
  - `Album Alpha/cover.jpg`
  - `Album Beta/cover.jpg`
- 然后执行：
  - `media-indexd scan-dir <fixture_root>`
- 最终应输出：
  - `Library scan regression passed`
- 当前这条回归通过时，至少会覆盖：
  - `wav`
  - `m4a`
  - `flac`
  - `mp3`
  - `ogg`
- 并且还应覆盖：
  - `albums = 3`
  - `covered_tracks = 4`
  - `artwork refs = 2`
  - `Album Alpha` 命中 `folder.jpg` 优先级

若这轮还要从 WebUI 再看一眼，执行：

```sh
curl -fsSL http://192.168.1.121:18080/library | rg "library-cover-art|Album Alpha|Album Beta"
curl -I http://192.168.1.121:18080/artwork/thumb/320/<hash>.jpg
```

判定方式：

- `/library` 页面应出现：
  - `class="library-cover-art"`
- `/artwork/thumb/320/...jpg` 应返回：
  - `200 OK`
  - `Content-Type: image/jpeg`

### 6.9 第九层：tagged 元数据真机回归

若本轮要确认“更像真实用户曲库”的元数据已经贯通到索引与 WebUI，至少补一轮：

- 专辑名
- 专辑艺人
- 曲目艺人
- 年份
- 流派
- `disc_no`
- `track_no`
- 专辑封面

当前推荐最小样本：

- `Northern Signals`
  - `Disc 1`
  - `Disc 2`
- `Transit Lines`

判定方式：

- `media-indexd scan-dir <tagged_root>` 后，数据库中应能查到：
  - `album_title`
  - `album_artist`
  - `year`
  - `disc_no`
  - `track_no`
  - `genres`
- `/library` 页面应出现：
  - 专辑标题
  - 专辑艺人
  - 年份
  - 对应曲目标题与曲目艺人
  - 封面缩略图
- 至少再从这套 tagged 曲库里直接点播一首：
  - 确认不是“只会展示，不会播放”

### 6.10 第十层：外部媒体最小入口

若当前现场还没有把 TF / USB 热插拔链完全做完，至少先确认“已挂载介质如何安全入库”有明确落脚点。

当前可用命令：

```sh
lumelo-media-import list-mounted
lumelo-media-import scan-mounted
lumelo-media-import scan-path /absolute/path/to/media-root
```

判定方式：

- 没有外部介质时：
  - `list-mounted` 应返回：
    - `[]`
- 对一个显式目录执行：
  - `scan-path <path>`
  - 应能成功触发：
    - `media-indexd scan-dir <path>`
- 若当前正处于播放期：
  - 再执行 `scan-path` 或 `scan-mounted`
  - 应拒绝扫描并返回：
    - `playback quiet mode is active; refusing media scan unless --force is used`

### 6.11 第十一层：模拟块设备导入回归

若现场暂时没有真 TF / USB 介质，但想先确认“块设备导入主链”是否成立，可以用 loop ISO 做一轮替代回归。

最小流程：

```sh
losetup -f --show /tmp/lumelo-metadata-suite.iso
lumelo-media-import import-device /dev/loop0
lumelo-media-import reconcile-volumes
```

判定方式：

- `import-device` 应能完成：
  - 挂载到 `/media/<label>`
  - 触发 `media-indexd scan-dir <mountpoint>`
  - 输出稳定 `volume_uuid`
- 随后数据库中应能查到：
  - 该 `mountpoint` 下的真实轨道
- 再至少从这块模拟介质里点播一首：
  - 验证不是“只会挂载入库，不会播放”
- 卸载后执行：
  - `reconcile-volumes`
  - 数据库中的该 volume 应转为离线

### 6.12 第十二层：稳定性回归

当前板端已补了两条正式回归命令，优先用它们收口“服务恢复”和“坏文件边界”。

执行：

```sh
lumelo-media-smoke regress-playbackd-restart --mount-root /var/lib/lumelo/test-media-tagged --timeout 8
lumelo-media-smoke regress-bad-media --timeout 8
```

判定方式：

- `regress-playbackd-restart`
  - 应输出：
    - `Playbackd restart regression passed`
  - 且应确认：
    - `queue_entries` 保持
    - `current_track` 保持
    - 服务重启后状态回到 `stopped`
- `regress-bad-media`
  - 应输出：
    - `Bad-media regression passed`
  - 当前允许出现的现状是：
    - 坏文件仍会被索引
  - 但必须满足：
    - 播放坏文件不会拖挂 `playbackd`
    - 状态会进入 `quiet_error_hold`
    - 随后有效轨道仍能恢复为 `quiet_active`

## 7. 典型失败信号与优先怀疑项

- 日志出现 `sshd: no hostkeys available -- exiting`
  - 优先怀疑 SSH host key 自动生成链未生效

- 日志出现 `Cannot open directory '/etc/firmware'`
  - 优先怀疑蓝牙 patch 目录没有从官方底图正确带进镜像

- 日志出现 `Patch not found, continue anyway`
  - 优先怀疑 `BCM4356A2.hcd` 未命中或 `hciattach.rk` 未按官方链路正确 attach

- 日志出现 `Direct firmware load for brcm/brcmfmac4356-sdio.bin failed`
  - 优先怀疑镜像仍在走错误的 `brcmfmac` 路线，而不是官方 `bcmdhd` 路线

- `bluetoothctl show` 里仍是 `Discoverable: no`
  - 优先怀疑适配器状态没有真正切到可发现

- App 的 `BLE TEST SCAN` 能看到很多别的设备，但看不到 `Lumelo T4`
  - 优先怀疑 T4 板端广播没真正发出来

## 8. 与经典蓝牙测试的关系

经典蓝牙测试有价值，但角色要限定清楚：

- 它适合做“控制器活没活”的快速 smoke test
- 它不等于 BLE 广播一定正常
- 它也不等于 GATT 配网链一定正常

所以后续如果加“经典蓝牙可发现”诊断模式，它应该作为：

- 辅助诊断项

而不是替代当前的 BLE 配网路径。
~~~

## `docs/T4_WiFi_Golden_Baseline.md`

- bytes: 8510
- segment: 1/1

~~~md
# T4 Wi-Fi 金样基线与 Lumelo 差异

> 文档边界：
> - 本文件只固定 `NanoPC-T4` 无线板级金样、当前 `Lumelo` 的实现差异，以及需要长期保留的 Wi-Fi 结论。
> - 当天现场进展、当前阻塞和下一步待办看 [AI_Handoff_Memory.md](/Volumes/SeeDisk/Codex/Lumelo/docs/AI_Handoff_Memory.md) 和 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)。
> - 手机与板子的经典蓝牙配网协议、安全传输契约看 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)。
> - 真机 bring-up 操作步骤和排障动作看 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md) 与 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)。

本文件用于固定 `NanoPC-T4` 官方金样在 `Wi-Fi` 相关底座上的真实做法，
并和当前 `Lumelo` 自定义 rootfs 的实现做对比。

当前金样来源：

- `rk3399-sd-debian-trixie-core-4.19-arm64-20260319.img.gz`
- `rk3399-usb-debian-trixie-core-4.19-arm64-20260319.zip`

目标不是“照抄官方整套系统”，而是明确哪些板级依赖必须继承，哪些用户态
网络策略可以继续保持 Lumelo 自己的路线。

## 1. 官方金样已确认的 Wi-Fi 底座

### 1.1 板级驱动与 firmware

- Wi-Fi 驱动走 `bcmdhd`
- 蓝牙走 `hci_uart + btbcm`
- 板载无线为 `AP6356S` 组合模组
- 官方实际加载的 Wi-Fi firmware 路径是：
  - `/system/etc/firmware/fw_bcm4356a2_ag.bin`
  - `/system/etc/firmware/nvram_ap6356.txt`
- 官方蓝牙 patch 路径是：
  - `/etc/firmware/BCM4356A2.hcd`
- 官方驱动策略文件是：
  - `/etc/modprobe.d/bcmdhd.conf`

### 1.2 官方运行态已验证现象

在官方金样真机运行态上，已经确认：

- `bcmdhd` 正常加载
- `/system/etc/firmware/fw_bcm4356a2_ag.bin` 打开成功
- `/system/etc/firmware/nvram_ap6356.txt` 打开成功
- 日志出现：
  - `Firmware up: op_mode=0x0005`
  - `wl_android_wifi_on : Success`
- 未出现我们 Lumelo 镜像那类：
  - `brcmfmac ... -110`
  - `failed backplane access over SDIO`

这说明：

- 官方板级 Wi-Fi firmware / 驱动链是健康的
- 我们后续对 `Lumelo` 的修复必须优先继承这条 vendor 路线

## 2. 官方金样的用户态网络栈

### 2.1 当前观察到的用户态组合

官方金样当前不是 `systemd-networkd + iw` 路线，而是：

- `NetworkManager`
- `ifupdown`
- `dhcpcd-base`
- `wireless-tools`
- `wpa_supplicant`

当前真机采样结果：

- `iw` 未安装
- `systemd-networkd.service` 已安装但默认 `disabled`
- `NetworkManager.service` 默认 `enabled`
- `wpa_supplicant@wlan0.service` 默认 `inactive`

### 2.2 已确认的配置要点

官方金样关键配置如下：

- `/etc/NetworkManager/NetworkManager.conf`

```ini
[main]
plugins=ifupdown,keyfile

[ifupdown]
managed=true
```

- `/etc/NetworkManager/conf.d/12-managed-wifi.conf`

```ini
[keyfile]
unmanaged-devices=wl*,except:type:wifi
```

- `/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf`

```ini
[keyfile]
unmanaged-devices=interface-name:wlan1
```

- `/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf`

```ini
[device]
wifi.scan-rand-mac-address=no
```

- `/etc/network/interfaces`

```ini
# interfaces(5) file used by ifup(8) and ifdown(8)
# Include files from /etc/network/interfaces.d:
source /etc/network/interfaces.d/*
```

## 3. 当前 Lumelo 的差异

### 3.1 已经补齐的正确项

当前 `Lumelo` 已经转回官方板级主线：

- 复制官方 `bcmdhd.conf`
- 复制官方 `/etc/firmware/BCM4356A2.hcd`
- 复制官方 `/system/etc/firmware/fw_bcm4356a2_ag.bin`
- 复制官方 `/system/etc/firmware/nvram_ap6356.txt`
- 复制官方 `hciattach.rk`

### 3.2 当前仍然不同的项

当前 `Lumelo` 用户态网络栈仍然是：

- `iw`
- `wpasupplicant`
- `systemd-networkd`
- `systemd-resolved`
- `lumelo-wifi-apply` 直接写 `wpa_supplicant-<iface>.conf`

对应文件：

- [t4-bringup-packages.txt](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/manifests/t4-bringup-packages.txt)
- [30-wireless-dhcp.network](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/systemd/network/30-wireless-dhcp.network)
- [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)

这意味着：

- 板级 firmware / 驱动链已向官方对齐
- 但用户态网络管理仍是 `Lumelo` 自己的实现
- 后续若 Wi-Fi 连接、DHCP、重连、扫描仍不稳定，不能只看 firmware，还必须检查用户态网络策略是否也要贴近官方

### 3.3 本轮已落地的 Wi-Fi 改造

本轮已经把两类高价值差异先落到仓库里：

- [lumelo-wifi-apply](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-wifi-apply)
  现在支持双路径：
  - `NetworkManager`
  - `wpa_supplicant + systemd-networkd`
- 默认策略：
  - `LUMELO_WIFI_BACKEND=auto`
  - 如果 `NetworkManager` 处于 active，则优先走 `nmcli`
  - 否则回退到当前 `wpa_supplicant` 写配置方式
- 无线接口探测已修正：
  - 优先看 `nmcli`
  - 再看 `iw`
  - 最后看 `/sys/class/net/*/wireless`
  - 显式跳过 `p2p-dev*`

这条修正很重要，因为官方金样运行态里同时存在：

- `wlan0`
- `p2p-dev-wlan0`

如果后续镜像没有 `iw`，旧逻辑有概率误把 `p2p-dev-wlan0` 当成主无线接口。

- [lumelo-t4-report](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-t4-report)
  已新增：
  - `nmcli general status`
  - `nmcli device status`
  - `/etc/NetworkManager/NetworkManager.conf`
  - `/etc/NetworkManager/conf.d/*.conf`
  - `/etc/network/interfaces`

- overlay 里已预置官方 `NetworkManager` 基线配置：
  - [NetworkManager.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/NetworkManager.conf)
  - [12-managed-wifi.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/12-managed-wifi.conf)
  - [99-unmanaged-wlan1.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf)
  - [disable-random-mac-during-wifi-scan.conf](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf)
  - [interfaces](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/etc/network/interfaces)

当前这一步的目标不是立刻把 `Lumelo` 整套用户态切到 `NetworkManager`，而是：

- 先把官方金样里已确认有价值的配置固化进镜像
- 让 `lumelo-wifi-apply` 具备双路径兼容能力
- 把后续真机验证需要的诊断信息补齐

## 4. 对 Lumelo Wi-Fi 改造的规划

### 4.1 P0 必须保留

这些项后续不能再回退成通用 Debian 假设：

- `bcmdhd`
- `/etc/firmware/BCM4356A2.hcd`
- `/system/etc/firmware/fw_bcm4356a2_ag.bin`
- `/system/etc/firmware/nvram_ap6356.txt`
- `hciattach.rk`

### 4.2 P1 近期需要补的兼容与验证

- 真机验证时除了 `provisioning-status.json`，还要同时记录：
  - `wpa_supplicant@<iface>.service`
  - `networkctl status <iface>`
  - `nmcli device status`
  - `ip addr show dev <iface>`
  - DHCP 是否真实拿到 IPv4
- 真机验证要增加一条“在连接目标 Wi-Fi 前先做 scan”的观察项

### 4.3 P2 若当前系统仍不稳定，需要考虑的路线

如果板级 firmware 已经对齐官方，但 `Lumelo` 自己的 Wi-Fi 联网仍持续不稳定，
则要认真评估是否改成更接近官方的用户态方案：

- 为 `T4` 单独引入并默认启用 `NetworkManager`
- 或保留当前双路径兼容，但明确：
  - 哪块板默认走 `systemd-networkd + wpa_supplicant`
  - 哪块板默认走 `NetworkManager`

当前阶段不直接切到 `NetworkManager`，原因是：

- 先把板级 firmware / 经典蓝牙配网主通道跑通更关键
- 用户态网络管理器切换属于第二层变量，不应和当前蓝牙主通道改造绑在同一轮

## 5. 当前结论

当前 `Lumelo` Wi-Fi 相关问题不能再简单归因成“缺 firmware”。

更准确的分层应该是：

1. 板级 vendor firmware / 驱动链必须继承官方
2. 用户态网络栈当前仍与官方明显不同
3. 后续若 Wi-Fi 配网或 DHCP 有问题，要同时检查：
   - vendor firmware 是否完整
   - `lumelo-wifi-apply` 是否选择了正确 backend
   - `systemd-networkd` 路线是否足够稳定
   - 是否需要把 `NetworkManager` 设为默认路径
~~~

