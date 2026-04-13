# AI Review Part 01

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `.gitignore`

- bytes: 226
- segment: 1/1

~~~text
.DS_Store
._*
.idea/
.vscode/

services/rust/target/
services/controld/controld
apps/android-provisioning/.gradle/
apps/android-provisioning/build/
apps/android-provisioning/local.properties
dist/
out/
tmp/
__pycache__/
*.pyc
~~~

## `README.md`

- bytes: 12434
- segment: 1/1

~~~md
# Lumelo V1 Skeleton

This repository now contains the first implementation skeleton for the
Lumelo local audio system described in the project docs.

`Lumelo` is the product name. `RK3399 / NanoPC-T4` remains the current V1
hardware platform.

Current focus:

- `base/`: rootfs overlay, board-support placeholders, and systemd units
- `services/rust/`: Rust workspace for `playbackd`, `sessiond`,
  `media-indexd`, and shared crates
- `services/controld/`: Go control-plane skeleton with embedded SSR assets
- `packaging/`: image, recovery, update, and systemd packaging placeholders
- `scripts/`: top-level project scripts only

The code in this skeleton is intentionally minimal. It exists to lock in the
repo layout, runtime paths, service boundaries, and build entrypoints before
the real playback and indexing logic is implemented.

## Runtime Paths

Production defaults remain:

- `playback_cmd.sock`: `/run/lumelo/playback_cmd.sock`
- `playback_evt.sock`: `/run/lumelo/playback_evt.sock`

The rootfs overlay now also pins that production layout in systemd and
`tmpfiles.d`, so deployed images create and preserve the shared runtime
directory under `/run/lumelo` without relying only on application defaults.

For local macOS development, prefer overriding the runtime directory instead of
changing debug/release constants:

```bash
export LUMELO_RUNTIME_DIR=/tmp/lumelo
export LUMELO_STATE_DIR=/tmp/lumelo-state
```

`playbackd` and `controld` will then use:

- `/tmp/lumelo/playback_cmd.sock`
- `/tmp/lumelo/playback_evt.sock`
- `/tmp/lumelo/quiet_mode`
- `/tmp/lumelo-state/queue.json`
- `/tmp/lumelo-state/history.json`
- `/tmp/lumelo-state/library.db`

If you ever need a one-off override, the per-socket variables still work:

- `PLAYBACK_CMD_SOCKET_PATH`
- `PLAYBACK_EVT_SOCKET_PATH`
- `PLAYBACK_HISTORY_STATE_PATH`
- `LIBRARY_DB_PATH`
- `ARTWORK_CACHE_DIR`
- `CONTROLD_PLAYBACK_CMD_SOCKET`
- `CONTROLD_PLAYBACK_EVT_SOCKET`

The dev scripts also default `CARGO_TARGET_DIR` and `GOCACHE` into `/tmp` so
local builds do not depend on filesystem features missing from external drives.

For the current minimum library/index worker, use:

```bash
./scripts/dev-media-indexd.sh ensure-schema
./scripts/dev-media-indexd.sh seed-demo
./scripts/dev-media-indexd.sh scan-dir /path/to/music
```

That will create or update `library.db` under `${LUMELO_STATE_DIR}` and seed a
small demo album for local validation.

`scan-dir` is now the first real library pass. It recursively scans a local
directory tree, records folders as `directories`, groups playable files into
 albums, and writes real `volumes / albums / tracks` rows into `library.db`.

Current `scan-dir` scope is intentionally small:

- audio file discovery only
- tag-first album/title/artist parsing with directory fallback
- `album artist` first, then `artist`, then `Unknown Artist`
- basic `track / disc / year / genre / duration / sample_rate / bit_depth`
- same-directory cover discovery for `folder.jpg` then `cover.jpg`
- source artwork cached into `${ARTWORK_CACHE_DIR}/source/...`
- `thumb/320` JPEG generation into `${ARTWORK_CACHE_DIR}/thumb/320/...`
- `artwork_refs` plus album/track `cover_ref_id` linking
- original artwork `width / height` plus `thumb_rel_path` backfilled into `artwork_refs`
- folder fallback when album tags are incomplete
- no incremental diffing yet
- no embedded artwork parsing yet
- no image serving endpoint yet

With `controld` running, the current minimum library page is available at:

- `http://127.0.0.1:18080/library`

It reads `library.db` directly and currently shows:

- library counts
- indexed volumes
- recent albums
- recent tracks

## OrbStack Dev Flow

OrbStack is now the recommended Linux validation layer on macOS for this repo.
After installing and opening OrbStack once, use the host-side bootstrap script:

```bash
./scripts/orbstack-bootstrap-lumelo-dev.sh
```

That script creates a Debian 12 `arm64` machine named `lumelo-dev`, installs the
minimum Debian packages we need for this project, installs Rust with `rustup`,
installs Go for Linux `arm64`, and prints the follow-up commands for:

- opening a shell in `lumelo-dev`
- running Rust tests with runtime/build outputs in `/tmp`
- running Go tests with `GOCACHE` in `/tmp`
- verifying the current systemd unit files

If OrbStack reports `Stopped`, finish the one-time GUI onboarding in
`/Applications/OrbStack.app` first and rerun the script.

## T4 Smoke Image

The first T4 bring-up path is now pinned to:

- official `FriendlyELEC NanoPC-T4` SD image family
- `Debian trixie core`
- `kernel 4.19.y`
- `u-boot v2017.09`

Lock metadata lives at:

- [packaging/image/t4-smoke-base.toml](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/t4-smoke-base.toml)

The first remaster entrypoint is:

```bash
sudo ./scripts/build-t4-smoke-image.sh \
  --base-image /path/to/rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz \
  --output /path/to/lumelo-t4-smoke.img
```

This is a smoke-image shortcut for bring-up only. It reuses the official board
boot chain and remasters the rootfs partition with Lumelo binaries and overlay.

First actual image artifact built on `2026-04-07 00:16` (Asia/Shanghai):

- base image: `out/t4-smoke/rk3399-sd-debian-trixie-core-4.19-arm64-20260319.img.gz`
- output image: [out/t4-smoke/lumelo-t4-smoke-20260406.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-smoke/lumelo-t4-smoke-20260406.img)
- sha256: [out/t4-smoke/lumelo-t4-smoke-20260406.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-smoke/lumelo-t4-smoke-20260406.img.sha256)

Minimal image verification already passed:

- rootfs `p8` contains `playbackd / sessiond / media-indexd / controld`
- `local-mode.target` is enabled in `multi-user.target.wants`
- build marker exists in `/etc/lumelo/smoke-build.txt`

## T4 Lumelo Rootfs Image

The first `Lumelo-defined rootfs` image is now also available.

It keeps the FriendlyELEC `p1-p7` board-support partitions, but replaces the
entire userspace above the kernel with a rootfs assembled by `mmdebstrap` and
then layered with `Lumelo` binaries and `base/rootfs/overlay`.

Build entrypoint:

```bash
sudo ./scripts/build-t4-lumelo-rootfs-image.sh \
  --board-base-image /path/to/rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz \
  --output /path/to/lumelo-t4-rootfs.img
```

For a headless bring-up image with SSH enabled, inject a public key explicitly:

```bash
ENABLE_SSH=1 SSH_AUTHORIZED_KEYS_FILE=/path/to/id_ed25519.pub \
  sudo ./scripts/build-t4-lumelo-rootfs-image.sh \
    --board-base-image /path/to/rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz \
    --output /path/to/lumelo-t4-rootfs-ssh.img
```

Convenience wrapper:

```bash
sudo ./scripts/build-t4-ssh-bringup-image.sh \
  --ssh-authorized-keys /path/to/id_ed25519.pub
```

The SSH bring-up path never sets a default password; after flashing, log in
with `ssh root@<T4_IP>` using the injected key.

First artifact built on `2026-04-07 01:20` (Asia/Shanghai):

- image: [out/t4-rootfs/lumelo-t4-rootfs-20260407.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260407.img)
- sha256: [out/t4-rootfs/lumelo-t4-rootfs-20260407.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260407.img.sha256)
- partition layout:
  - `p8 rootfs = 1G`
  - `p9 userdata = 128M`

Minimal verification already passed:

- image size dropped from `7.3G` smoke raw image to `1.3G`
- `p8 rootfs` reports `312M used / 646M free`
- `/etc/os-release` inside the image is `Debian 13 (trixie)`
- next bring-up images include `/usr/bin/lumelo-t4-report` for one-command
  boot, network, service, WebUI, and ALSA diagnostics
- `/etc/lumelo/image-build.txt` confirms the `t4-bringup` profile
- `FriendlyELEC` kernel modules `4.19.232` are present under `/lib/modules`

Do not use the first `20260407` rootfs image for T4 boot validation. Hardware
testing showed that it did not boot: the T4 green LED stayed off. The cause was
that the first custom-rootfs builder recreated the GPT and copied only `p1-p7`,
but did not preserve the FriendlyELEC pre-partition RK3399 loader area before
`p1`.

Boot-fix artifact built on `2026-04-08 02:25` (Asia/Shanghai):

- image: [out/t4-rootfs/lumelo-t4-rootfs-20260408-bootfix.img](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260408-bootfix.img)
- sha256: [out/t4-rootfs/lumelo-t4-rootfs-20260408-bootfix.img.sha256](/Volumes/SeeDisk/Codex/Lumelo/out/t4-rootfs/lumelo-t4-rootfs-20260408-bootfix.img.sha256)
- sha256 value: `b191e21d8325a4c83bdf5ad3204a37d2660f12e54912dc98c0fe00b1f22237b7`
- size: `1.3G`
- offline verifier result: `0 failure(s), 0 warning(s)`
- pre-partition bootloader area check: official FriendlyELEC base and boot-fix
  image match for sectors `34..16383`

Offline image verification:

```bash
sudo ./scripts/verify-t4-lumelo-rootfs-image.sh \
  out/t4-rootfs/lumelo-t4-rootfs-20260408-bootfix.img
```

The `20260408-bootfix` image passes this check with `0 failure(s)` and
`0 warning(s)`.

The bring-up manifest now includes `alsa-utils`, so the next rebuilt image can
run `aplay` and `amixer` during the first real audio-device pass.
It also includes `/usr/bin/lumelo-audio-smoke`, a manual `speaker-test` wrapper
for the first short ALSA tone check, for example:

```bash
lumelo-audio-smoke hw:0,0
```

For no-SSH first boot checks, the current source also exposes the following
endpoint. It will be available in images rebuilt after this change:

```text
http://<T4_IP>:18080/healthz
```

The next rebuilt image will also include a Web log page for no-SSH debugging:

```text
http://<T4_IP>:18080/logs
http://<T4_IP>:18080/logs.txt
```

The page reads the current boot journal with `journalctl -b` on demand and
shows the latest 300 lines by default. The `.txt` endpoint is intended for easy
copy/paste when reporting T4 bring-up failures. This is a debug surface, not a
high-frequency polling loop or a separate logging daemon.

The next rebuilt image also starts the first T4-side Wi-Fi/Bluetooth
provisioning base:

- rootfs package manifest now includes `bluez`, `wpasupplicant`, `iw`,
  `rfkill`, and `wireless-regdb`
- Bluetooth is enabled at boot when the controller is available
- `/usr/bin/lumelo-bluetooth-provisioning-mode` asks BlueZ to power on,
  become discoverable, and become pairable
- `/usr/libexec/lumelo/bluetooth-wifi-provisiond` provides the first BlueZ
  GATT service for receiving Wi-Fi credentials
- `/usr/bin/lumelo-wifi-apply <ssid> <password>` writes a
  `wpa_supplicant@wlan0` config and restarts the Wi-Fi path
- `/etc/systemd/network/30-wireless-dhcp.network` gives wireless interfaces
  a DHCP profile

This is not the phone app yet. The BLE GATT service exists as a first
implementation, but still needs real T4 + Android validation.

Bring-up strategy after the first `20260408-bootfix` hardware pass:

- `20260408-bootfix` reached `lumelo login:`, so the pre-partition bootloader
  fix is effective enough to enter userspace
- the next debug image should default to console login `root/root`
- the next debug image should also keep `controld` on `0.0.0.0:18080`
- wired and wireless networkd profiles now keep DHCP with MAC-based client IDs,
  while disabling link-local addressing, LLMNR, and mDNS for quieter bring-up
- do not rebuild the next hardware image until the Web log page,
  Wi-Fi/Bluetooth provisioning base, and simplest phone APK path are ready

The interrupted `out/t4-rootfs/lumelo-t4-rootfs-console-20260408.img.partial`
stopped build artifact was deleted on request and must not be referenced as a
valid image.

The current Android provisioning APK status and roadmap are tracked in
[docs/Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md).

The current board/app provisioning contract is tracked in
[docs/Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md).

The first Android project skeleton is available at
[apps/android-provisioning](/Volumes/SeeDisk/Codex/Lumelo/apps/android-provisioning).
It is a native Android Java app without AndroidX/Compose dependencies for the
bring-up phase. It can scan for `Lumelo` BLE devices, connect to the Lumelo GATT
service, send Wi-Fi credentials, trigger apply, and display the returned status.
This workspace currently has Java but no local Gradle/Android SDK toolchain, so
APK assembly still needs Android Studio or a configured Android build host.
~~~

## `apps/android-provisioning/.gitignore`

- bytes: 41
- segment: 1/1

~~~text
._*
.gradle/
local.properties
app/build/
~~~

## `apps/android-provisioning/README.md`

- bytes: 3001
- segment: 1/1

~~~md
# Lumelo Android Provisioning MVP

This is the first Android-only provisioning app skeleton for Lumelo T4 bring-up.

Scope:

- scan for classic Bluetooth devices named `Lumelo`
- merge classic Bluetooth discovery results when the device name arrives late
- run a raw BLE scan with per-device UUID / manufacturer detail
- show scan summary counts for total / UUID matched / name matched devices
- connect to the Lumelo provisioning service over classic Bluetooth RFCOMM
- write Wi-Fi SSID/password JSON
- trigger apply
- prefill the current phone Wi-Fi SSID when Android exposes it
- automatically poll provisioning status for a short window after apply
- automatically open the Lumelo main interface inside the APK after provisioning reports `connected`
- show the status payload and WebUI URL reported by the T4
- manually re-read status and disconnect during bring-up retries
- open the reported WebUI, `/library`, `/provisioning`, `/logs`, and `/healthz` pages inside the APK shell
- keep and clear a small on-screen debug log for scan / connect / provisioning transitions
- export a plain-text diagnostic report for scan / connect / provisioning sessions
- show app version, build time, and git short SHA inside the setup screen

This app intentionally does not implement music browsing or playback control.

Build note:

- This workspace now includes a Gradle wrapper.
- The app is aligned to `AGP 8.13.2` and Android `36.1`, using `compileSdk { release(36) { minorApiLevel = 1 } }`.
- The preferred macOS workspace is the APFS sparsebundle path under `/Volumes/LumeloDev/...`.
- If you build directly from the `SeeDisk` `exFAT` mirror instead, keep Gradle cache, project cache, and Android build outputs under `/tmp` to avoid `._*` AppleDouble sidecar files breaking Android resource parsing.
- The first debug APK target remains `:app:assembleDebug`.
- A known-good local build pattern is:

```sh
export JAVA_HOME='/Applications/Android Studio.app/Contents/jbr/Contents/Home'
export PATH="$JAVA_HOME/bin:$PATH"
export GRADLE_USER_HOME='/tmp/lumelo-android-gradle-home'
export LUMELO_ANDROID_BUILD_ROOT='/tmp/lumelo-android-build'
./gradlew --project-cache-dir /tmp/lumelo-android-project-cache :app:assembleDebug
```

The classic Bluetooth provisioning contract, retained BLE diagnostic scope, and current credential security rules are documented in
[../../docs/Provisioning_Protocol.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Provisioning_Protocol.md).

The current APK structure, status, and follow-up roadmap are documented in
[../../docs/Android_Provisioning_App_Progress.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Android_Provisioning_App_Progress.md).

That document also defines the current APK artifact naming rule and records the
latest debug package path.

As of `2026-04-12`, the latest debug artifact is:

- [lumelo-android-provisioning-20260412-webviewpollfix-debug.apk](/Volumes/SeeDisk/Codex/Lumelo/out/android-provisioning/lumelo-android-provisioning-20260412-webviewpollfix-debug.apk)
~~~

## `apps/android-provisioning/app/build.gradle.kts`

- bytes: 1232
- segment: 1/1

~~~kotlin
import java.time.Instant

plugins {
    id("com.android.application")
}

val localBuildRoot = System.getenv("LUMELO_ANDROID_BUILD_ROOT") ?: "/tmp/lumelo-android-build"
val buildTimeUtc = Instant.now().toString()

fun shortSha(value: String?): String? {
    if (value.isNullOrBlank()) {
        return null
    }
    return value.trim().take(8)
}

val gitShaShort = shortSha(System.getenv("LUMELO_GIT_SHA_SHORT"))
    ?: shortSha(System.getenv("GIT_COMMIT"))
    ?: shortSha(System.getenv("GITHUB_SHA"))
    ?: "nogit"

layout.buildDirectory.set(file("$localBuildRoot/app"))

android {
    namespace = "com.lumelo.provisioning"
    compileSdk {
        version = release(36) {
            minorApiLevel = 1
        }
    }

    defaultConfig {
        applicationId = "com.lumelo.provisioning"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        buildConfigField("String", "BUILD_TIME_UTC", "\"$buildTimeUtc\"")
        buildConfigField("String", "GIT_SHA_SHORT", "\"$gitShaShort\"")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        buildConfig = true
    }
}
~~~

## `apps/android-provisioning/app/src/main/AndroidManifest.xml`

- bytes: 1490
- segment: 1/1

~~~xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-feature
        android:name="android.hardware.bluetooth_le"
        android:required="false" />

    <uses-permission
        android:name="android.permission.BLUETOOTH"
        android:maxSdkVersion="30" />
    <uses-permission
        android:name="android.permission.BLUETOOTH_ADMIN"
        android:maxSdkVersion="30" />
    <uses-permission android:name="android.permission.BLUETOOTH_SCAN" />
    <uses-permission android:name="android.permission.BLUETOOTH_CONNECT" />
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />
    <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.INTERNET" />

    <application
        android:allowBackup="false"
        android:label="Lumelo Setup"
        android:usesCleartextTraffic="true"
        android:theme="@style/AppTheme">
        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
        <activity
            android:name=".MainInterfaceActivity"
            android:exported="false" />
    </application>
</manifest>
~~~

## `apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ClassicBluetoothTransport.java`

- bytes: 14261
- segment: 1/1

~~~java
package com.lumelo.provisioning;

import android.annotation.SuppressLint;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothSocket;
import android.os.Handler;
import android.os.Looper;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.util.UUID;

final class ClassicBluetoothTransport {
    interface Listener {
        void onClassicConnected(BluetoothDevice device);

        void onClassicDisconnected(String message);

        void onClassicDeviceInfo(String payload);

        void onClassicStatus(String payload);

        void onClassicError(String message);

        void onClassicLog(String message);
    }

    private static final UUID SPP_UUID =
            UUID.fromString("00001101-0000-1000-8000-00805F9B34FB");
    private static final int RFCOMM_CHANNEL = 1;

    private final Listener listener;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    private BluetoothSocket socket;
    private BufferedWriter writer;
    private Thread readerThread;
    private volatile boolean disconnectRequested;
    private volatile ProvisioningSecurity.Session credentialSecuritySession;

    ClassicBluetoothTransport(Listener listener) {
        this.listener = listener;
    }

    synchronized boolean isConnected() {
        return socket != null && socket.isConnected();
    }

    @SuppressLint("MissingPermission")
    void connect(BluetoothDevice device) {
        disconnect();
        disconnectRequested = false;
        credentialSecuritySession = null;
        new Thread(() -> connectInBackground(device), "LumeloClassicConnect").start();
    }

    private void connectInBackground(BluetoothDevice device) {
        IOException lastFailure = null;
        BluetoothSocket candidate = null;
        lastFailure = null;

        candidate = tryConnectAttempt(
                "trying insecure RFCOMM",
                () -> device.createInsecureRfcommSocketToServiceRecord(SPP_UUID)
        );
        if (candidate == null) {
            lastFailure = lastAttemptFailure;
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to secure RFCOMM",
                    () -> device.createRfcommSocketToServiceRecord(SPP_UUID)
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to insecure RFCOMM channel 1",
                    () -> createChannelSocket(device, "createInsecureRfcommSocket", RFCOMM_CHANNEL)
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to secure RFCOMM channel 1",
                    () -> createChannelSocket(device, "createRfcommSocket", RFCOMM_CHANNEL)
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            String message = "Classic Bluetooth connect failed";
            if (lastFailure != null && lastFailure.getMessage() != null && !lastFailure.getMessage().isEmpty()) {
                message += ": " + lastFailure.getMessage();
            }
            postError(message);
            postDisconnected("Disconnected from T4.");
            return;
        }

        BufferedWriter connectedWriter;
        BufferedReader connectedReader;
        try {
            connectedWriter = new BufferedWriter(
                    new OutputStreamWriter(candidate.getOutputStream(), StandardCharsets.UTF_8)
            );
            connectedReader = new BufferedReader(
                    new InputStreamReader(candidate.getInputStream(), StandardCharsets.UTF_8)
            );
        } catch (IOException exception) {
            closeQuietly(candidate);
            postError("Classic Bluetooth stream setup failed: " + exception.getMessage());
            postDisconnected("Disconnected from T4.");
            return;
        }

        synchronized (this) {
            socket = candidate;
            writer = connectedWriter;
        }

        post(() -> listener.onClassicConnected(device));
        startReaderLoop(candidate, connectedReader);
        requestDeviceInfo();
        requestStatus();
    }

    private interface SocketFactory {
        BluetoothSocket create() throws Exception;
    }

    private IOException lastAttemptFailure;

    private BluetoothSocket tryConnectAttempt(String label, SocketFactory factory) {
        BluetoothSocket candidate = null;
        try {
            postLog("Classic Bluetooth connect: " + label);
            candidate = factory.create();
            candidate.connect();
            lastAttemptFailure = null;
            return candidate;
        } catch (Exception exception) {
            closeQuietly(candidate);
            lastAttemptFailure = toIoException(label, exception);
            postLog("Classic Bluetooth connect attempt failed (" + label + "): "
                    + lastAttemptFailure.getMessage());
            return null;
        }
    }

    private BluetoothSocket createChannelSocket(BluetoothDevice device, String methodName, int channel)
            throws Exception {
        Method method = BluetoothDevice.class.getMethod(methodName, int.class);
        Object result = method.invoke(device, channel);
        if (!(result instanceof BluetoothSocket)) {
            throw new IOException("Hidden RFCOMM method returned no BluetoothSocket");
        }
        return (BluetoothSocket) result;
    }

    private IOException toIoException(String label, Exception exception) {
        if (exception instanceof IOException) {
            return (IOException) exception;
        }
        Throwable cause = exception.getCause();
        if (cause instanceof IOException) {
            return (IOException) cause;
        }
        String message = exception.getMessage();
        if (message == null || message.isEmpty()) {
            message = label + " failed";
        }
        return new IOException(message, exception);
    }

    private void startReaderLoop(BluetoothSocket activeSocket, BufferedReader reader) {
        readerThread = new Thread(() -> readLoop(activeSocket, reader), "LumeloClassicRead");
        readerThread.start();
    }

    private void readLoop(BluetoothSocket activeSocket, BufferedReader reader) {
        try {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.trim().isEmpty()) {
                    continue;
                }
                handleIncomingLine(line);
            }
        } catch (IOException exception) {
            if (!disconnectRequested) {
                postError("Classic Bluetooth read failed: " + exception.getMessage());
            }
        } finally {
            boolean notifyDisconnect = !disconnectRequested;
            synchronized (this) {
                if (socket == activeSocket) {
                    socket = null;
                    writer = null;
                }
            }
            credentialSecuritySession = null;
            closeQuietly(activeSocket);
            if (notifyDisconnect) {
                postDisconnected("Disconnected from T4.");
            }
        }
    }

    private void handleIncomingLine(String line) {
        try {
            JSONObject message = new JSONObject(line);
            String type = message.optString("type");
            if ("hello".equals(type)) {
                try {
                    JSONObject security = message.optJSONObject("security");
                    if (security != null) {
                        credentialSecuritySession = ProvisioningSecurity.parseSession(
                                security.optString("session_id"),
                                security.optString("scheme"),
                                security.optString("dh_group"),
                                security.optString("server_nonce"),
                                security.optString("server_public_key")
                        );
                    } else {
                        credentialSecuritySession = null;
                    }
                    if (credentialSecuritySession != null) {
                        postLog("Classic Bluetooth credential security negotiated: "
                                + ProvisioningSecurity.CREDENTIAL_SCHEME);
                    }
                } catch (GeneralSecurityException securityException) {
                    credentialSecuritySession = null;
                    postError("Classic Bluetooth security negotiation failed: "
                            + securityException.getMessage());
                }
                postLog("Classic Bluetooth hello: " + line);
                return;
            }
            if ("ack".equals(type)) {
                postLog("Classic Bluetooth ack: " + message.optString("message"));
                return;
            }
            if ("error".equals(type)) {
                String errorMessage = message.optString("message");
                if (errorMessage.isEmpty()) {
                    errorMessage = line;
                }
                postError(errorMessage);
                return;
            }

            JSONObject payload = message.optJSONObject("payload");
            if ("device_info".equals(type) && payload != null) {
                post(() -> listener.onClassicDeviceInfo(payload.toString()));
                return;
            }
            if ("status".equals(type) && payload != null) {
                post(() -> listener.onClassicStatus(payload.toString()));
                return;
            }

            postLog("Classic Bluetooth message: " + line);
        } catch (JSONException exception) {
            postLog("Classic Bluetooth non-JSON line: " + line);
        }
    }

    void requestDeviceInfo() {
        sendSimpleCommand("device_info");
    }

    void requestStatus() {
        sendSimpleCommand("status");
    }

    void sendCredentials(String ssid, String password) {
        try {
            ProvisioningSecurity.Session securitySession = credentialSecuritySession;
            if (securitySession == null) {
                postError("Classic Bluetooth secure credential transport is unavailable. "
                        + "Update the T4 provisioning daemon before sending Wi-Fi credentials.");
                return;
            }

            ProvisioningSecurity.EncryptedPayload encryptedPayload =
                    ProvisioningSecurity.encryptCredentials(securitySession, ssid, password);
            JSONObject secureMessage = new JSONObject();
            JSONObject payload = new JSONObject();
            payload.put("scheme", encryptedPayload.scheme);
            payload.put("dh_group", encryptedPayload.dhGroup);
            payload.put("session_id", encryptedPayload.sessionId);
            payload.put("client_public_key", encryptedPayload.clientPublicKey);
            payload.put("client_nonce", encryptedPayload.clientNonce);
            payload.put("message_nonce", encryptedPayload.messageNonce);
            payload.put("ciphertext", encryptedPayload.ciphertext);
            payload.put("mac", encryptedPayload.mac);
            secureMessage.put("type", "wifi_credentials_encrypted");
            secureMessage.put("payload", payload);
            sendMessage(secureMessage);
            postLog("Classic Bluetooth credential transport: encrypted payload queued for SSID " + ssid);
            sendSimpleCommand("apply");
        } catch (JSONException | GeneralSecurityException exception) {
            postError("Failed to build Classic Bluetooth credential payload: " + exception.getMessage());
        }
    }

    void disconnect() {
        disconnectRequested = true;
        BluetoothSocket currentSocket;
        synchronized (this) {
            currentSocket = socket;
            socket = null;
            writer = null;
        }
        closeQuietly(currentSocket);
    }

    private void sendSimpleCommand(String type) {
        try {
            JSONObject message = new JSONObject();
            message.put("type", type);
            sendMessage(message);
        } catch (JSONException exception) {
            postError("Failed to build Classic Bluetooth command: " + type);
        }
    }

    private void sendMessage(JSONObject message) {
        BufferedWriter currentWriter;
        synchronized (this) {
            currentWriter = writer;
        }
        if (currentWriter == null) {
            postError("Classic Bluetooth session is not connected.");
            return;
        }

        try {
            currentWriter.write(message.toString());
            currentWriter.write("\n");
            currentWriter.flush();
            postLog("Classic Bluetooth send: " + message.optString("type"));
        } catch (IOException exception) {
            postError("Classic Bluetooth write failed: " + exception.getMessage());
            disconnect();
            postDisconnected("Disconnected from T4.");
        }
    }

    private void postDisconnected(String message) {
        post(() -> listener.onClassicDisconnected(message));
    }

    private void postError(String message) {
        post(() -> listener.onClassicError(message));
    }

    private void postLog(String message) {
        post(() -> listener.onClassicLog(message));
    }

    private void post(Runnable action) {
        mainHandler.post(action);
    }

    private void closeQuietly(BluetoothSocket currentSocket) {
        if (currentSocket == null) {
            return;
        }
        try {
            currentSocket.close();
        } catch (IOException ignored) {
        }
    }
}
~~~

