fn main() -> eframe::Result<()> {
    let vault_arg = std::env::args().nth(1).unwrap_or_default();
    obsidian_rust_app::run_app(vault_arg)
}
