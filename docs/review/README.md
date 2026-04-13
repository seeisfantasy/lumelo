# Lumelo AI Review Overview

- generated_at: 2026-04-13 07:42 UTC
- repo_root: `/Volumes/SeeDisk/Codex/Lumelo`
- included_text_files: 146
- omitted_binary_or_non_utf8_files: 1

## 1. 这是什么系统

Lumelo 当前是一个本地优先的网络音频系统，目标运行在 T4 板子上。它提供板端媒体索引、播放控制、Web UI、经典蓝牙配网，以及 Android 侧配网 App。当前主路径已经能完成：配网、进入 WebUI、索引真实曲库、展示封面、并在板子上真实输出音频。

## 2. 底座是什么

- 硬件底座：T4 板级平台，当前调试主要跑在 `sd` 系统上。
- 系统底座：Linux rootfs 镜像，基于 `base/rootfs/overlay` 和 `packaging/image` 组装。
- 服务编排：`systemd`。
- 网络与配网：`NetworkManager`、`wpa_supplicant`、BlueZ 经典蓝牙、RFCOMM 配网 daemon。
- 控制与页面：Go 写的 `controld` 提供 Web UI 和 HTTP API。
- 播放：Rust 写的 `playbackd`，当前通过 `aplay` 接 ALSA 输出。
- 索引：`media-indexd` 负责扫描介质、写入 `library.db`、生成封面缓存。

## 3. 当前架构

1. Android App 通过经典蓝牙和板子上的 `classic-bluetooth-wifi-provisiond` 建立配网会话。
2. 配网成功后，手机和板子在同网段，通过 `controld` 提供的 Web UI 继续操作。
3. `media-indexd` 扫描本地目录、外部介质或测试 fixture，把媒体元数据写入 `library.db`。
4. `controld` 读取 `library.db`，渲染首页、曲库页、封面缩略图和配网页。
5. `playbackd` 通过 Unix socket 接收播放命令，解析媒体，最后用 ALSA 输出。
6. `lumelo-media-import` 和 `lumelo-media-smoke` 是当前板端验证与导入的关键 helper。

## 4. 特色功能

- 经典蓝牙配网已经支持加密凭据传输，并移除了明文回退。
- 板端 `wpa_supplicant` 落盘已改为 `psk=<64hex>`，不再明文落盘 Wi-Fi 密码。
- 真曲库索引和封面缩略图已经贯通到 `/library` 页面。
- `wav + m4a/aac + flac + mp3 + ogg` 已在真机上完成播放验证。
- 外部媒体已有导入入口，且在无真介质条件下补了一轮模拟块设备导入验证。
- 已有正式板端回归命令覆盖：播放回归、批量扫描回归、坏文件边界、`playbackd` 重启恢复。

## 5. 当前已知未闭环

- 真 TF / USB 介质在场下的热插入 / 热拔出闭环。
- 整机重启后的状态回归。
- 坏文件是否要在索引层直接过滤，避免出现在用户曲库里。
- 调试 `sd` 系统目前仍需人工按键选择启动，不是默认启动介质。

## 6. 顶层目录结构

```text
.
├── .gitignore
├── README.md
├── apps
│   └── android-provisioning
│       ├── .gitignore
│       ├── README.md
│       ├── app
│       │   ├── build.gradle.kts
│       │   └── src
│       │       └── main
│       │           ├── AndroidManifest.xml
│       │           ├── java
│       │           │   └── com
│       │           │       └── lumelo
│       │           │           └── provisioning
│       │           │               ├── ClassicBluetoothTransport.java
│       │           │               ├── MainActivity.java
│       │           │               ├── MainInterfaceActivity.java
│       │           │               └── ProvisioningSecurity.java
│       │           └── res
│       │               └── values
│       │                   └── styles.xml
│       ├── build.gradle.kts
│       ├── gradle
│       │   └── wrapper
│       │       └── gradle-wrapper.properties
│       ├── gradlew
│       ├── gradlew.bat
│       └── settings.gradle.kts
├── base
│   ├── README.md
│   ├── board-support
│   │   └── friendly
│   │       └── README.md
│   └── rootfs
│       ├── hooks
│       │   ├── README.md
│       │   └── t4-bringup-postbuild.sh
│       ├── manifests
│       │   ├── README.md
│       │   └── t4-bringup-packages.txt
│       └── overlay
│           ├── etc
│           │   ├── NetworkManager
│           │   │   ├── NetworkManager.conf
│           │   │   └── conf.d
│           │   │       ├── 12-managed-wifi.conf
│           │   │       ├── 99-unmanaged-wlan1.conf
│           │   │       └── disable-random-mac-during-wifi-scan.conf
│           │   ├── bluetooth
│           │   │   └── main.conf
│           │   ├── dbus-1
│           │   │   └── system.d
│           │   │       └── org.lumelo.provisioning.conf
│           │   ├── lumelo
│           │   │   ├── config.toml
│           │   │   └── sessiond.env
│           │   ├── network
│           │   │   └── interfaces
│           │   ├── systemd
│           │   │   ├── network
│           │   │   │   ├── 20-wired-dhcp.network
│           │   │   │   └── 30-wireless-dhcp.network
│           │   │   ├── resolved.conf.d
│           │   │   │   └── lumelo.conf
│           │   │   └── system
│           │   │       ├── auth-recovery.service
│           │   │       ├── bluetooth.service.d
│           │   │       │   ├── 10-lumelo-rfkill-unblock.conf
│           │   │       │   └── 20-lumelo-uart-attach.conf
│           │   │       ├── bridge-mode.target
│           │   │       ├── controld.service
│           │   │       ├── local-mode.target
│           │   │       ├── lumelo-bluetooth-provisioning.service
│           │   │       ├── lumelo-bluetooth-uart-attach.service
│           │   │       ├── lumelo-media-import@.service
│           │   │       ├── lumelo-media-reconcile.service
│           │   │       ├── lumelo-ssh-hostkeys.service
│           │   │       ├── lumelo-wifi-provisiond.service
│           │   │       ├── media-indexd.service
│           │   │       ├── playbackd.service
│           │   │       ├── sessiond.service
│           │   │       └── ssh.service.d
│           │   │           └── 10-lumelo-hostkeys.conf
│           │   ├── udev
│           │   │   └── rules.d
│           │   │       └── 90-lumelo-media-import.rules
│           │   └── wpa_supplicant
│           │       └── wpa_supplicant-wlan0.conf
│           └── usr
│               ├── bin
│               │   ├── lumelo-audio-smoke
│               │   ├── lumelo-bluetooth-provisioning-mode
│               │   ├── lumelo-media-import
│               │   ├── lumelo-media-smoke
│               │   ├── lumelo-t4-report
│               │   └── lumelo-wifi-apply
│               ├── lib
│               │   └── tmpfiles.d
│               │       └── lumelo.conf
│               ├── libexec
│               │   └── lumelo
│               │       ├── auth-recovery
│               │       ├── bluetooth-uart-attach
│               │       ├── bluetooth-wifi-provisiond
│               │       └── classic-bluetooth-wifi-provisiond
│               └── share
│                   └── lumelo
│                       └── default_config.toml
├── docs
│   ├── AI_Handoff_Memory.md
│   ├── Android_Provisioning_App_Progress.md
│   ├── Development_Environment_README.md
│   ├── Development_Progress_Log.md
│   ├── Product_Development_Manual.md
│   ├── Provisioning_Protocol.md
│   ├── README.md
│   ├── T4_Bringup_Checklist.md
│   ├── T4_WiFi_Golden_Baseline.md
│   └── archive
│       ├── Android_Provisioning_App_MVP.md
│       ├── Real_Device_Findings_20260412_v15.md
│       ├── Repo_Rename_To_Lumelo_Checklist.md
│       ├── T4_moode_port_blueprint.md
│       ├── V1_Local_Mode_Function_and_Service_Spec.md
│       └── V1_Technical_Architecture_Proposal.md
├── examples
│   ├── sysctl
│   │   └── 90-t4-audio.conf.sample
│   └── systemd
│       ├── background-service.override.conf.sample
│       └── mpd.override.conf.sample
├── fixtures
│   └── README.md
├── packaging
│   ├── README.md
│   ├── image
│   │   ├── README.md
│   │   ├── t4-lumelo-rootfs-base.toml
│   │   └── t4-smoke-base.toml
│   ├── recovery
│   │   └── README.md
│   ├── systemd
│   │   └── README.md
│   └── update
│       └── README.md
├── scripts
│   ├── README.md
│   ├── build-ai-review-docs.py
│   ├── build-t4-lumelo-rootfs-image.sh
│   ├── build-t4-smoke-image.sh
│   ├── build-t4-ssh-bringup-image.sh
│   ├── compare-t4-wireless-golden.sh
│   ├── deploy-t4-runtime-update.sh
│   ├── dev-controld.sh
│   ├── dev-media-indexd.sh
│   ├── dev-playbackd.sh
│   ├── dev-sessiond.sh
│   ├── dev-up.sh
│   ├── mount-lumelodev-apfs.sh
│   ├── orbstack-bootstrap-fono-dev.sh
│   ├── orbstack-bootstrap-lumelo-dev.sh
│   ├── sync-to-lumelodev-apfs.sh
│   └── verify-t4-lumelo-rootfs-image.sh
├── services
│   ├── controld
│   │   ├── README.md
│   │   ├── cmd
│   │   │   └── controld
│   │   │       └── main.go
│   │   ├── go.mod
│   │   ├── go.sum
│   │   ├── internal
│   │   │   ├── api
│   │   │   │   ├── server.go
│   │   │   │   └── server_test.go
│   │   │   ├── auth
│   │   │   │   └── auth.go
│   │   │   ├── libraryclient
│   │   │   │   ├── client.go
│   │   │   │   └── client_test.go
│   │   │   ├── logclient
│   │   │   │   └── client.go
│   │   │   ├── playbackclient
│   │   │   │   ├── client.go
│   │   │   │   └── client_test.go
│   │   │   ├── provisioningclient
│   │   │   │   ├── client.go
│   │   │   │   └── client_test.go
│   │   │   ├── settings
│   │   │   │   ├── config.go
│   │   │   │   └── config_test.go
│   │   │   └── sshctl
│   │   │       └── controller.go
│   │   └── web
│   │       ├── embed.go
│   │       ├── static
│   │       │   └── css
│   │       │       └── app.css
│   │       └── templates
│   │           ├── index.html
│   │           ├── library.html
│   │           ├── logs.html
│   │           └── provisioning.html
│   └── rust
│       ├── Cargo.lock
│       ├── Cargo.toml
│       ├── README.md
│       └── crates
│           ├── artwork-cache
│           │   ├── Cargo.toml
│           │   └── src
│           │       └── lib.rs
│           ├── ipc-proto
│           │   ├── Cargo.toml
│           │   └── src
│           │       └── lib.rs
│           ├── media-indexd
│           │   ├── Cargo.toml
│           │   └── src
│           │       └── main.rs
│           ├── media-model
│           │   ├── Cargo.toml
│           │   └── src
│           │       └── lib.rs
│           ├── playbackd
│           │   ├── Cargo.toml
│           │   └── src
│           │       └── main.rs
│           └── sessiond
│               ├── Cargo.toml
│               └── src
│                   └── main.rs
└── tests
    └── README.md
```

## 7. 顶层体量概览

| Top Level | Files | Bytes |
| --- | ---: | ---: |
| `(root)` | 2 | 12660 |
| `apps` | 15 | 199748 |
| `base` | 48 | 147338 |
| `docs` | 15 | 326757 |
| `examples` | 3 | 889 |
| `fixtures` | 1 | 93 |
| `packaging` | 7 | 6838 |
| `scripts` | 17 | 78289 |
| `services` | 38 | 330548 |
| `tests` | 1 | 116 |

## 8. Review 分卷

| Review Doc | Bytes |
| --- | ---: |
| [AI_Review_Part_01.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_01.md) | 33494 |
| [AI_Review_Part_02.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_02.md) | 38295 |
| [AI_Review_Part_03.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_03.md) | 55193 |
| [AI_Review_Part_04.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_04.md) | 57976 |
| [AI_Review_Part_05.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_05.md) | 21024 |
| [AI_Review_Part_06.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_06.md) | 55932 |
| [AI_Review_Part_07.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_07.md) | 28014 |
| [AI_Review_Part_08.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_08.md) | 56472 |
| [AI_Review_Part_09.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_09.md) | 29123 |
| [AI_Review_Part_10.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_10.md) | 38240 |
| [AI_Review_Part_11.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_11.md) | 38226 |
| [AI_Review_Part_12.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_12.md) | 44110 |
| [AI_Review_Part_13.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_13.md) | 55624 |
| [AI_Review_Part_14.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_14.md) | 46215 |
| [AI_Review_Part_15.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_15.md) | 57987 |
| [AI_Review_Part_16.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_16.md) | 56653 |
| [AI_Review_Part_17.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_17.md) | 55863 |
| [AI_Review_Part_18.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_18.md) | 56707 |
| [AI_Review_Part_19.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_19.md) | 47958 |
| [AI_Review_Part_20.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_20.md) | 25043 |
| [AI_Review_Part_21.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_21.md) | 38229 |
| [AI_Review_Part_22.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_22.md) | 50430 |
| [AI_Review_Part_23.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_23.md) | 38267 |
| [AI_Review_Part_24.md](/Volumes/SeeDisk/Codex/Lumelo/docs/review/AI_Review_Part_24.md) | 53544 |

## 9. 省略文件

- `apps/android-provisioning/gradle/wrapper/gradle-wrapper.jar`: binary_or_non_utf8 (43705 bytes)
