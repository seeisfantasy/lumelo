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
