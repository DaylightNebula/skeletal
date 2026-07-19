use std::path::PathBuf;

use anarchy::macros::system;
use anarchy::anyhow::{self, bail};
use anarchy::{Res, ResourceMeta};
use cell::EguiPlugin;
use cell::{App, EguiCtx, egui::egui};
use gearbox::GearboxRenderPlugin;

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugin(GearboxRenderPlugin)
        // .add_plugin(EguiPlugin)
        .on_render_update(update)
        .run()
}

#[system]
fn update(
    egui: Res<EguiCtx>
) {
    // let Some(path) = get_path() else { bail!("No path provided") };

    egui::Window::new("Model").show(&egui.context, |ui| {
        ui.label(format!("Path"));
    });
}

fn get_path() -> Option<PathBuf> {
    // Skip args[0] (the binary name) and look for the first real argument
    if let Some(arg) = std::env::args().nth(1) {
        Some(PathBuf::from(arg))
    } else {
        rfd::FileDialog::new()
            .set_title("Select a file")
            .pick_file()
    }
}
