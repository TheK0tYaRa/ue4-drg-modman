mod app;
mod db;
mod installer;
mod mod_io;
mod ui;

use app::ModManager;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "DRG Mod Manager",
        options,
        Box::new(|_cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(Box::new(ModManager::default()))
        }),
    )
}
