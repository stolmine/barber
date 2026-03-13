mod app;
mod audio;
mod edit;
mod history;
mod keybinds;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 600.0])
            .with_title("Barber"),
        event_loop_builder: Some(Box::new(|builder| {
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::EventLoopBuilderExtMacOS;
                builder.with_default_menu(false);
            }
        })),
        ..Default::default()
    };

    eframe::run_native("Barber", options, Box::new(|cc| {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "jbmono".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(
                include_bytes!("../assets/JetBrainsMonoNF-Regular.ttf"),
            )),
        );
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
            .insert(0, "jbmono".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
            .insert(0, "jbmono".to_owned());
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        Ok(Box::new(app::BarberApp::default()))
    }))
}
