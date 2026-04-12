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
