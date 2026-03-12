mod app;
mod audio;
mod edit;
mod history;
mod keybinds;
mod ui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 600.0])
            .with_title("Barber"),
        ..Default::default()
    };

    eframe::run_native("Barber", options, Box::new(|_cc| Ok(Box::new(app::BarberApp::default()))))
}
