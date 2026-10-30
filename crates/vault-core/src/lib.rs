//! Obsidian vault format, read as plain files.
//!
//! Compatibility goal, v0.1:
//! - A vault is just a folder of `.md` files. Nothing is converted,
//!   nothing is stored in a database. Opening a vault must never
//!   rewrite it, so the same folder still opens in Obsidian unchanged.
//! - Frontmatter (`---` block at the top), `[[wikilinks]]`,
//!   `![[embeds]]`, `#tags` and backlinks/graph are understood.
//! - `.obsidian/`, `.git/` and other dot-folders are skipped when
//!   scanning notes, but are never deleted or modified.
//!
//! Not in v0.1, on purpose (see `docs/COMPATIBILITY.md`):
//! Obsidian's JavaScript plugin API, Sync, Publish, Canvas and
//! themes. Those are a different compatibility layer from the
//! vault file format.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// One `[[...]]` (or `![[...]]`) found in a note body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Text before `#` / `|`, e.g. `Note`, `folder/Note`, `image.png`.
    pub target: String,
    /// `#Heading` part, if any.
    pub heading: Option<String>,
    /// `|Alias` part, if any.
    pub alias: Option<String>,
    /// True for `![[...]]`.
    pub embed: bool,
}

/// One Markdown note in a vault.
#[derive(Debug, Clone)]
pub struct Note {
    /// Path relative to the vault root, e.g. `folder/Note.md`.
    pub rel_path: PathBuf,
    /// Filename stem, e.g. `Note`. Fallback title.
    pub stem: String,
    /// `title:` from frontmatter, if present.
    pub frontmatter_title: Option<String>,
    /// Raw frontmatter block (without the `---` fences), if present.
    pub frontmatter_raw: Option<String>,
    /// Tags from frontmatter plus `#tags` in the body.
    pub tags: Vec<String>,
    /// `aliases:` from frontmatter.
    pub aliases: Vec<String>,
    /// Wikilinks and embeds found outside fenced code blocks.
    pub links: Vec<Link>,
    /// Body without the frontmatter block.
    pub body: String,
    /// Full file text, as read from disk. Saving edits writes this
    /// shape back, so frontmatter is preserved byte-for-byte.
    pub raw: String,
}

impl Note {
    pub fn title(&self) -> &str {
        self.frontmatter_title.as_deref().unwrap_or(&self.stem)
    }

    pub fn parse(rel_path: PathBuf, raw: String) -> Note {
        let stem = rel_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (frontmatter_raw, body) = split_frontmatter(&raw);
        let mut tags = Vec::new();
        let mut aliases = Vec::new();
        let mut frontmatter_title = None;
        if let Some(fm) = &frontmatter_raw {
            let parsed = parse_frontmatter(fm);
            frontmatter_title = parsed.title;
            tags.extend(parsed.tags);
            aliases.extend(parsed.aliases);
        }
        let searchable = strip_fenced_code(&body);
        let mut links = parse_wikilinks(&searchable);
        // Keep document order, drop exact duplicates.
        links.dedup();
        for tag in parse_tags(&searchable) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        Note {
            rel_path,
            stem,
            frontmatter_title,
            frontmatter_raw,
            tags,
            aliases,
            links,
            body,
            raw,
        }
    }
}

/// A vault: a folder scanned into notes, plus a resolved link graph.
#[derive(Debug)]
pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
}

impl Vault {
    /// Scan `root` recursively. Does not write anything.
    pub fn open(root: &Path) -> std::io::Result<Vault> {
        let mut files = Vec::new();
        collect_md_files(root, root, &mut files)?;
        files.sort();
        let mut notes = Vec::new();
        for path in files {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            let raw = fs::read_to_string(&path)?;
            notes.push(Note::parse(rel, raw));
        }
        Ok(Vault {
            root: root.to_path_buf(),
            notes,
        })
    }

    /// Resolve a link target to a note, Obsidian-style (shortest match):
    /// exact relative path first, then filename stem, then alias.
    pub fn resolve(&self, target: &str) -> Option<&Note> {
        let target = target.trim();
        if target.is_empty() {
            return None;
        }
        let with_md = if target.ends_with(".md") {
            target.to_string()
        } else {
            format!("{target}.md")
        };
        if let Some(n) = self.notes.iter().find(|n| {
            n.rel_path.to_string_lossy().replace('\\', "/") == with_md
                || n.rel_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .ends_with(&format!("/{with_md}"))
        }) {
            return Some(n);
        }
        if let Some(n) = self.notes.iter().find(|n| n.stem == target) {
            return Some(n);
        }
        self.notes
            .iter()
            .find(|n| n.aliases.iter().any(|a| a == target))
    }

    /// Outgoing resolved links per note, keyed by relative path.
    pub fn graph(&self) -> BTreeMap<String, Vec<String>> {
        let mut map = BTreeMap::new();
        for note in &self.notes {
            let mut out = BTreeSet::new();
            for link in &note.links {
                if link.embed {
                    continue;
                }
                if let Some(to) = self.resolve(&link.target) {
                    out.insert(to.rel_path.to_string_lossy().into_owned());
                }
            }
            map.insert(
                note.rel_path.to_string_lossy().into_owned(),
                out.into_iter().collect(),
            );
        }
        map
    }

    /// Notes that link to `rel_path` (backlinks).
    pub fn backlinks(&self, rel_path: &str) -> Vec<&Note> {
        let graph = self.graph();
        self.notes
            .iter()
            .filter(|n| {
                graph
                    .get(&n.rel_path.to_string_lossy().into_owned())
                    .map(|outs| outs.iter().any(|o| o == rel_path))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Case-insensitive substring search over title, tags and body.
    pub fn search<'a>(&'a self, query: &str) -> Vec<&'a Note> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return self.notes.iter().collect();
        }
        self.notes
            .iter()
            .filter(|n| {
                n.title().to_lowercase().contains(&q)
                    || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || n.body.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Write a note back as plain Markdown. Creates parent folders.
    /// This is the only write path in v0.1, and it never touches
    /// `.obsidian/` or any other note.
    pub fn save(&self, rel_path: &Path, content: &str) -> std::io::Result<()> {
        let path = self.root.join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)
    }
}

fn collect_md_files(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let _ = root;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            // Skip dot-folders: .obsidian, .git, .trash, ...
            if name.starts_with('.') {
                continue;
            }
            collect_md_files(&path, root, out)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

/// Split `---\n...\n---\n<body>` into (frontmatter, body).
pub fn split_frontmatter(raw: &str) -> (Option<String>, String) {
    let mut lines = raw.lines();
    if lines.next().as_deref() != Some("---") {
        return (None, raw.to_string());
    }
    let mut fm = Vec::new();
    for line in lines.by_ref() {
        if line == "---" || line == "..." {
            // Remainder after the closing fence is the body.
            let body: String = lines.collect::<Vec<_>>().join("\n");
            // Preserve a trailing newline if the original had one.
            let body = if raw.ends_with('\n') && !body.is_empty() {
                format!("{body}\n")
            } else {
                body
            };
            return (Some(fm.join("\n")), body);
        }
        fm.push(line);
    }
    // Unterminated frontmatter: treat the whole file as body.
    (None, raw.to_string())
}

struct Frontmatter {
    title: Option<String>,
    tags: Vec<String>,
    aliases: Vec<String>,
}

/// Minimal YAML-subset parser, enough for Obsidian frontmatter:
/// `key: value`, `key: [a, b]`, and `key:\n  - a\n  - b`.
fn parse_frontmatter(fm: &str) -> Frontmatter {
    let mut out = Frontmatter {
        title: None,
        tags: Vec::new(),
        aliases: Vec::new(),
    };
    let mut current_list: Option<String> = None;
    for raw_line in fm.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("- ") {
            if let Some(key) = &current_list {
                let value = stripped.trim().trim_matches('"').trim_matches('\'');
                match key.as_str() {
                    "tags" | "tag" => out.tags.push(value.to_string()),
                    "aliases" | "alias" => out.aliases.push(value.to_string()),
                    _ => {}
                }
            }
            continue;
        }
        current_list = None;
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            current_list = Some(key.to_string());
            continue;
        }
        let value = value.trim_matches('"').trim_matches('\'');
        match key {
            "title" => out.title = Some(value.to_string()),
            "tags" | "tag" => {
                if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                    out.tags.extend(
                        inner
                            .split(',')
                            .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|v| !v.is_empty()),
                    );
                } else {
                    out.tags.extend(
                        value
                            .split(',')
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty()),
                    );
                }
            }
            "aliases" | "alias" => {
                if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                    out.aliases.extend(
                        inner
                            .split(',')
                            .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|v| !v.is_empty()),
                    );
                } else {
                    out.aliases.push(value.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

/// Remove fenced code blocks (``` / ~~~) so links/tags inside
/// code are not mistaken for real links, as in Obsidian.
pub fn strip_fenced_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_fence: Option<&str> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(fence) = in_fence {
            if trimmed.starts_with(fence) {
                in_fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") {
            in_fence = Some("```");
            continue;
        }
        if trimmed.starts_with("~~~") {
            in_fence = Some("~~~");
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn parse_wikilinks(text: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let embed = i > 0 && bytes[i - 1] == b'!';
            if let Some(end) = text[i + 2..].find("]]") {
                let inner = &text[i + 2..i + 2 + end];
                let (before_alias, alias) = match inner.split_once('|') {
                    Some((a, b)) => (a, Some(b.trim().to_string())),
                    None => (inner, None),
                };
                let (target, heading) = match before_alias.split_once('#') {
                    Some((a, b)) => (a.trim().to_string(), Some(b.trim().to_string())),
                    None => (before_alias.trim().to_string(), None),
                };
                if !target.is_empty() {
                    links.push(Link {
                        target,
                        heading,
                        alias,
                        embed,
                    });
                }
                i = i + 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    links
}

pub fn parse_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in text.lines() {
        // Headings (`# Title`) are not tags.
        let trimmed = line.trim_start();
        if trimmed.starts_with("# ") || trimmed.starts_with("##") && !trimmed.starts_with("###") {
            // Fall through: a heading line can still contain tags
            // later in the line, handled by the scan below, but a
            // leading `# ` must not become a tag. The scan checks
            // the character before `#`, so a line-start `#` followed
            // by a space is already excluded there.
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '#'
                && (i == 0 || chars[i - 1].is_whitespace())
                && i + 1 < chars.len()
                && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_' || chars[i + 1] == '-')
            {
                let mut j = i + 1;
                while j < chars.len()
                    && (chars[j].is_alphanumeric()
                        || chars[j] == '_'
                        || chars[j] == '-'
                        || chars[j] == '/')
                {
                    j += 1;
                }
                let tag: String = chars[i + 1..j].iter().collect();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
                i = j;
            } else {
                i += 1;
            }
        }
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wikilink_forms() {
        let links = parse_wikilinks("a [[Note]] b [[Note#Head|Alias]] c ![[img.png]]");
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target, "Note");
        assert_eq!(links[1].heading.as_deref(), Some("Head"));
        assert_eq!(links[1].alias.as_deref(), Some("Alias"));
        assert!(links[2].embed);
    }

    #[test]
    fn frontmatter_and_tags() {
        let raw = "---\ntitle: Hello\ntags: [a, b]\naliases:\n  - Hi\n---\nBody #c and [[Hello]]\n";
        let note = Note::parse(PathBuf::from("Hello.md"), raw.to_string());
        assert_eq!(note.title(), "Hello");
        assert!(note.tags.contains(&"a".to_string()));
        assert!(note.tags.contains(&"c".to_string()));
        assert!(note.aliases.contains(&"Hi".to_string()));
        assert_eq!(note.links.len(), 1);
    }

    #[test]
    fn code_fence_is_not_a_link() {
        let raw = "```\n[[Fake]]\n```\nReal [[Note]]\n";
        let note = Note::parse(PathBuf::from("x.md"), raw.to_string());
        assert_eq!(note.links.len(), 1);
        assert_eq!(note.links[0].target, "Note");
    }
}
