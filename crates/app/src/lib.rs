//! Editor UI over `vault-core`.
//!
//! v0.2 fixes the two things that made v0.1 unusable on a phone:
//! 1. The desktop 3-panel layout does not fit a narrow screen, so
//!    on narrow windows the UI becomes one view at a time with
//!    large tab buttons: Notes / Editor / Links.
//! 2. On modern Android, an app cannot just open another app's
//!    folder with READ_EXTERNAL_STORAGE. So v0.2 adds a sample
//!    vault inside the app's own files directory (no permission
//!    needed) and a MANAGE_EXTERNAL_STORAGE manifest permission
//!    for users who want to open their own vault folder — that
//!    permission has to be granted in Settings, it is not granted
//!    by tapping a button inside the app.

use std::path::PathBuf;
use vault_core::Vault;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Notes,
    Editor,
    Links,
}

pub struct App {
    vault_path_text: String,
    vault: Option<Vault>,
    search: String,
    selected: Option<String>,
    editor: String,
    status: String,
    tab: Tab,
}

fn sample_vault_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/tech.kuboes.obsidianrust/files/vault-sample")
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("vault-sample")
    }
}

fn create_sample_vault(dir: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("sub"))?;
    std::fs::write(
        dir.join("A.md"),
        "---\ntitle: はじめに\ntags: [sample]\n---\nこれはサンプルvaultです。\n\n[[B]] へリンクしています。タグは #todo です。\n\nこのフォルダはそのまま Obsidian でも開けます。\n",
    )?;
    std::fs::write(
        dir.join("sub").join("B.md"),
        "---\ntags: [sample]\n---\nB です。[[A]] に戻れます。\n",
    )?;
    Ok(())
}

impl App {
    pub fn new(vault_arg: String) -> App {
        App {
            vault_path_text: vault_arg,
            vault: None,
            search: String::new(),
            selected: None,
            editor: String::new(),
            status: "vaultを開いてください。まず「サンプルvault」が確実です。".to_string(),
            tab: Tab::Notes,
        }
    }

    #[cfg(target_os = "android")]
    pub fn new_android() -> App {
        let mut app = App::new(String::new());
        let dir = sample_vault_dir();
        // Auto-open the sample vault on Android: no typing, no
        // storage permission needed, and the user immediately sees
        // notes, links and backlinks working.
        if create_sample_vault(&dir).is_ok() {
            app.vault_path_text = dir.to_string_lossy().into_owned();
            app.load_vault();
            if let Some(vault) = &app.vault {
                if let Some(first) = vault.notes.first() {
                    let rel = first.rel_path.to_string_lossy().into_owned();
                    app.select(&rel);
                }
            }
        }
        app
    }

    fn load_vault(&mut self) {
        let path = PathBuf::from(self.vault_path_text.trim());
        match Vault::open(&path) {
            Ok(vault) => {
                self.status = format!("{}件のノートを開きました", vault.notes.len());
                self.selected = None;
                self.editor.clear();
                self.vault = Some(vault);
                self.tab = Tab::Notes;
            }
            Err(e) => {
                self.status = format!(
                    "vaultを開けませんでした: {e}\n\
                    Androidで自分のvaultフォルダを開くには、設定 > アプリ > obsidian_rust > 権限 > すべてのファイルへのアクセス をONにしてください。\n\
                    まず動くか確かめるなら「サンプルvault」を押してください。"
                );
                self.vault = None;
            }
        }
    }

    fn open_sample(&mut self) {
        let dir = sample_vault_dir();
        match create_sample_vault(&dir) {
            Ok(()) => {
                self.vault_path_text = dir.to_string_lossy().into_owned();
                self.load_vault();
            }
            Err(e) => {
                self.status = format!("サンプルvaultを作れませんでした: {e}");
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
        self.status = format!("編集中: {rel}");
        self.tab = Tab::Editor;
    }

    fn save(&mut self) {
        let Some(vault) = &self.vault else { return };
        let Some(rel) = &self.selected else { return };
        match vault.save(&PathBuf::from(rel), &self.editor) {
            Ok(()) => self.status = format!("保存しました: {rel}"),
            Err(e) => self.status = format!("保存できませんでした: {e}"),
        }
        let path = vault.root.clone();
        if let Ok(v) = Vault::open(&path) {
            self.vault = Some(v);
        }
    }

    fn new_note(&mut self) {
        let Some(vault) = &self.vault else {
            self.status = "先にvaultを開いてください".to_string();
            return;
        };
        let mut i = 1;
        let rel = loop {
            let candidate = format!("新しいノート{i}.md");
            if !vault
                .notes
                .iter()
                .any(|n| n.rel_path.to_string_lossy() == candidate)
            {
                break candidate;
            }
            i += 1;
        };
        if vault
            .save(&PathBuf::from(&rel), "# 新しいノート\n\n")
            .is_ok()
        {
            let path = vault.root.clone();
            if let Ok(v) = Vault::open(&path) {
                self.vault = Some(v);
            }
            self.select(&rel);
        }
    }

    fn notes_view(&mut self, ui: &mut egui::Ui, narrow: bool) {
        ui.label("検索");
        ui.text_edit_singleline(&mut self.search);
        ui.separator();
        let query = self.search.clone();
        let mut to_select: Option<String> = None;
        if let Some(vault) = &self.vault {
            for note in vault.search(&query) {
                let rel = note.rel_path.to_string_lossy().into_owned();
                let label = format!("{}\n{}", note.title(), rel);
                if narrow {
                    if ui
                        .add_sized([ui.available_width(), 56.0], egui::Button::new(label))
                        .clicked()
                    {
                        to_select = Some(rel);
                    }
                } else if ui
                    .selectable_label(
                        self.selected.as_deref() == Some(rel.as_str()),
                        format!("{} ({})", note.title(), rel),
                    )
                    .clicked()
                {
                    to_select = Some(rel);
                }
            }
            if vault.notes.is_empty() {
                ui.label("このvaultに .md がありません。パスが間違っていないか確認してください。");
            }
        } else {
            ui.label(
                "vaultが開かれていません。\n「サンプルvault」を押すと、すぐノートが表示されます。",
            );
        }
        if let Some(rel) = to_select {
            // On narrow screens, selecting goes straight to the editor.
            // On desktop, keep the 3-panel view and stay on the list.
            if narrow {
                self.select(&rel);
            } else {
                let Some(vault) = &self.vault else { return };
                if let Some(note) = vault
                    .notes
                    .iter()
                    .find(|n| n.rel_path.to_string_lossy() == rel.as_str())
                {
                    self.status = format!("編集中: {rel}");
                    self.selected = Some(rel);
                    self.editor = note.raw.clone();
                }
            }
        }
    }

    fn editor_view(&mut self, ui: &mut egui::Ui) {
        if self.selected.is_some() {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.editor)
                        .desired_width(f32::INFINITY)
                        .desired_rows(30),
                );
            });
        } else {
            ui.label("ノートを選んでください。\n「ノート一覧」タブから選べます。");
        }
    }

    fn links_view(&mut self, ui: &mut egui::Ui, narrow: bool) {
        let mut to_select: Option<String> = None;
        if let (Some(vault), Some(rel)) = (&self.vault, &self.selected) {
            if let Some(note) = vault
                .notes
                .iter()
                .find(|n| n.rel_path.to_string_lossy() == rel.as_str())
            {
                ui.label(format!("タグ: {}", note.tags.join(", ")));
                ui.separator();
                ui.label("リンク先:");
                for link in &note.links {
                    let label = format!("[[{}]]", link.target);
                    if narrow {
                        if ui
                            .add_sized([ui.available_width(), 48.0], egui::Button::new(label))
                            .clicked()
                        {
                            if let Some(to) = vault.resolve(&link.target) {
                                to_select = Some(to.rel_path.to_string_lossy().into_owned());
                            }
                        }
                    } else {
                        ui.label(label);
                    }
                }
                ui.separator();
                ui.label("バックリンク:");
                for back in vault.backlinks(rel) {
                    let back_rel = back.rel_path.to_string_lossy().into_owned();
                    if narrow {
                        if ui
                            .add_sized([ui.available_width(), 48.0], egui::Button::new(&back_rel))
                            .clicked()
                        {
                            to_select = Some(back_rel);
                        }
                    } else {
                        ui.label(back_rel);
                    }
                }
            }
        } else {
            ui.label("ノートを選んでください。");
        }
        if let Some(rel) = to_select {
            if narrow {
                self.select(&rel);
            } else {
                self.selected = Some(rel);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let narrow = ctx.screen_rect().width() < 900.0;
        if narrow {
            // Bigger text and buttons on a phone. This is per-frame
            // style, not a stored setting.
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(26.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(19.0, egui::FontFamily::Monospace),
            );
            style.spacing.button_padding = egui::vec2(16.0, 12.0);
            style.spacing.item_spacing = egui::vec2(10.0, 10.0);
            ctx.set_style(style);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.label("Vault:");
            ui.text_edit_singleline(&mut self.vault_path_text);
            ui.horizontal(|ui| {
                if ui
                    .add_sized([110.0, 48.0], egui::Button::new("開く"))
                    .clicked()
                {
                    self.load_vault();
                }
                if ui
                    .add_sized([110.0, 48.0], egui::Button::new("保存"))
                    .clicked()
                {
                    self.save();
                }
                if ui
                    .add_sized([150.0, 48.0], egui::Button::new("新規ノート"))
                    .clicked()
                {
                    self.new_note();
                }
            });
            if ui
                .add_sized(
                    [ui.available_width(), 52.0],
                    egui::Button::new("サンプルvaultで試す"),
                )
                .clicked()
            {
                self.open_sample();
            }
            ui.label(&self.status);
        });

        if narrow {
            egui::TopBottomPanel::bottom("tabs").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let w = ui.available_width() / 3.0 - 8.0;
                    if ui
                        .add_sized(
                            [w, 56.0],
                            egui::SelectableLabel::new(self.tab == Tab::Notes, "ノート一覧"),
                        )
                        .clicked()
                    {
                        self.tab = Tab::Notes;
                    }
                    if ui
                        .add_sized(
                            [w, 56.0],
                            egui::SelectableLabel::new(self.tab == Tab::Editor, "編集"),
                        )
                        .clicked()
                    {
                        self.tab = Tab::Editor;
                    }
                    if ui
                        .add_sized(
                            [w, 56.0],
                            egui::SelectableLabel::new(self.tab == Tab::Links, "リンク"),
                        )
                        .clicked()
                    {
                        self.tab = Tab::Links;
                    }
                });
            });
            egui::CentralPanel::default().show(ctx, |ui| match self.tab {
                Tab::Notes => self.notes_view(ui, true),
                Tab::Editor => self.editor_view(ui),
                Tab::Links => self.links_view(ui, true),
            });
            return;
        }

        // Desktop: keep the 3-panel layout.
        egui::SidePanel::left("notes").show(ctx, |ui| {
            self.notes_view(ui, false);
        });
        egui::SidePanel::right("links").show(ctx, |ui| {
            ui.heading("Links");
            self.links_view(ui, false);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor_view(ui);
        });
    }
}

pub fn run_app(vault_arg: String) -> eframe::Result<()> {
    let app = App::new(vault_arg);
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
        Box::new(|_cc| Ok(Box::new(App::new_android()))),
    );
}
