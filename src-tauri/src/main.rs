#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(debug_assertions)]
    if std::env::var("SKCF_INTERNAL_EXPORT_SMOKE").as_deref() == Ok("1") {
        match skulld_clip_forge_lib::internal_smoke::run() {
            Ok(()) => {
                println!("Installed export smoke passed.");
                return;
            }
            Err(error) => {
                eprintln!("Installed export smoke failed: {error}");
                std::process::exit(1);
            }
        }
    }

    skulld_clip_forge_lib::run();
}
