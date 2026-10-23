//! Bevy application: wiring of resources and systems.

mod autoshot;
mod camera;
mod editor;
mod jobs;
mod scene;
mod state;
mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub struct BrickBuilderPlugin;

impl Plugin for BrickBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(state::Doc::new(state::empty_build()))
            .insert_resource(state::Library::load())
            .init_resource::<state::Checked>()
            .init_resource::<state::Editor>()
            .init_resource::<state::Ui>()
            .init_resource::<state::Manual>()
            .init_resource::<state::Toasts>()
            .insert_resource(state::FrameRequest(true))
            .init_resource::<camera::Gestures>()
            .init_resource::<scene::Spawned>()
            .init_resource::<jobs::Jobs>()
            .init_resource::<ui::UiCache>()
            .init_resource::<ui::Confirm>()
            .add_systems(Startup, scene::setup_scene)
            .add_systems(
                Update,
                (
                    camera::gestures,
                    camera::orbit_camera,
                    editor::hover,
                    editor::edit,
                    editor::manual_keys,
                    jobs::poll_jobs,
                    state::update_check,
                    scene::sync_scene,
                    scene::update_ghost,
                    scene::draw_gizmos,
                )
                    .chain(),
            )
            .add_systems(EguiPrimaryContextPass, ui::ui);
        autoshot::plugin(app);
    }
}
