//! Entry point of zoneEdit (eframe GUI).

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "zoneEDIT",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1500.0, 820.0])
                .with_min_inner_size([1000.0, 640.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            zoneedit::ui_kit::apply_theme(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(zoneedit::ui::ZoneEditApp::new()))
        }),
    )
}
