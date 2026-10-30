use std::fs;
use std::path::PathBuf;
use vault_core::Vault;

fn tmp_vault() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("obsidian-rust-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::create_dir_all(dir.join(".obsidian")).unwrap();
    fs::write(
        dir.join("A.md"),
        "---\ntags: [work]\n---\nLinks to [[B]] and #todo\n",
    )
    .unwrap();
    fs::write(dir.join("sub").join("B.md"), "Back to [[A]]\n").unwrap();
    // .obsidian must be ignored as a note source, and left alone.
    fs::write(dir.join(".obsidian").join("app.json"), "{}").unwrap();
    fs::write(dir.join("not-a-note.txt"), "[[A]]").unwrap();
    dir
}

#[test]
fn vault_graph_and_backlinks() {
    let dir = tmp_vault();
    let vault = Vault::open(&dir).unwrap();
    assert_eq!(vault.notes.len(), 2, "only .md outside dot-folders");
    assert_eq!(vault.backlinks("A.md").len(), 1);
    assert_eq!(vault.backlinks("sub/B.md").len(), 1);
    assert_eq!(vault.search("work").len(), 1);
    assert_eq!(vault.search("todo").len(), 1);
    assert_eq!(vault.search("").len(), 2);
    // Opening must not modify the vault.
    assert!(fs::read_to_string(dir.join(".obsidian").join("app.json")).unwrap() == "{}");
    let _ = fs::remove_dir_all(&dir);
}
