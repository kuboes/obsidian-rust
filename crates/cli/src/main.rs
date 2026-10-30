use std::path::{Path, PathBuf};
use vault_core::Vault;

fn usage() -> ! {
    eprintln!("Usage:");
    eprintln!("  kubo-notes <vault> list");
    eprintln!("  kubo-notes <vault> graph");
    eprintln!("  kubo-notes <vault> backlinks <Note.md>");
    eprintln!("  kubo-notes <vault> search <query>");
    eprintln!("  kubo-notes <vault> check   # open-only: proves the vault is readable without rewriting it");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        usage();
    }
    let vault_path = PathBuf::from(&args[1]);
    let cmd = args[2].as_str();
    let vault = match Vault::open(&vault_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cannot open vault {}: {e}", vault_path.display());
            std::process::exit(1);
        }
    };
    match cmd {
        "list" => {
            for note in &vault.notes {
                println!(
                    "{}\t{}\ttags={}",
                    note.rel_path.display(),
                    note.title(),
                    note.tags.join(",")
                );
            }
        }
        "graph" => {
            for (from, tos) in vault.graph() {
                for to in tos {
                    println!("{from} -> {to}");
                }
            }
        }
        "backlinks" => {
            let target = args.get(3).cloned().unwrap_or_else(|| usage());
            for note in vault.backlinks(&target) {
                println!("{}", note.rel_path.display());
            }
        }
        "search" => {
            let query = args.get(3).cloned().unwrap_or_else(|| usage());
            for note in vault.search(&query) {
                println!("{}\t{}", note.rel_path.display(), note.title());
            }
        }
        "check" => {
            println!(
                "ok: {} notes in {} (no files written)",
                vault.notes.len(),
                Path::new(&vault_path).display()
            );
        }
        _ => usage(),
    }
}
