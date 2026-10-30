# Compatibility: what "Obsidian-compatible" means here

## The vault is the contract

Obsidian's real interoperability surface is not its app binary.
It is the folder:

- notes are plain `.md` files, in nested folders
- links are `[[wikilinks]]`, embeds are `![[...]]`
- metadata is YAML frontmatter at the top of a file
- tags are `#tag` in text or `tags:` in frontmatter
- settings live in `.obsidian/` and are not notes

This app implements exactly that contract, in `vault-core`,
with no other dependencies. That is why a folder opened here
still opens in Obsidian, and vice versa.

## Supported in v0.1

| Feature | Status |
|---|---|
| Recursive `.md` scan, `.obsidian/`/`.git/` skipped | ✅ |
| Frontmatter `title`, `tags`, `aliases` (inline + list) | ✅ |
| `[[Note]]`, `[[Note#Heading]]`, `[[Note\|Alias]]`, combined | ✅ |
| `![[embed]]` (parsed, not yet rendered inline) | ✅ parsed |
| `#tags`, incl. `parent/child` nested tags | ✅ |
| Link resolution: exact path → filename stem → alias | ✅ |
| Backlinks, outgoing graph | ✅ |
| Search (title / tags / body, case-insensitive) | ✅ |
| Fenced code blocks excluded from link/tag parsing | ✅ |
| Saving: only the edited `.md` file, plain text | ✅ |

## Deliberately not in v0.1

| Feature | Why it is separate |
|---|---|
| Obsidian plugins (JS API) | Plugins are JavaScript. Running them needs a JS host/sandbox, not just a Markdown parser. Claiming "plugin compatible" without that would be false. |
| Sync / Publish | Paid Obsidian services, not a file format. Vault folders already sync with Git/Syncthing. |
| Canvas (`.canvas` JSON) | A different file format; parseable later, not a v0.1 note feature. |
| Themes / CSS snippets | Cosmetic layer on top of the editor; the vault does not depend on it. |
| Dataview / Templater queries | Plugin features, same reason as plugins. |
| Image/PDF rendering in preview | Editor is plain-text in v0.1; preview is on the roadmap. |

## Rules this project will not break

1. Opening a vault never writes to it.
2. Saving writes one `.md` file, in place, as text.
3. `.obsidian/` is never modified or deleted by this app.
4. No database, no import/export step, no proprietary format.
5. If Obsidian and this app disagree on a file, the file on disk wins.
