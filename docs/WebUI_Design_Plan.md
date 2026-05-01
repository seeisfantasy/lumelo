# Lumelo WebUI Design Plan

## 1. 文档用途

本文件只负责维护 `V1 / Local Mode / headless` 阶段的 WebUI 视觉与信息架构方案。

目标：

- 在不改 `playbackd / sessiond / media-indexd` 语义的前提下
- 把当前 WebUI 收成更像 `HiFi music library + playback console`
- 让后续多轮 UI 重构尽量继续只影响：
  - `templates`
  - `CSS`
  - 浏览器侧 JS

它不替代：

- [Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md)
- [Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md)
- [WebUI_API_Contract_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/WebUI_API_Contract_Plan.md)

## 2. 当前功能边界

这份 UI 方案只围绕 Lumelo 当前真实已存在的功能设计。

当前纳入设计范围：

- 首页 `/`
  - 系统默认配置
  - provisioning 摘要
  - playback 状态
  - queue
  - playback controls
- 曲库 `/library`
  - `Volumes`
  - `Folders`
  - `Albums`
  - `Tracks`
  - `Play From Here`
  - playback boundary
  - now playing
- 配网 `/provisioning`
  - diagnostics
  - raw status JSON
- 日志 `/logs`
  - journal copy view

当前明确不纳入这轮设计：

- `radio / webradio`
- 播放进度条
- 歌词
- 全屏 `Now Playing`
- 艺术家页
- 歌单页
- 搜索 command palette
- 输出设备切换 UI
  - V1 只在设置页展示当前解码器
  - V2 多解码器选择计划看 [Audio_Output_Device_Plan.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Audio_Output_Device_Plan.md)

这些不是设计不重要，而是当前产品功能和页面结构还没到。

## 3. 外部参考结论

### 3.1 官方资料

- [moOde GitHub](https://github.com/moode-player/moode)
  - 官方强调它是一个“beautifully designed and responsive user interface”，同时保留 `Audiophile-grade features`
  - Lumelo 应借它的：
    - HiFi 设备控制感
    - 响应式 Web 控制台气质
  - 不应照搬它偏工具化、偏旧式播放器的观感
- [Apple HIG: Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars)
  - 官方建议用 sidebar 展示同级主区域；当层级更深时，使用 split view 展示导航、内容和细节
- [Apple HIG: Split views](https://developer.apple.com/design/human-interface-guidelines/split-views)
  - 官方明确 split view 适合同时展示：
    - leading navigation
    - content list
    - detail / inspector
- [Apple Music MiniPlayer on the web](https://support.apple.com/guide/music-web-review/use-music-miniplayer-apdm333a63ec/web)
  - 官方交互里，MiniPlayer 仍保留：
    - playback controls
    - Up Next
    - compact player state
  - 这很适合 Lumelo 当前“底部常驻播放条，不做进度条”的方向
- [myMPD Feature matrix](https://jcorporation.github.io/myMPD/060-references/feature-matrix/)
  - 说明它作为功能基线很适合参考：
    - queue save / append / replace
    - queue sorting
    - playlist insert
    - `add after current`
- [myMPD Webserver URIs](https://jcorporation.github.io/myMPD/060-references/webserver-uris/)
  - 说明 album art / folder art / playlist art / API endpoint 是核心 UI 基础设施
- [myMPD Pictures](https://jcorporation.github.io/myMPD/060-references/pictures/)
  - 说明图片和缩略图是 music UI 的一等公民，不该把曲库页面做成纯文本文件管理器
- [QQ音乐 App Store 页面](https://apps.apple.com/cn/app/qq%E9%9F%B3%E4%B9%90-%E5%90%AC%E6%88%91%E6%83%B3%E5%90%AC/id414603431)
  - 官方对外描述里强调：
    - 强大的搜索
    - 歌手 / 专辑 / 歌单等多种入口
    - 中文用户熟悉的音乐浏览路径
  - Lumelo 可借它的：
    - 中文音乐产品的亲和层级
    - 专辑 / 歌曲卡片的可读性
  - 不借它的：
    - 平台运营流
    - 推荐流
    - 电台入口

### 3.2 用户分享与社区反馈

- [Reddit: Tired of Volumio](https://www.reddit.com/r/audiophile/comments/13vae8z)
  - 用户反馈 `moOde` 很强，但 web interface “weird”
  - 这支持 Lumelo 不要直接把 `moOde` 当视觉模板
- [Reddit: Turning a spare pi into a Spotify and web radio play?](https://www.reddit.com/r/raspberry_pi/comments/1l747gp)
  - 用户反馈 `moOde` “web interface is not my fav design but always works”
  - 这说明：
    - 功能和稳定性值得借
    - 视觉层需要二次设计

## 4. Lumelo 的 UI 定义

这轮把 Lumelo WebUI 定义成：

- `Quiet HiFi Console`

一句话描述：

- 一个面向本地音乐库和 headless 播放系统的安静型 Web 控制台
- 它不是流媒体内容门户
- 也不是拟物播放器
- 而是：
  - 本地曲库浏览
  - 当前播放控制
  - 队列管理
  - 设备 / bring-up 诊断

### 4.1 2026-04-19 用户反馈后的纠偏

当前这版 `Quiet HiFi Console` 已经通过 `runtime update` 部署到 live `T4`：

- `http://192.168.1.110:18080/`
- `http://192.168.1.110:18080/library`
- `http://192.168.1.110:18080/provisioning`
- `http://192.168.1.110:18080/logs`

但用户对 live 页面给出的反馈很明确：

- 当前布局仍然更像：
  - dashboard
  - diagnostics console
- 还不像一个真正的音乐播放器

因此后续方向不应继续加重 “control console / diagnostics shell” 气质，而应改成：

- `music player first, diagnostics second`

这条纠偏不改变当前功能边界：

- 不加假的：
  - 进度条
  - `radio`
  - 歌词
  - 全屏 `Now Playing`
  - 尚不存在的 `Settings`
- 只重排现有：
  - artwork
  - now playing
  - queue
  - album-first library
  - diagnostics hierarchy

## 5. 信息架构

### 5.1 当前保留的主导航

- `Home`
- `Library`
- `Provisioning`
- `Logs`

当前不提前加进主导航：

- `Radio`
- `Artists`
- `Playlists`
- `Settings`

原因：

- 这些入口当前没有完整页面或没有稳定功能支持
- 不做假导航

### 5.2 页面角色

- `Home`
  - 角色：music-player-first dashboard
  - 重点：
    - `Now Playing`
    - artwork
    - queue
    - 最近加入 / 继续播放这类音乐入口
  - diagnostics 只保留为次级信息，不再做首页主视觉
- `Library`
  - 角色：album wall + album detail
  - 重点：
    - album-first browse
    - 当前专辑 / 当前上下文
    - 更像专辑曲目表的 tracklist
  - folder / volume / technical context 继续保留，但不主导页面气质
- `Provisioning`
  - 角色：diagnostic page
  - 重点：bring-up 与状态说明
- `Logs`
  - 角色：copy-friendly diagnostic page
  - 重点：日志提取，不做数据可视化

## 6. 版式

### 6.1 桌面端

当前推荐固定为：

- 左侧导航：`220px`
- 中间内容区：`minmax(0, 1fr)`
- 右侧信息栏：`320px ~ 340px`
- 底部 Mini Player：`76px ~ 84px`

其中：

- `Home`
  - 使用三栏
- `Library`
  - 使用三栏
- `Provisioning / Logs`
  - 使用两栏：
    - sidebar
    - main content

### 6.2 移动端

当前不强行做复杂 responsive shell。

这轮移动端目标只有：

- 卡片纵向堆叠
- 顶部横向导航继续可用
- 底部 Mini Player 在 `Home / Library` 继续可用

## 7. 视觉方向

### 7.1 主主题

采用深色高级灰：

- background: `#0d1117 ~ #11161d`
- primary card: `#171c24`
- secondary card: `#1d2430`
- line: `rgba(255,255,255,0.08)`
- primary text: `#f3f5f7`
- secondary text: `#9ba6b2`
- tertiary text: `#6c7885`
- accent: 暖金铜色 + 受控绿色状态色

原则：

- 不用纯黑
- 不用高饱和霓虹
- 不用重阴影
- 用层级、间距、边框和弱光泽做高级感

### 7.2 曲库视觉

- `Albums` 以唱片墙方式展示
- `Tracks / Folders / Volumes` 保持清晰列表
- 当前播放项只做：
  - 左侧 accent line
  - 轻微背景高亮
  - 小型状态 pill

### 7.3 组件原则

- 按钮：
  - 主按钮：圆角胶囊、accent 实底
  - 次按钮：暗底描边
- 卡片：
  - `16px ~ 20px` 圆角
  - 1px 半透明描边
  - 很轻的阴影
- 图标：
  - 当前阶段不额外引入 icon library
  - 先用文字 + 布局层级解决可读性

## 8. 交互原则

- `Home` 不是内容流首页
  - 但第一眼必须像在进入音乐播放器
  - 不是在进入 ops dashboard
- `Library` 不是后台文件管理器
  - 是 album-first browse surface
  - 默认阅读顺序应先是：
    - album art
    - album / artist
    - tracklist
  - 不是：
    - debug 状态
    - 文件路径
    - 技术字段
- `Provisioning / Logs` 不做 fancy UI
  - 重点是可读、可复制、可诊断
- 不为了“像 Apple Music”去加当前没有的：
  - 进度条
  - 歌词
  - 电台
  - 流媒体推荐流

## 9. 本轮实现范围

### Phase 1

- 重做全局 dark shell
- `Home` 改成 dashboard
- `Library` 改成 HiFi music wall + right rail
- `Provisioning / Logs` 改成同视觉体系的 diagnostics pages
- `Home / Library` 补底部 Mini Player
  - 仅使用当前已存在的 playback controls
  - 不做进度条
- `Home` 第二轮 refinement
  - 加入更音乐化的 overview strip
  - 把 stable defaults 与 raw runtime paths 分层显示
- `Library` 第二轮 refinement
  - 提前显示当前 browse context summary
  - 清理专辑卡片里偏 debug 的 artwork 文案
- `Home` 第三轮 refinement
  - `Playback Engine` 改成更清楚的 live status card
  - `Transport Controls` 改成 control surface
  - `Up Next` 增加 queue summary chips
- `Library` 第三轮 refinement
  - `Tracks` 区改得更像专辑内 track list
  - 用 metadata chips 代替更生硬的技术文本拼接
  - 把 `track uid` 收成次级 debug 信息
- `Library` 第四轮 refinement
  - `Tracks` 正式吃进已有的 `track_no / disc_no`
  - album card 不再露 artwork 文件名
  - artwork 存在时只提示 `Artwork linked`
- `Provisioning / Logs` refinement
  - 补 overview tiles
  - 把快速动作和原始诊断内容分层
  - 保持 copy-friendly，不做 fancy analytics UI

### Phase 1.5：`player-first` 纠偏

这一阶段不新增功能，只纠正“看起来不像音乐播放器”的问题。

下一轮重点应该是：

- `Home`
  - 把：
    - artwork
    - current track
    - queue
    - recent albums
    放到主视觉
  - 把 defaults / runtime / diagnostics 下沉成次级卡片
- `Library`
  - 把 album wall 和当前专辑 detail 做成主阅读路径
  - 让 tracklist 更像专辑页，而不是数据库浏览器
- `Provisioning / Logs`
  - 继续保留 diagnostics 角色
  - 不再反向影响 `Home / Library` 的主视觉语气

### 暂缓

- 动态专辑氛围色
- 真 `Settings` 页面
- 搜索 command palette
- 全屏 `Now Playing`
- 更多封面 / artwork 动效

## 10. 与 API contract 的关系

这份 UI 方案建立在当前已落地的 stable contract 上：

- `GET /api/v1/playback/status`
- `GET /api/v1/playback/queue`
- `GET /api/v1/playback/events`
- `GET /api/v1/library/snapshot`
- `POST /api/v1/playback/commands`
- `POST /api/v1/library/commands`

原则：

- 新视觉优先消费这批 contract
- 不反向把页面样式要求塞回 Rust 服务
