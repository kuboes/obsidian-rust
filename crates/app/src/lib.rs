//! Minimal editor UI over `vault-core`.
//! Same crate on Linux, Windows and Android (eframe/winit):
//! file list + search on the left, plain-Markdown editor in the
//! middle, backlinks/tags on the right. Saving writes only the
//! edited `.md` file, as plain text, so Obsidian keeps opening it.

use std::path::PathBuf;
use vault_core::Vault;

pub struct App {
    vault_path_text: String,
    vault: Option<Vault>,
    search: String,
    selected: Option<String>,
    editor: String,
    status: String,
}

impl App {
    fn load_vault(&mut self) {
        let path = PathBuf::from(self.vault_path_text.trim());
        match Vault::open(&path) {
            Ok(vault) => {
                self.status = format!("{} notes loaded, no files rewritten", vault.notes.len());
                self.selected = None;
                self.editor.clear();
                self.vault = Some(vault);
            }
            Err(e) => {
                self.status = format!("cannot open vault: {e}");
                self.vault = None;
            }
        }
    }

    fn select(&mut self, rel: &str) {
        let Some(vault) = &self.vault else { return };
        let Some(note) = vault
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == rel)
        else {
            return;
        };
        self.selected = Some(rel.to_string());
        self.editor = note.raw.clone();
        self.status = format!("editing {}", rel);
    }

    fn save(&mut self) {
        let Some(vault) = &self.vault else { return };
        let Some(rel) = &self.selected else { return };
        match vault.save(&PathBuf::from(rel), &self.editor) {
            Ok(()) => self.status = format!("saved {rel}"),
            Err(e) => self.status = format!("save failed: {e}"),
        }
        // Reload the index so backlinks/tags follow the edit.
        let path = vault.root.clone();
        if let Ok(v) = Vault::open(&path) {
            self.vault = Some(v);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Vault:");
                ui.text_edit_singleline(&mut self.vault_path_text);
                if ui.button("Open").clicked() {
                    self.load_vault();
                }
                if ui.button("Save").clicked() {
                    self.save();
                }
                ui.label(&self.status);
            });
        });

        egui::SidePanel::left("notes").show(ctx, |ui| {
            ui.label("Search");
            ui.text_edit_singleline(&mut self.search);
            ui.separator();
            let query = self.search.clone();
            let mut to_select: Option<String> = None;
            if let Some(vault) = &self.vault {
                for note in vault.search(&query) {
                    let rel = note.rel_path.to_string_lossy().into_owned();
                    let selected = self.selected.as_deref() == Some(rel.as_str());
                    if ui
                        .selectable_label(selected, format!("{} ({})", note.title(), rel))
                        .clicked()
                    {
                        to_select = Some(rel);
                    }
                }
            } else {
                ui.label("Open a vault folder first.\nThat folder also opens in Obsidian.");
            }
            if let Some(rel) = to_select {
                self.select(&rel);
            }
        });

        egui::SidePanel::right("links").show(ctx, |ui| {
            ui.heading("Links");
            if let (Some(vault), Some(rel)) = (&self.vault, &self.selected) {
                if let Some(note) = vault
                    .notes
                    .iter()
                    .find(|n| n.rel_path.to_string_lossy() == rel.as_str())
                {
                    ui.label(format!("Tags: {}", note.tags.join(", ")));
                    ui.separator();
                    ui.label("Outgoing:");
                    for link in &note.links {
                        ui.label(format!("[[{}]]", link.target));
                    }
                    ui.separator();
                    ui.label("Backlinks:");
                    for back in vault.backlinks(rel) {
                        ui.label(back.rel_path.to_string_lossy().into_owned());
                    }
                }
            } else {
                ui.label("Select a note.");
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected.is_some() {
                ui.add(
                    egui::TextEdit::multiline(&mut self.editor)
                        .desired_width(f32::INFINITY)
                        .desired_rows(40),
                );
            } else {
                ui.label("Obsidian-compatible notes, rewritten in Rust.\n\nA vault is a folder of .md files. This app edits those files directly — no database, no lock-in.");
            }
        });
    }
}

pub fn run_app(vault_arg: String) -> eframe::Result<()> {
    let app = App {
        vault_path_text: vault_arg,
        vault: None,
        search: String::new(),
        selected: None,
        editor: String::new(),
        status: "Open a vault folder.".to_string(),
    };
    let options = eframe::NativeOptions::default();
    eframe::run_native("obsidian-rust", options, Box::new(|_cc| Ok(Box::new(app))))
}

/// Android entry point (cargo-apk / NativeActivity).
/// Desktop keeps using `main()`; Android has no `main()`.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "obsidian-rust",
        options,
        Box::new(|_cc| {
            Ok(Box::new(App {
                vault_path_text: String::new(),
                vault: None,
                search: String::new(),
                selected: None,
                editor: String::new(),
                status: "Open a vault folder.".to_string(),
            }))
        }),
    );
}
