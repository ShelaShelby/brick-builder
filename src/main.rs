//! Brick Builder: build with toy bricks in 3D, load ready-made builds and
//! follow step-by-step instructions.

// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Brick Builder".into(),
                        resolution: WindowResolution::new(1440.0, 900.0),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin { filter: "wgpu=error,naga=warn,bevy_render=warn".into(), ..default() }),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(app::BrickBuilderPlugin)
        .run();
}
