//! `era-builder [project folder]`: open Era Builder.

fn main() -> eframe::Result<()> {
    let path = std::env::args().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Era Builder")
            .with_inner_size([1600.0, 980.0])
            .with_min_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Era Builder",
        options,
        Box::new(move |_cc| Ok(Box::new(era_builder::app::Builder::new(path)))),
    )
}
