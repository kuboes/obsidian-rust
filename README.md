# obsidian-rust

Obsidian-compatible notes, rewritten from scratch in Rust.
Linux / Windows / Android, one codebase.

> **Not a fork of Obsidian's code, and not affiliated with Obsidian.**
> Obsidian itself is closed-source, so there is no source tree to fork.
> This is a clean-room rewrite that speaks the same *vault format*:
> a folder of plain `.md` files. Open the same folder in Obsidian
> and in this app, interchangeably — no conversion, no database,
> no lock-in.

## What works in v0.1

- Open any Obsidian vault folder (it is never rewritten on open)
- Plain-Markdown editing, saved back as plain `.md`
- Frontmatter: `title`, `tags`, `aliases` (inline and list forms)
- `[[wikilinks]]`, `[[Note#Heading|Alias]]`, `![[embeds]]`
- `#tags`, backlinks, outgoing-link graph, full-text search
- CLI: `kubo-notes <vault> list | graph | backlinks <Note.md> | search <q> | check`
- GUI: file list + search, editor, backlinks/tags panel (egui/eframe)

Verified in this repo's CI / local checks:

- `cargo test -p vault-core` — 4 tests pass
- `cargo check -p obsidian-rust-app` on Linux (host)
- `cargo check -p obsidian-rust-app --target x86_64-pc-windows-gnu`
- `cargo check -p obsidian-rust-app --target aarch64-linux-android`
- Android APK actually built (2026-10-30): debug 85 MB and
  release-optimized 2.4 MB, both `apksigner verify` v2/v3,
  package `tech.kuboes.obsidianrust`, arm64-v8a, min SDK 26 /
  target SDK 34. See `docs/ANDROID.md`. Not run on a device
  here (no device attached, no KVM emulator).

## Layout

```text
crates/vault-core/   no-dependency vault parser: frontmatter, wikilinks, tags, graph
crates/cli/          kubo-notes CLI
crates/app/          egui/eframe GUI (lib + bin; android_main for Android)
examples/vault/      tiny sample vault (A <-> B) for smoke tests
docs/COMPATIBILITY.md   exactly what is and is not compatible, and why
docs/ANDROID.md         APK build steps (cargo-apk + NDK)
docs/BUILD.md           Linux / Windows / Android build matrix
```

## Quick start

```bash
# Linux / Windows (from a Rust toolchain)
cargo test -p vault-core
cargo run -p kubo-notes -- examples/vault list
cargo run -p kubo-notes -- examples/vault graph
cargo run -p obsidian-rust -- examples/vault   # GUI

# Your real vault (read-only check first — writes nothing)
cargo run -p kubo-notes -- /path/to/your/vault check
```

## Compatibility, honestly

See [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). The short version:

- ✅ Vault file format: Markdown, frontmatter, wikilinks, tags, attachments as files
- ✅ Same folder opens in Obsidian unchanged (`.obsidian/` is left alone)
- ❌ Obsidian's JavaScript plugin API — plugins are JS, this is Rust.
  Plugin *compatibility* would mean a JS plugin host, which is a
  separate, later layer, not v0.1.
- ❌ Sync / Publish — those are Obsidian's paid services, not a file format.
  Use Git / Syncthing on the vault folder instead.
- ❌ Canvas, themes (CSS), Dataview queries — not yet, see roadmap.

## Roadmap

- [ ] Markdown preview rendering (not just plain-text editing)
- [ ] Graph view
- [ ] Daily notes, templates, attachments panel
- [ ] Mobile vault picker (Android scoped storage / SAF) instead of a typed path
- [ ] Optional JS plugin host (evaluating: a small, sandboxed host, not Electron)
- [ ] Releases: Linux AppImage / Windows `.exe` / Android APK via CI

## License

MIT OR Apache-2.0.
