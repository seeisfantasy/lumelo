# Lumelo 本地音频系统产品开发说明书

> 文档边界：
> - 本文件只维护产品原则、版本边界、服务权威、状态机、API / 服务 contract、恢复语义、验收矩阵和长期运行规则。
> - 环境、出包、在线更新和真机操作约定看 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)。
> - 每天真实发生的开发过程和阶段变化看 [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)。
> - 手机 APK 当前状态、结构和后续计划看 [Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md)。
> - 经典蓝牙配网协议、安全传输和板端交互契约看 [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)。
> - T4 板级无线金样、当前差异和真机核查看 [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md) 与 [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)。

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
- 无封面的专辑也必须显示默认方形封面占位，保持专辑卡片格式统一
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
- V1 当前 setup path 已切到经典蓝牙 provisioning
- `Raw BLE Scan` 只保留为诊断能力，不再作为主配网发现路径
- 一旦完成联网，steady-state 主交互仍回到 Ethernet / Wi-Fi 下的 WebUI
- 蓝牙不作为 V1 的主要 WebUI 承载方式
- 蓝牙不参与 V1 的音频输入输出链路

### 7.4 手机 APK

V1 当前保留一个独立手机 APK，但它的角色需要明确限定。

当前定位：

- 经典蓝牙 / Wi-Fi provisioning 工具
- 首次联网 setup path
- 联网成功后的 APK 内 `WebView` 外壳
- 板端 Bluetooth / 配网异常时的诊断入口

当前不是：

- 主播放器 App
- 主曲库浏览 App
- steady-state 主控制端

V1 的 steady-state 主交互仍然是：

- Ethernet / Wi-Fi 下的 WebUI

当前 APK 的功能结构按产品层划分为：

- `Setup Shell`
  - 权限、环境状态、输入与主流程按钮
- `Classic Bluetooth 扫描层`
  - Lumelo 经典蓝牙扫描、候选筛选与主通道发现
- `BLE Diagnostic 扫描层`
  - `Raw BLE Scan` 自检入口
- `RFCOMM / SPP 会话层`
  - 连接、逐行 JSON 收发、最小恢复
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
- 不在固件、`playbackd`、helper、bring-up 流程里主动调节 `DAC` / ALSA mixer 音量
- 默认保持原始数字信号与固定电平输出
- 音量控制交给后级：
  - 前级
  - 功放
  - 有源音箱

### 输出电平原则

- 默认不暴露板端音量滑杆
- 默认不自动改 `PCM` / `Master` / 设备私有 gain
- 不允许为了“先出声”而在板端偷偷拉高 mixer 音量
- 若现场需要临时改 mixer，只能作为 bring-up 诊断动作，不视为正式产品行为

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
- 不要求在 `eMMC` 已装系统时，无人工干预默认优先从 `TF` 启动
  - 对当前开发板来说，这属于：
    - `boot chain / board-support` 课题
    - 单独 backlog
  - 不作为 `V1` 功能验收阻塞项

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
- 浏览器侧逐步补稳定的 `/api/v1/...` Web contract
- `/api/v1/...` 继续跑在同一个 `controld` 进程与监听端口内
- 不引入 SPA 构建链
- 页面模板和静态资源可直接打包进 `controld` 二进制
- 运行时临时对象统一放入 `/run/lumelo/`
- 持久化状态统一放入 `/var/lib/lumelo/`

设计目标：

- 保持构建链简单
- 保持播放核心与 UI 层解耦
- 让后续 UI 重构尽量只影响 Web 层
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
- 浏览器侧 JSON / SSE contract
- 控制命令转发
- 设置读写
- 登录、会话与认证

边界：

- 不自行计算顺序播放逻辑
- 不自行重建 `shuffle` 顺序
- 不自行更新当前播放指针
- 不持有队列状态权威
- 不要求 UI 布局变化同步改 Rust 服务语义

WebUI contract：

- 当前 WebUI 不是独立 frontend app；它由 `controld` 内嵌 SSR templates、static files、少量原生 JS 和 `/api/v1/...` JSON / SSE 组成。
- 不新增单独 frontend daemon，不新增第二个监听端口，不引入 SPA 构建链。
- UI 重构应优先只影响 `templates`、CSS、浏览器侧 JS 和 `controld` 的薄 API adapter。
- 产品方向是 `music player first, diagnostics second`：首页和曲库优先呈现播放、曲库和当前输出状态；配网、日志和 bring-up 信息必须可查，但不压过播放器主界面。
- 不做假的功能入口：未实现的歌词、进度条、radio、全屏 Now Playing、歌单等不能只做 UI 壳。
- 页面只消费 domain-oriented data，不把按钮文案、布局细节或旧模板字段写进底层 API 语义。

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
- `PLAYBACK_FAILED` 必须带错误分类：
  - `failure_class=output`：输出链错误，退出 Quiet Mode
  - `failure_class=content`：内容错误，可进入 `quiet_error_hold`
  - `failure_class=media_offline`：介质离线，默认不自动切歌
- 播放期间不应继续保留手机配网用的 Bluetooth provisioning / advertising
  - 至少不应在播放态持续广播
  - 停止播放或播放失败后，应恢复到可重新配网的板端状态

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

持久化写入 contract：

- 配置、队列、历史、认证状态等关键持久化对象必须采用 atomic write。
- 写入流程：
  - 写临时文件
  - `fsync(file)`
  - `rename`
  - `fsync(parent directory)`
- 读取时必须限制文件大小，避免坏文件或异常大文件拖垮服务。
- 解析失败时必须有清晰 fallback 和 warning，不允许静默伪装成功。
- 适用对象至少包括：
  - `/etc/lumelo/config.toml`
  - `/var/lib/lumelo/queue.json`
  - `/var/lib/lumelo/history.json`
  - `/var/lib/lumelo/auth.json`

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
- WebUI 首页可以读取 `playbackd` 的历史快照用于展示播放历史
- WebUI 首页播放历史按最近播放在上方显示
- WebUI 首页不得提前暴露 shuffle / 伪随机后续队列顺序
- WebUI 首页播放历史里的曲目必须可直接播放，使用 `play_history` 路径
- `play_history` 是 `Play Now` 语义：
  - 不把历史曲目追加到队尾
  - 不重建后续 `play_order`
  - 只替换当前播放位并保留原后续队列
- WebUI transport 中间主按钮随状态切换：
  - 播放中显示 `暂停`
  - 已停止 / 已暂停 / 无活动播放时显示 `播放`
- WebUI 首页与曲库页的主 transport 都应保留 `停止`，避免用户只能暂停或切歌
- WebUI 底部 mini player 的曲名和路径类信息必须单行省略，不能因长标题撑高悬浮播放器

## 19. 错误处理与恢复原则

### 输出链错误

下列错误按 `fail-stop` 处理：

- DAC 不可用
- DAC 被拔掉
- ALSA 打不开
- 所选 `DSD` transport 不支持，且 `PCM fallback` 也不可用

处理原则：

- 立即停止当前输出
- 发出 `PLAYBACK_FAILED`
- 退出 Quiet Mode
- UI 明确提示错误原因
- `native_dsd` 下不自动切到 `DoP`
- `dop` 下不自动切到 `Native DSD`
- 所选 `DSD` transport 不可用时，允许自动回退到 `PCM`
- 不自动切下一首

补充：

- 当前 DSD 输出策略默认值为：
  - `native_dsd`
- `dop` 保留为手动可选策略
- `native_dsd` 的行为是：
  - 优先 `Native DSD`
  - Native 不可用时，不自动切到 `DoP`
  - 先尝试 `PCM fallback`
- `dop` 的行为是：
  - 只尝试 `DoP`
  - DoP 不可用时，不自动切到 `Native DSD`
  - 再尝试 `PCM fallback`

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
- 开发 / bring-up 图默认应表现为 appliance DHCP client；当前产品入口已明确要求启用 `MulticastDNS`，发布 `lumelo.local` 和 `_http._tcp:80`。`LinkLocalAddressing` 与 `LLMNR` 仍默认关闭。
- WebUI 正式入口监听 `80/tcp`。默认入口是 `http://lumelo.local/`；可靠入口是配网状态返回的 `http://<T4_IP>/`。
- 不开发、不承诺、不验收 `http://lumelo/` 这种单标签 hostname 入口；不得为了它开启 `LLMNR`、NetBIOS 或引入 router DNS 依赖。
- mDNS / DNS-SD 是 setup、stopped、control 阶段的增强入口。当前实现通过已有 `systemd-resolved` 发布，live T4 观测为 `1 thread / RSS ~14 MB`，新增开销很小但不是零；为满足极限纯净播放器目标，后续 `sessiond` 必须在正式 `Playback Quiet Mode active` 中关闭或抑制 mDNS/DNS-SD 广播，停止播放后按 snapshot 恢复。
- Wi-Fi 配置应用流程优先重配置目标无线接口，不优先重启整套网络栈。
- 开发图必须支持无头排障：SSH host keys 需要可自动补齐，Web 侧默认保留 `/healthz`、`/provisioning-status`、`/logs`、`/logs.txt`。

更细的现场核查步骤、无线金样差异和配网协议细节，统一分别维护在：

- [T4_Bringup_Checklist.md](/Volumes/SeeDisk/Codex/Lumelo/docs/T4_Bringup_Checklist.md)
- [Development_Environment_README.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Environment_README.md)
- [Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md)

## 22. 状态机契约

本章把前文原则落成可实现、可测试的状态机。

实现时如果字段、页面或 service 已存在，但状态迁移不符合本章，则视为未完成。

### 22.1 Playback Core 状态机

权威服务：`playbackd`

稳定状态：

| 状态 | 含义 | 允许的主要命令 | 退出条件 |
| --- | --- | --- | --- |
| `stopped` | 无活动播放，队列可存在但没有正在输出的曲目 | `play`, `play_context`, `play_history`, `queue_*`, `set_order_mode`, `set_repeat_mode` | 播放请求被接受 |
| `pre_quiet` | 播放请求已接受，正在准备输出和预缓冲，尚未确认第一帧写入 | `stop`, `next`, `previous` | 第一帧写入成功或准备失败 |
| `playing` | 音频已开始真实输出 | `pause`, `stop`, `next`, `previous`, `set_repeat_mode` | 暂停、停止、切歌、失败或自然结束 |
| `paused` | 当前播放上下文仍保留，但音频输出暂停 | `resume`, `stop`, `next`, `previous` | 恢复、停止、切歌 |
| `quiet_error_hold` | 内容错误后的受控等待状态 | `stop`, `next`, `previous`, 用户显式 `play*` | 6 秒到期自动切歌，或用户显式操作取消 |
| `failed` | 输出链失败或连续内容错误达到上限 | `play`, `play_context`, `play_history`, `queue_*` | 新播放请求 |

迁移规则：

- `stopped -> pre_quiet`
  - 用户发起 `play` / `play_context` / `play_history`。
  - `playbackd` 接受请求后发 `PLAY_REQUEST_ACCEPTED`。
- `pre_quiet -> playing`
  - 输出链成功打开并且第一帧音频成功写入 ALSA。
  - 只有此时才允许发 `PLAYBACK_STARTED`。
- `pre_quiet -> failed`
  - DAC、ALSA、文件打开、解码准备或首帧写入失败。
  - 必须发 `PLAYBACK_FAILED`。
- `playing -> paused`
  - 用户显式暂停。
  - 发 `PLAYBACK_PAUSED`。
- `paused -> playing`
  - 用户显式恢复。
  - 若恢复后需要重新打开输出链，必须重新满足首帧语义。
  - 发 `PLAYBACK_RESUMED` 或新的 `PLAYBACK_STARTED`，由实现 contract 固定，不允许 UI 猜测。
- `playing / paused / pre_quiet / quiet_error_hold -> stopped`
  - 用户显式停止。
  - 发 `PLAYBACK_STOPPED`。
- `playing -> quiet_error_hold`
  - 内容错误，例如当前文件损坏、读不到、解码失败但输出链本身健康。
  - 发 `PLAYBACK_FAILED`，错误分类必须是 content 类。
- `quiet_error_hold -> pre_quiet`
  - 6 秒后自动尝试下一首可播放曲。
  - 连续自动跳过不超过 3 次。
- `quiet_error_hold -> stopped`
  - 队列没有可继续项，或连续跳过达到上限。

禁止行为：

- 不允许在输出链尚未写入第一帧前发 `PLAYBACK_STARTED`。
- 不允许 `controld` 自行把按钮状态推导成播放核心状态。
- 不允许内容错误写入 `queue.json` / `history.json` / `library.db`。
- 不允许输出链错误自动切下一首。

### 22.2 Quiet Mode 状态机

权威服务：`sessiond`

`sessiond` 只消费 `playbackd` 事件，不参与播放决策。

| 状态 | 进入事件 | 系统动作 | 退出事件 |
| --- | --- | --- | --- |
| `inactive` | 启动默认、停止后、失败后 | 非播放态服务按当前模式恢复 | `PLAY_REQUEST_ACCEPTED` |
| `pre_quiet` | `PLAY_REQUEST_ACCEPTED` | 停止 / 冻结非关键服务，停止 Bluetooth provisioning / advertising | `PLAYBACK_STARTED` / `PLAYBACK_FAILED` / `PLAYBACK_STOPPED` |
| `active` | `PLAYBACK_STARTED` | 保持播放静默态，只保留必要控制入口 | `PLAYBACK_STOPPED` / `PLAYBACK_FAILED` |
| `error_hold` | content 类 `PLAYBACK_FAILED` 且播放核心进入 hold | 保持静默态，等待播放核心内部 auto-skip 或用户操作 | auto-skip 成功、用户停止、失败上限 |

Quiet Mode 服务切换 contract：

- `sessiond` 必须有明确配置项：
  - `quiet_stop_units`
  - `quiet_start_units`
  - `freezable_units`
  - protected unit deny-list
- `controld`、`playbackd`、SSH 调试通道不得被误停。
- `media-indexd`、封面派生、缩略图生成、Bluetooth provisioning 属于默认可停 / 可冻结域。
- mDNS/DNS-SD 属于默认可抑制域：`sessiond` 进入正式 Quiet Mode 时关闭或抑制 `lumelo.local` / `_http._tcp` 发布，退出 Quiet Mode 后恢复进入前状态；不得通过 stop 核心网络栈来实现。
- stop / freeze 失败必须进入可诊断状态，不允许静默吞掉。
- 进入 Quiet Mode 前必须记录本次运行期 `QuietReconcileSnapshot`：
  - 原来 active 的 unit，退出后才恢复
  - 原来 inactive 的 unit，退出后不得擅自启动
  - 原来 failed 的 unit，不得误恢复成 active
- `PLAYBACK_FAILED failure_class=output` 后必须恢复到可重新配网的非静默态。
- `PLAYBACK_FAILED failure_class=content quiet_behavior=hold` 可继续 `error_hold`。

### 22.3 Mode / Connectivity 状态机

权威模块：`mode manager` + `controld settings`

`mode` 状态：

| 状态 | V1 行为 |
| --- | --- |
| `local` | 启动 Local Mode 播放栈，是 V1 主状态 |
| `bridge` | 只启动占位页、基础网络、`controld`、设置页和 `healthz`，不启动真实桥接服务或 Local playback stack |

迁移规则：

- 首次启动默认 `local`。
- 后续启动读取 `/etc/lumelo/config.toml` 中最后成功保存的 `mode`。
- 模式切换只允许在设置页进行。
- 用户确认重启前，不写入 committed mode。
- 用户取消重启时，UI 恢复当前 committed mode。
- `mode=bridge` 在 V1 只允许进入 placeholder，不得启动 AirPlay / UPnP / bridge daemons。
- `bridge` placeholder 必须保留切回 `local` 的设置入口，避免用户失去恢复路径。

`interface_mode` 状态：

| 状态 | V1 行为 |
| --- | --- |
| `ethernet` | 启用有线网络路径，Wi-Fi 非当前控制路径 |
| `wifi` | 启用 Wi-Fi 网络路径，有线非当前控制路径 |
| `bluetooth_control` | V1 只保留未来枚举，不作为正式 steady-state 控制路径 |

单接口运行 contract：

- 任一时刻只允许一个 steady-state 控制接口作为主路径。
- setup / provisioning 阶段可短暂启用所需通道，但成功联网后应回到 Ethernet / Wi-Fi WebUI。
- 播放态不得持续保留 Bluetooth provisioning / advertising。
- 修改 `mode` 或 `interface_mode` 属于重启生效设置。

### 22.4 Settings 状态机

权威服务：`controld`

配置对象：

- 默认配置：`/usr/share/lumelo/default_config.toml`
- 当前配置：`/etc/lumelo/config.toml`
- 运行时状态：`/run/lumelo/*`

设置状态：

| 状态 | 含义 |
| --- | --- |
| `clean` | UI 展示值等于 committed config |
| `dirty` | 用户已修改但未保存 |
| `pending_reboot` | 已保存，必须重启生效 |
| `invalid` | 校验失败，不允许保存 |
| `fallback_active` | 当前配置解析失败，系统使用默认配置启动 |

设置 contract：

- `mode`、`interface_mode`、`dsd_output_policy` 属于 `pending_reboot`。
- UI theme 等纯 UI 设置可立即生效。
- 配置解析失败时允许 fallback 到默认配置，但 UI 必须明确告警。
- `ssh_enabled` 是特权设置，不只是展示字段；开发镜像和正式镜像默认值可以不同，但必须可验证。
- 设置保存必须鉴权，并且有 CSRF / Origin protection。

## 23. API 与服务 Contract

本章定义模块之间“谁拥有语义、谁只是转发”的边界。

具体 JSON 字段可以随实现扩展，但不得违反本章。

### 23.1 Service Authority Matrix

| 领域 | 权威服务 | 可读消费者 | 禁止事项 |
| --- | --- | --- | --- |
| 播放状态 | `playbackd` | `controld`, WebUI, `sessiond` | WebUI / `controld` 自行推导播放状态 |
| 队列与 `play_order` | `playbackd` | `controld`, WebUI | `controld` 构建 shuffle 顺序或更新 current pointer |
| 播放历史 | `playbackd` | `controld`, WebUI | WebUI 自行写历史 |
| Quiet Mode | `sessiond` | `controld`, WebUI | `sessiond` 参与播放决策 |
| 曲库索引 | `media-indexd` | `controld`, WebUI | 播放期高活跃扫描 |
| 设置与认证 | `controld` | WebUI | 未鉴权修改状态 |
| mode 启动选择 | `mode manager` | `controld`, rootfs | 静态硬编码永远启动某一 target |

### 23.2 Playback Command Contract

所有播放命令最终必须进入 `playbackd`。

| 命令 | 权威语义 | 队列影响 |
| --- | --- | --- |
| `play` | 播放当前队列当前位置，或恢复可播放上下文 | 不重建队列 |
| `pause` | 暂停当前活动播放 | 不改变队列 |
| `resume` | 恢复暂停播放 | 不改变队列 |
| `stop` | 停止当前输出，保留队列 | 不重建队列 |
| `next` | 按 `play_order` 和 `repeat_mode` 前进 | 更新 current index |
| `previous` | 按 `play_order` 回退 | 更新 current index |
| `play_context` | 从曲库上下文开始播放 | 替换队列或按实现定义的 bounded candidate list 建队列 |
| `play_history` | Play Now 历史曲目 | 只替换当前播放位，保留后续队列 |
| `queue_append` | 追加到队尾 | 由 `playbackd` 更新 queue |
| `queue_insert_next` | 插到当前播放之后 | 由 `playbackd` 更新 queue |
| `queue_move` | 顺序模式下移动队列项 | `shuffle` 下必须拒绝 |
| `queue_remove` | 删除队列项 | 必须维护 current index 一致性 |
| `queue_clear` | 清空队列 | 若正在播放，必须先停止或明确定义行为 |
| `set_order_mode` | 设置 `sequential` / `shuffle` | shuffle 由 `playbackd` 生成 `play_order` |
| `set_repeat_mode` | 设置 `off` / `one` / `all` | 不重建队列 |

命令通用要求：

- 所有 state-changing command 必须鉴权。
- 浏览器来源 command 必须有 CSRF / Origin protection。
- command 失败必须返回可诊断错误码，不允许只返回泛化失败。
- `controld` 可以做参数校验和 track lookup，但不能成为播放顺序权威。
- `play_context` 如由 `controld` 传 candidate list，必须有长度上限，并明确这不是 shuffle 顺序。
- release profile 不允许 remote API 触发任意绝对路径播放。
- 开发调试路径播放必须由显式 dev flag 开启，并限制在受控媒体根目录内。

`PLAYBACK_FAILED` event 字段 contract：

- `failure_class = output | content | media_offline | decoder | permission`
- `recoverable = true | false`
- `quiet_behavior = exit | hold`
- `auto_action = none | skip_after`
- `auto_action_after_ms`
- `queue_entry_id`
- `track_uid`
- `reason_code`
- `reason_text`

### 23.3 Web API Contract

稳定 API 统一使用 `/api/v1/...`。

| API 类别 | 示例 | 鉴权 | 权威来源 |
| --- | --- | --- | --- |
| health | `GET /healthz` | 匿名 | `controld` |
| playback read | `GET /api/v1/playback/state`, `GET /api/v1/playback/queue`, `GET /api/v1/playback/history` | 登录后 | `playbackd` snapshot |
| playback command | `POST /api/v1/playback/commands` | 登录 + CSRF / Origin | 转发到 `playbackd` |
| library read | `GET /api/v1/library/...` | 登录后 | `library.db` / `media-indexd` 结果 |
| library command | `POST /api/v1/library/commands` | 登录 + CSRF / Origin | lookup 后转发到 `playbackd` |
| settings read | `GET /api/v1/settings` | 登录后 | `controld` |
| settings write | `POST /api/v1/settings` | 登录 + CSRF / Origin | `controld` |
| diagnostics | `/logs`, `/logs.txt`, `/provisioning-status` | 登录后，setup 阶段可有受限例外 | `controld` / system |
| auth setup/login | `/setup-admin`, `/login`, `/logout` | setup/login 匿名，logout 登录后 | `controld auth` |

WebUI 行为 contract：

- SSR first paint 可以保留。
- JS refresh 失败时，不允许改变播放核心语义。
- 页面重构不应要求修改 `playbackd` / `sessiond` 的状态机。
- 播放历史只显示已到达过的曲目，最近播放在上。
- 首页不得提前暴露 shuffle 后续顺序。
- transport 主按钮必须随状态显示 `播放` / `暂停`，并保留 `停止`。

### 23.4 Audio Output Contract

V1 只支持“自动选择当前唯一 USB Audio DAC”，不支持多 DAC 手动选择。

V1 UI / API：

- 设置页显示 `当前解码器` 下拉框。
- 未发现 USB Audio 解码器时显示 `未连接`。
- 发现唯一 USB Audio 解码器时显示当前解码器名称。
- 当前只读 API 为 `GET /api/v1/system/audio-output`。

V1 播放语义：

- `playbackd` 每次开始播放前读取当前 `/proc/asound/cards`。
- 只把 `USB-Audio` ALSA card 视为 V1 自动输出候选。
- 发现唯一 USB Audio DAC 时使用当前设备，例如 `plughw:CARD=<detected>,DEV=0`。
- 未发现 USB Audio DAC 时必须 fail-fast，返回 `audio_output_unavailable`。
- 同时发现多个 USB Audio DAC 时必须 fail-fast，返回 `audio_output_ambiguous`。
- 不允许假装进入播放后再自动变成 stopped。
- 不做隐式 fallback，不自动切换到另一个 DAC。

V2 多 DAC 选择预留：

- 用 `udev` 只在 `sound` 设备 `add/remove` 时触发一次扫描。
- 不用高频轮询 USB 口。
- helper 写入 `/run/lumelo/audio-output-status.json` 这类轻量 runtime 状态。
- 当前选中 DAC 断开时进入 `未连接 / 输出不可用`，播放链按 fail-stop 处理。
- 选择持久化、稳定 device key、播放中切换策略必须在 V2 开工前确认。

### 23.5 Library API Contract

曲库 API 不得把大曲库一次性无上限塞给浏览器。

最低 contract：

- 列表 API 必须支持 `limit`。
- 大列表必须支持 `cursor` 或等价分页。
- track / album / folder 响应必须能表达：
  - `available`
  - `media_offline`
  - `playback_supported`
  - `unsupported_reason`
  - `artwork_ref`
- 文件夹过滤必须正确处理 `%` / `_` 等 LIKE 特殊字符。
- WebUI 不应自行维护一套与后端不一致的可播放扩展名判断。

### 23.6 Auth / Security Contract

V1 安全模型：

- 单管理员密码。
- 首次启动必须设置密码。
- 没有“跳过设置密码”的正式路径。
- 登录后使用 session cookie。
- 所有状态修改 API 必须登录并通过 CSRF / Origin protection。
- `/healthz` 是唯一默认匿名健康检查。
- 正式镜像 SSH 默认关闭。
- 开发 / bring-up 镜像可以默认开启 SSH，但必须在镜像类型中可识别。

Endpoint profile：

| Endpoint | dev / bring-up image | release image |
| --- | --- | --- |
| `/healthz` | 匿名 | 匿名 |
| `/provisioning-status` | 可匿名或 setup token | 登录；setup 阶段可有受限例外 |
| `/logs`, `/logs.txt` | 可匿名、本地网段限定或 setup token | 登录 |
| playback commands | 登录 | 登录 |
| settings | 登录 | 登录 |
| SSH | 默认可开 | 默认关闭，设置页显式开启 |

控制面资源边界：

- HTTP server 必须配置 `ReadHeaderTimeout`、`ReadTimeout` 和 `IdleTimeout`。
- HTTP request body 和 form post 必须有限制。
- 单次 queue candidate list 必须有长度上限。
- 单个 track ID / queue entry ID 必须有限长。
- UDS command line 必须有限长。
- UDS command connection 和 SSE subscriber 必须有上限。
- 资源超限必须返回明确错误，不允许拖挂服务。

恢复 contract：

- 忘记密码只能通过 physical recovery media 重置。
- `auth-recovery.service` 只在启动阶段检查一次。
- recovery 生效后删除 auth state，并重新进入 first setup。
- recovery 不应清空音乐库和用户媒体。

### 23.7 Rootfs / systemd Contract

核心服务：

- `playbackd`
- `sessiond`
- `controld`

按需 worker：

- `media-indexd`
- 封面派生 / 缩略图生成 worker
- media reconcile worker

systemd contract：

- `local-mode.target` 是 V1 主目标。
- `bridge-mode.target` 在 V1 只启动 placeholder、基础网络、`controld`、设置页和 `healthz`。
- `media-indexd` 不应被 local target 当作高活跃常驻核心服务。
- `playbackd` 不依赖网络启动。
- `sessiond` 依赖 `playbackd`，不依赖 `controld`。
- `controld` 崩溃不影响 `playbackd`。
- release image 应逐步收紧：
  - 非 root 服务用户
  - 受控 runtime/cache/state 目录权限
  - UDS socket 权限
  - `NoNewPrivileges`
  - `ProtectSystem`
  - `PrivateTmp`
  - 精确 `ReadWritePaths`

## 24. V1 验收矩阵

本矩阵用于判断“功能真的按产品架构运行”，而不是只判断字段、页面或 service 是否存在。

### 24.1 First Boot / Auth

| 验收项 | 必须结果 |
| --- | --- |
| 新机器首次访问 WebUI | 进入管理员密码设置 |
| 未设置密码时 | 不能进入正常控制面 |
| 设置密码后 | 需要登录才能控制播放 |
| 未登录 POST playback command | 返回 401 / 403 |
| 登录后 POST playback command | 正常转发到 `playbackd` |
| `/healthz` | 匿名可访问 |
| `/logs` / `/settings` / playback commands | 默认需要登录 |
| 资源超限的 HTTP / UDS / SSE 请求 | 返回明确错误，不拖挂服务 |
| physical recovery marker | 下次启动清理 auth state，重新 first setup |

### 24.2 Mode / Connectivity

| 验收项 | 必须结果 |
| --- | --- |
| 缺失 config 首次启动 | 默认进入 `local` |
| `mode=local` | 启动 Local Mode 播放栈 |
| `mode=bridge` | 只进入 V1 placeholder，不启动 bridge daemons |
| 设置页取消 mode 切换 | 不写入 committed config |
| 设置页确认 mode 切换 | 写入 config，并要求立即重启 |
| `interface_mode` 切换 | 标记重启生效 |
| 播放态 | 不持续 Bluetooth provisioning / advertising |

### 24.3 Playback / Queue / History

| 验收项 | 必须结果 |
| --- | --- |
| `PLAY_REQUEST_ACCEPTED` | 进入 `pre_quiet` |
| `PLAYBACK_STARTED` | 只在第一帧写入 ALSA 后发出 |
| `pause` | 进入 `paused`，主按钮变 `播放` |
| `resume` | 恢复播放，主按钮变 `暂停` |
| `stop` | 停止输出但保留队列 |
| `shuffle` | 由 `playbackd` 生成并持久化 `play_order` |
| `repeat_mode` | 普通播放命令不应隐式重置 |
| `play_history` | 只替换当前播放位，保留后续队列 |
| WebUI 历史 | 只显示已到达曲目，最近播放在上 |
| 首页 | 不提前暴露 shuffle 后续顺序 |
| release profile remote API | 不能触发任意绝对路径播放 |

### 24.4 Quiet Mode

| 验收项 | 必须结果 |
| --- | --- |
| 收到 `PLAY_REQUEST_ACCEPTED` | `sessiond` 进入 `pre_quiet` |
| 收到 `PLAYBACK_STARTED` | `sessiond` 进入 `active` |
| active 播放中 | media scan / artwork worker / Bluetooth provisioning 不持续活动 |
| 收到 `PLAYBACK_STOPPED` | 退出 Quiet Mode 并恢复必要 setup 能力 |
| 输出链 `PLAYBACK_FAILED` | 退出 Quiet Mode |
| content error hold | 可保持静默等待 auto-skip |

### 24.5 Error Recovery

| 验收项 | 必须结果 |
| --- | --- |
| DAC 不可用 | fail-stop，不自动切下一首 |
| ALSA 打不开 | fail-stop，不提前发 `PLAYBACK_STARTED` |
| 当前文件损坏 | 进入 `quiet_error_hold` |
| content error hold | UI 静态提示，不做高频倒计时 |
| 6 秒后 | 自动尝试下一首可播放曲 |
| 连续 content auto-skip | 上限 3 次 |
| 用户显式操作 | 取消 auto-skip 等待 |
| TF / USB 离线 | UI 明确提示，不默认重扫或重建队列 |

### 24.6 Library / WebUI

| 验收项 | 必须结果 |
| --- | --- |
| 无封面专辑 | 显示默认方形封面占位 |
| 长曲名 / 长路径 | 单行省略，不撑高 mini player |
| 大曲库列表 | 有 limit / cursor 或等价分页 |
| unsupported format | 后端与 UI 判断一致 |
| offline media | 可显示缓存元数据，但不可播放 |
| 页面重构 | 不改变 Rust 服务状态机 |

### 24.7 Rootfs / Runtime

| 验收项 | 必须结果 |
| --- | --- |
| `/run/lumelo/` | 只放 socket 与轻量运行时标志 |
| `/var/lib/lumelo/` | 放 queue/history/library/auth 等持久状态 |
| `queue.json` | 唯一队列恢复入口 |
| 关键持久化写入 | atomic write + file fsync + rename + parent directory fsync |
| 重启后 | 恢复队列但进入 `stopped` |
| `media-indexd` crash | 不影响当前播放 |
| `controld` crash | 不影响 `playbackd` |
| release image SSH | 默认关闭 |

## 25. 文档使用规则

后续开发按以下顺序使用文档：

1. 本文件判断长期产品语义和验收标准。
2. [Milestone_Progress_Document.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Milestone_Progress_Document.md) 判断当前阶段优先级、需求池和 bug 池。
3. [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md) 记录实际完成内容和验证结果。

文档维护规则：

- 本文件不维护每日开发流水。
- 本文件不复制 bug 池。
- 修改长期产品语义时改本文件。
- 修改阶段排期、优先级和开发大纲时改 `Milestone_Progress_Document.md`。
- 修改实际进展和验证事实时改 `Development_Progress_Log.md`。
