//! Developer aid: with BRICK_AUTOSHOT=<dir> the app walks through its screens,
//! saves screenshots and exits. Unused in normal runs.

use super::state::{Doc, Editor, FrameRequest, Library, Manual, Screen, Ui};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

#[derive(Resource)]
pub struct AutoShot {
    dir: std::path::PathBuf,
    frame: u32,
}

pub fn plugin(app: &mut App) {
    if let Some(dir) = std::env::var_os("BRICK_AUTOSHOT") {
        app.insert_resource(AutoShot { dir: dir.into(), frame: 0 }).add_systems(Update, step);
    }
}

#[allow(clippy::too_many_arguments)]
fn step(
    mut commands: Commands,
    mut a: ResMut<AutoShot>,
    mut ui: ResMut<Ui>,
    mut doc: ResMut<Doc>,
    lib: Res<Library>,
    mut manual: ResMut<Manual>,
    mut editor: ResMut<Editor>,
    mut frame: ResMut<FrameRequest>,
    mut exit: EventWriter<AppExit>,
) {
    a.frame += 1;
    let f = a.frame;
    let build = std::env::var("BRICK_AUTOSHOT_BUILD").ok().and_then(|s| s.parse().ok()).unwrap_or(4usize);
    let shot = |commands: &mut Commands, name: &str| {
        let path = a.dir.join(name);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    };
    match f {
        40 => shot(&mut commands, "1_picker.png"),
        50 => {
            doc.replace(lib.entries[build].model.clone(), None);
            ui.screen = Screen::Editor;
            editor.view_version += 1;
            frame.0 = true;
        }
        110 => shot(&mut commands, "2_editor.png"),
        120 => {
            manual.open(&doc.model, Screen::Editor);
            manual.step = manual.ins.len() / 3;
            ui.screen = Screen::Instructions;
            editor.view_version += 1;
        }
        180 => shot(&mut commands, "3_instructions.png"),
        240 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
