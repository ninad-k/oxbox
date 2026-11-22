#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod accessibility;
mod app;
mod host;
mod icons;
mod instance;
mod integration;
mod jobs;
mod recovery;
mod settings_tx;
mod shortcuts;
mod startup;
mod tools;
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--uninstall-cleanup") {
        return startup::remove_if_owned()
            .map_err(|error| eframe::Error::AppCreation(error.into()));
    }
    let directory = if let Some(index) = args.iter().position(|value| value == "--profile") {
        args.get(index + 1)
            .map(std::path::PathBuf::from)
            .ok_or_else(|| eframe::Error::AppCreation("--profile requires a directory".into()))?
    } else {
        host::default_data_dir().map_err(|error| eframe::Error::AppCreation(Box::new(error)))?
    };
    let Some(instance) = instance::Instance::acquire(&directory)
        .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?
    else {
        return Ok(());
    };
    let host =
        host::Host::new(directory).map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    let icon = image::load_from_memory(include_bytes!("../../../src-tauri/icons/128x128.png"))
        .ok()
        .map(|image| {
            let rgba = image.into_rgba8();
            egui_icon(rgba)
        });
    let options = eframe::NativeOptions {
        viewport: {
            let view = eframe::egui::ViewportBuilder::default()
                .with_inner_size([800.0, 800.0])
                .with_min_inner_size([640.0, 640.0])
                .with_app_id("dev.oxbox.desktop");
            if let Some(icon) = icon {
                view.with_icon(icon)
            } else {
                view
            }
        },
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let autostart = args.iter().any(|arg| arg == "--autostart");
    let default_startup_allowed = !args.iter().any(|arg| arg == "--profile");
    eframe::run_native(
        "Oxbox",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::App::new(
                cc,
                host,
                instance,
                autostart,
                default_startup_allowed,
            )?))
        }),
    )
}
fn egui_icon(rgba: image::RgbaImage) -> eframe::egui::IconData {
    eframe::egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}
