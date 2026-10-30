# Android build (APK)

The app crate already has the Android entry point:

- `crates/app/src/lib.rs`: `android_main` (NativeActivity) under
  `#[cfg(target_os = "android")]`
- `crates/app/Cargo.toml`: `[package.metadata.android]` with
  package `tech.kuboes.obsidianrust`, target `aarch64-linux-android`,
  and storage permissions. Note: on modern Android, a vault folder
  should come from the Storage Access Framework picker; the typed
  path in v0.1 is a desktop-first placeholder (see roadmap).

`cargo check --target aarch64-linux-android` passes for the app
crate (verified locally). That is a compile check, not an APK.

## To produce an APK (needs Android SDK + NDK)

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk

# Point cargo-apk at your SDK/NDK, e.g.:
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/<version>"

cd crates/app
cargo apk run --lib   # builds + installs to a connected device/emulator
# or: cargo apk build --lib --release
```

## Known Android gaps in v0.1 (honest list)

- No SAF vault picker yet; the desktop path field is shown as-is.
- Soft-keyboard behaviour in egui on Android is usable but not polished.
- No APK is published from CI yet; CI runs the Android
  `cargo check` only, because GitHub-hosted runners need an
  explicit NDK setup step for a real APK build.
