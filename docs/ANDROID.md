# Android build (APK)

The app crate already has the Android entry point:

- `crates/app/src/lib.rs`: `android_main` (NativeActivity) under
  `#[cfg(target_os = "android")]`
- `crates/app/Cargo.toml`: `[package.metadata.android]` with
  package `tech.kuboes.obsidianrust`, target `aarch64-linux-android`,
  min SDK 26, target SDK 34, and storage permissions.
  Note: on modern Android, a vault folder should come from the
  Storage Access Framework picker; the typed path in v0.1 is a
  desktop-first placeholder (see roadmap).

## v0.2 (2026-10-30, after "UIが操作できない / 互換性0" feedback)

Root causes found:

- v0.1 used the desktop 3-panel layout on a phone screen:
  panels crowded each other out and buttons were tiny.
- On modern Android, READ_EXTERNAL_STORAGE does not let an
  app open another app's folder, so opening a real vault
  failed and the note list stayed empty — which looks like
  "compatibility 0" even though the vault parser itself works.

Fixes in v0.2:

- Narrow windows (< 900 pt) now use one view at a time with
  large tab buttons: ノート一覧 / 編集 / リンク, larger text
  and 48–56 pt buttons. Desktop keeps the 3-panel layout.
- Android now auto-opens a sample vault inside the app's own
  files directory (`/data/data/tech.kuboes.obsidianrust/files/vault-sample`,
  no permission needed), so notes/links/backlinks work
  immediately, with no path typing.
- Manifest now also declares MANAGE_EXTERNAL_STORAGE, so a
  user who wants their own vault folder can grant all-files
  access in Settings > Apps > obsidian_rust > Permissions.
  The app cannot grant this itself; the error message in
  the app now says so.
- "サンプルvaultで試す" and "新規ノート" buttons added.

## Built APK (2026-10-30, verified locally)

- Debug APK via `cargo apk build --lib`: 85 MB, signed with the
  Android debug key, `apksigner verify` passes (v2/v3).
  SHA-256 (debug): `2d3ead9ae5e49ee9d979b960aeb2da36d3774a8fc52278d7b1a1a25169f6335b`
- Release-optimized APK, assembled from the release `.so`
  (cargo-apk's release flow needs a configured release keystore,
  so the release `.so` was zipaligned + signed with the debug
  key instead — installable, but **debug-signed, not a
  production key**; a future production-signed build will
  require uninstalling this one first): **2.4 MB**,
  v0.2 SHA-256: `dc4c799266bfd4fd75670374dd843a8e4be758e8548e3439f92eecc95c09b7d9`,
  `apksigner verify` passes (v2/v3), not debuggable,
  `arm64-v8a` only. (v0.1 release was `f4620c19…7ea1a07`.)
- Toolchain used: cargo-apk 0.10.0, Temurin JDK 17,
  build-tools 34.0.0, platform android-34, NDK r27c
  (27.2.12479018). Note: `sdkmanager` failed behind this VM's
  proxy, so the SDK packages were downloaded directly from
  `dl.google.com` instead.
- Not tested on a device or emulator here: no device was
  attached (`adb devices` empty) and this VM has no KVM for
  an emulator. On-device behaviour (keyboard, vault path)
  is unverified — please report what happens on your phone.

## To produce an APK (needs Android SDK + NDK)

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk

# Point cargo-apk at your SDK/NDK, e.g.:
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"

cd crates/app
cargo apk build --lib   # debug APK, signed with the debug key
# release: build the release .so, then zipalign + apksigner
# (see above — cargo-apk release needs a release keystore config)
```

## Known Android gaps in v0.1 (honest list)

- No SAF vault picker yet; the desktop path field is shown as-is.
- Soft-keyboard behaviour in egui on Android is usable but not polished.
- No APK is published from CI yet; CI runs the Android
  `cargo check` only, because GitHub-hosted runners need an
  explicit NDK setup step for a real APK build.
