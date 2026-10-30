# Build matrix

| Target | Command | Verified |
|---|---|---|
| Linux (host) | `cargo test -p vault-core && cargo run -p obsidian-rust` | ✅ locally |
| Windows (cross-check) | `cargo check -p obsidian-rust-app --target x86_64-pc-windows-gnu` | ✅ locally (check only) |
| Windows (native) | `cargo run -p obsidian-rust` on Windows | ⏳ needs a Windows runner (CI covers `cargo check` on windows-latest) |
| Android (check) | `cargo check -p obsidian-rust-app --target aarch64-linux-android` | ✅ locally |
| Android (APK) | See `docs/ANDROID.md` (`cargo apk`) | ⏳ needs SDK/NDK + device |

Linux GUI needs a display server (X11/Wayland) and OpenGL;
the CLI (`kubo-notes`) needs none of that and is the fastest
way to validate a vault on any machine.
