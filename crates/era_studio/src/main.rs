//! `era-studio [project folder]`: open ERA Studio.

fn main() -> eframe::Result<()> {
    let path = std::env::args().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ERA Studio")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ERA Studio",
        options,
        Box::new(move |cc| Ok(Box::new(era_studio::app::Studio::new(&cc.egui_ctx, path)))),
    )
}
