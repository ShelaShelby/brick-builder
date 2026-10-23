//! Mouse gestures and the orbit camera.

use super::state::{Doc, FrameRequest};
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;
use brick_builder::parts::Part;

/// Mouse state for the 3D view, with presses that started over the UI ignored.
#[derive(Resource, Default)]
pub struct Gestures {
    pub cursor: Option<Vec2>,
    pub over_ui: bool,
    pub motion: Vec2,
    pub scroll: f32,
    pub left_pressed: bool,
    pub left_down: bool,
    pub left_released: bool,
    pub left_start: Option<Vec2>,
    pub right_click: bool,
    pub orbiting: bool,
    pub panning: bool,
    last_cursor: Option<Vec2>,
    right_active: bool,
    right_moved: f32,
    middle_active: bool,
}

pub fn gestures(
    mut g: ResMut<Gestures>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: EventReader<MouseMotion>,
    mut wheel: EventReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    egui: Res<EguiWantsInput>,
) {
    g.cursor = window.cursor_position();
    g.over_ui = egui.wants_any_pointer_input();
    // Prefer cursor movement (works everywhere, incl. remote desktops); fall back
    // to raw motion when the cursor is grabbed or outside the window.
    let raw: Vec2 = motion.read().map(|m| m.delta).sum();
    g.motion = match (g.cursor, g.last_cursor) {
        (Some(c), Some(l)) => c - l,
        _ => raw,
    };
    g.last_cursor = g.cursor;
    let scroll: f32 = wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y,
            MouseScrollUnit::Pixel => w.y / 40.0,
        })
        .sum();
    g.scroll = if g.over_ui { 0.0 } else { scroll };

    if g.left_released {
        g.left_start = None;
    }
    g.left_pressed = buttons.just_pressed(MouseButton::Left) && !g.over_ui && g.cursor.is_some();
    if g.left_pressed {
        g.left_start = g.cursor;
        g.left_down = true;
    }
    g.left_released = buttons.just_released(MouseButton::Left) && g.left_down;
    if !buttons.pressed(MouseButton::Left) {
        g.left_down = false;
    }

    g.right_click = false;
    if buttons.just_pressed(MouseButton::Right) && !g.over_ui {
        g.right_active = true;
        g.right_moved = 0.0;
    }
    if g.right_active {
        g.right_moved += g.motion.length();
        if buttons.just_released(MouseButton::Right) || !buttons.pressed(MouseButton::Right) {
            g.right_click = g.right_moved < 5.0;
            g.right_active = false;
        }
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let right_drag = g.right_active && g.right_moved >= 5.0;
    if buttons.just_pressed(MouseButton::Middle) && !g.over_ui {
        g.middle_active = true;
    }
    if !buttons.pressed(MouseButton::Middle) {
        g.middle_active = false;
    }
    g.orbiting = right_drag && !shift;
    g.panning = (right_drag && shift) || g.middle_active;
}

#[derive(Component)]
pub struct OrbitCam {
    pub focus: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}

impl OrbitCam {
    pub fn transform(&self) -> Transform {
        let rot = Quat::from_euler(EulerRot::YXZ, self.yaw, -self.pitch, 0.0);
        let pos = self.focus + rot * Vec3::new(0.0, 0.0, self.dist);
        Transform::from_translation(pos).looking_at(self.focus, Vec3::Y)
    }
}

pub fn orbit_camera(
    g: Res<Gestures>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut frame: ResMut<FrameRequest>,
    doc: Res<Doc>,
    egui: Res<EguiWantsInput>,
    mut q: Query<(&mut OrbitCam, &mut Transform)>,
) {
    let Ok((mut cam, mut tf)) = q.single_mut() else { return };
    if frame.0 {
        frame.0 = false;
        let frame_bricks: Vec<_> = doc.model.bricks.iter().filter(|b| b.part != Part::Baseplate).copied().collect();
        let bounds = if frame_bricks.len() >= 3 {
            let mut m = brick_builder::model::Model::new("");
            m.bricks = frame_bricks;
            m.bounds()
        } else {
            doc.model.bounds()
        };
        if let Some((mn, mx)) = bounds {
            let mn = Vec3::from(mn);
            let mx = Vec3::from(mx);
            cam.focus = (mn + mx) * 0.5;
            cam.focus.y = mn.y + (mx.y - mn.y) * 0.3;
            let r = (mx - mn).length() * 0.5;
            cam.dist = (r * 2.4).clamp(28.0, 400.0);
        } else {
            cam.focus = Vec3::ZERO;
            cam.dist = 40.0;
        }
        cam.yaw = 0.6;
        cam.pitch = 0.6;
    }
    let speed = 0.008;
    if g.orbiting {
        cam.yaw -= g.motion.x * speed;
        cam.pitch = (cam.pitch + g.motion.y * speed).clamp(0.05, 1.5);
    }
    if g.panning {
        let right = tf.right();
        let fwd = Vec3::new(tf.forward().x, 0.0, tf.forward().z).normalize_or_zero();
        let k = cam.dist * 0.0018;
        let delta = -right * g.motion.x * k + fwd * g.motion.y * k;
        cam.focus += delta;
    }
    if g.scroll != 0.0 {
        cam.dist = (cam.dist * (1.0 - g.scroll * 0.12)).clamp(3.0, 600.0);
    }
    // Keyboard: Q/E to turn, +/- to zoom (only when the UI is not typing).
    if !egui.wants_any_keyboard_input() {
        let dt = time.delta_secs();
        if keys.pressed(KeyCode::KeyQ) {
            cam.yaw += dt * 1.5;
        }
        if keys.pressed(KeyCode::KeyE) {
            cam.yaw -= dt * 1.5;
        }
        if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
            cam.dist = (cam.dist * (1.0 - dt * 1.5)).max(3.0);
        }
        if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
            cam.dist = (cam.dist * (1.0 + dt * 1.5)).min(600.0);
        }
    }
    *tf = cam.transform();
}
