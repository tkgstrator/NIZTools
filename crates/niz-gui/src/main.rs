fn main() -> eframe::Result {
    let demo = std::env::args().any(|a| a == "--demo");
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([960., 820.])
            .with_min_inner_size([700., 580.]),
        ..Default::default()
    };
    eframe::run_native(
        "NIZ Tools",
        options,
        Box::new(move |cc| Ok(Box::new(niz_gui::app::App::new(cc, demo)))),
    )
}
