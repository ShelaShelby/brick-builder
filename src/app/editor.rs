//! Editing: picking bricks under the mouse, the snapping ghost, placing,
//! removing, selecting, moving, rotating, undo/redo.

use super::camera::{Gestures, OrbitCam};
use super::state::{Doc, Editor, Manual, MoveDrag, Screen, Toasts, Tool, Ui};
use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;
use brick_builder::model::{Brick, Model};
use brick_builder::parts::{Part, BASEPLATE_SIZE, PLATE_H, STUD};
use std::collections::HashSet;

pub struct Hit {
    pub index: Option<usize>,
    pub point: Vec3,
    pub normal: Vec3,
}

/// Nearest brick (or the ground plane) hit by the ray.
pub fn raycast(bricks: &[Brick], ray: Ray3d) -> Option<Hit> {
    let o = ray.origin;
    let d = *ray.direction;
    let mut best: Option<(f32, Hit)> = None;
    for (i, b) in bricks.iter().enumerate() {
        let (mn, mx) = b.aabb();
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;
        let mut axis = 0;
        let mut ok = true;
        for a in 0..3 {
            if d[a].abs() < 1e-9 {
                if o[a] < mn[a] || o[a] > mx[a] {
                    ok = false;
                    break;
                }
                continue;
            }
            let t1 = (mn[a] - o[a]) / d[a];
            let t2 = (mx[a] - o[a]) / d[a];
            let (n, f) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
            if n > tmin {
                tmin = n;
                axis = a;
            }
            tmax = tmax.min(f);
        }
        if !ok || tmin > tmax || tmax < 0.0 || tmin < 0.0 {
            continue;
        }
        if best.as_ref().is_some_and(|(t, _)| *t <= tmin) {
            continue;
        }
        let mut normal = Vec3::ZERO;
        normal[axis] = -d[axis].signum();
        best = Some((tmin, Hit { index: Some(i), point: o + d * tmin, normal }));
    }
    if d.y < -1e-6 {
        let t = -o.y / d.y;
        if t > 0.0 && best.as_ref().is_none_or(|(bt, _)| t < *bt) {
            best = Some((t, Hit { index: None, point: o + d * t, normal: Vec3::Y }));
        }
    }
    best.map(|(_, h)| h)
}

/// Where a new part would go for this hit. Snaps to the stud grid and moves
/// up until it does not collide with anything.
pub fn place_target(model: &Model, hit: &Hit, part: Part, rot: u8, color: u8) -> Option<Brick> {
    let p = hit.point + hit.normal * 0.02;
    let cx = (p.x / STUD).floor() as i32;
    let cz = (p.z / STUD).floor() as i32;
    let mut cand = Brick::new(part, 0, 0, 0, rot, color);
    let (fw, fl) = cand.footprint();
    if part == Part::Baseplate {
        let s = BASEPLATE_SIZE as i32;
        cand.x = cx.div_euclid(s) * s;
        cand.z = cz.div_euclid(s) * s;
        cand.y = 0;
        return (!model.collides(&cand, &[])).then_some(cand);
    }
    cand.x = cx - (fw - 1) / 2;
    cand.z = cz - (fl - 1) / 2;
    cand.y = match hit.index {
        None => 0,
        Some(i) => {
            let b = &model.bricks[i];
            if hit.normal.y > 0.5 {
                b.top()
            } else if hit.normal.y < -0.5 {
                (b.y - part.height()).max(0)
            } else {
                b.y
            }
        }
    };
    if hit.index.is_none() {
        // On bare ground, sit on top of whatever is below (usually nothing).
        cand.y = 0;
    }
    for _ in 0..64 {
        let blockers: Vec<&Brick> = model.bricks.iter().filter(|o| o.overlaps(&cand)).collect();
        if blockers.is_empty() {
            return Some(cand);
        }
        cand.y = blockers.iter().map(|o| o.top()).max().unwrap_or(cand.y + 1);
    }
    None
}

fn camera_ray(cam: &Camera, gt: &GlobalTransform, cursor: Vec2) -> Option<Ray3d> {
    cam.viewport_to_world(gt, cursor).ok()
}

/// Hover highlight and ghost preview.
pub fn hover(
    g: Res<Gestures>,
    doc: Res<Doc>,
    ui: Res<Ui>,
    mut editor: ResMut<Editor>,
    cam: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
) {
    let Ok((cam, gt)) = cam.single() else { return };
    let hit = if ui.screen == Screen::Editor && !g.over_ui && !g.orbiting && !g.panning {
        g.cursor.and_then(|c| camera_ray(cam, gt, c)).and_then(|r| raycast(&doc.model.bricks, r))
    } else {
        None
    };
    let hover = hit.as_ref().and_then(|h| h.index).map(|i| doc.model.bricks[i].id);
    if editor.hover != hover {
        editor.hover = hover;
    }
    let ghost = match (&hit, editor.tool, &editor.drag, editor.box_select) {
        (Some(h), Tool::Build, None, None) => place_target(&doc.model, h, editor.part, editor.rot, editor.color),
        _ => None,
    };
    if editor.ghost != ghost {
        editor.ghost = ghost;
    }
}

fn ctrl(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
}

fn shift(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
}

/// Try to move the given bricks; fails if they would hit anything else.
fn try_move(model: &mut Model, originals: &[Brick], dx: i32, dy: i32, dz: i32) -> bool {
    let ids: Vec<u32> = originals.iter().map(|b| b.id).collect();
    let moved: Vec<Brick> = originals
        .iter()
        .map(|b| Brick { x: b.x + dx, y: b.y + dy, z: b.z + dz, ..*b })
        .collect();
    if moved.iter().any(|b| b.y < 0) {
        return false;
    }
    if moved.iter().any(|b| model.collides(b, &ids)) {
        return false;
    }
    for m in moved {
        if let Some(i) = model.index_of(m.id) {
            model.bricks[i] = m;
        }
    }
    true
}

/// Rotate bricks a quarter turn around the centre of their footprint.
fn try_rotate(model: &mut Model, originals: &[Brick]) -> bool {
    if originals.is_empty() {
        return false;
    }
    let (mut x0, mut z0, mut x1, mut z1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for b in originals {
        let (fw, fl) = b.footprint();
        x0 = x0.min(b.x);
        z0 = z0.min(b.z);
        x1 = x1.max(b.x + fw);
        z1 = z1.max(b.z + fl);
    }
    // Rotation by -90 degrees around Y maps (x, z) -> (-z, x) in doubled coordinates.
    let cx2 = x0 + x1;
    let cz2 = z0 + z1;
    let rotated: Vec<Brick> = originals
        .iter()
        .map(|b| {
            let (fw, fl) = b.footprint();
            // centre of the footprint, doubled
            let bx2 = 2 * b.x + fw - cx2;
            let bz2 = 2 * b.z + fl - cz2;
            let (nx2, nz2) = (bz2, -bx2);
            let (nfw, nfl) = (fl, fw);
            let nx = (nx2 + cx2 - nfw).div_euclid(2);
            let nz = (nz2 + cz2 - nfl).div_euclid(2);
            Brick { x: nx, z: nz, rot: (b.rot + 1) % 4, ..*b }
        })
        .collect();
    let ids: Vec<u32> = originals.iter().map(|b| b.id).collect();
    if rotated.iter().any(|b| model.collides(b, &ids)) {
        return false;
    }
    for r in rotated {
        if let Some(i) = model.index_of(r.id) {
            model.bricks[i] = r;
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub fn edit(
    g: Res<Gestures>,
    keys: Res<ButtonInput<KeyCode>>,
    egui: Res<EguiWantsInput>,
    time: Res<Time>,
    mut doc: ResMut<Doc>,
    mut editor: ResMut<Editor>,
    mut toasts: ResMut<Toasts>,
    ui: Res<Ui>,
    cam: Query<(&Camera, &GlobalTransform), With<OrbitCam>>,
) {
    if ui.screen != Screen::Editor {
        editor.drag = None;
        editor.box_select = None;
        return;
    }
    let Ok((cam, gt)) = cam.single() else { return };
    let typing = egui.wants_any_keyboard_input();
    let now = time.elapsed_secs_f64();

    // ---- keyboard ----
    if !typing {
        if ctrl(&keys) && keys.just_pressed(KeyCode::KeyZ) {
            let ok = if shift(&keys) { doc.redo() } else { doc.undo() };
            if !ok {
                toasts.push(now, "Nothing to undo");
            }
            editor.view_version += 1;
        }
        if ctrl(&keys) && keys.just_pressed(KeyCode::KeyY) && !doc.redo() {
            toasts.push(now, "Nothing to redo");
        }
        if ctrl(&keys) && keys.just_pressed(KeyCode::KeyA) {
            let all = doc.model.bricks.iter().filter(|b| b.part != Part::Baseplate).map(|b| b.id).collect();
            editor.tool = Tool::Select;
            editor.set_selection(all);
        }
        if !ctrl(&keys) {
            if keys.just_pressed(KeyCode::KeyB) {
                editor.tool = Tool::Build;
            }
            if keys.just_pressed(KeyCode::KeyV) || keys.just_pressed(KeyCode::KeyS) {
                editor.tool = Tool::Select;
            }
        }
        if keys.just_pressed(KeyCode::Escape) {
            editor.set_selection(HashSet::new());
        }
        if keys.just_pressed(KeyCode::KeyR) && !ctrl(&keys) {
            if editor.tool == Tool::Select && !editor.selection.is_empty() {
                let sel: Vec<Brick> = doc.model.bricks.iter().filter(|b| editor.selection.contains(&b.id)).copied().collect();
                let before = doc.model.clone();
                if try_rotate(&mut doc.model, &sel) {
                    let after = std::mem::replace(&mut doc.model, before);
                    doc.snapshot();
                    doc.model = after;
                    doc.changed();
                } else {
                    toasts.push(now, "No room to rotate here");
                }
            } else {
                editor.rot = (editor.rot + 1) % 4;
            }
        }
        if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
            let ids: Vec<u32> = if !editor.selection.is_empty() {
                editor.selection.iter().copied().collect()
            } else {
                editor.hover.into_iter().collect()
            };
            if !ids.is_empty() {
                doc.snapshot();
                doc.model.remove_ids(&ids);
                doc.changed();
                editor.set_selection(HashSet::new());
            }
        }
        // Move the selection with the arrow keys (relative to the view) and PageUp/PageDown.
        if !editor.selection.is_empty() {
            let mut d = IVec3::ZERO;
            let fwd = gt.forward();
            let (f, r) = if fwd.x.abs() > fwd.z.abs() {
                (IVec3::new(fwd.x.signum() as i32, 0, 0), IVec3::new(0, 0, fwd.x.signum() as i32))
            } else {
                (IVec3::new(0, 0, fwd.z.signum() as i32), IVec3::new(-fwd.z.signum() as i32, 0, 0))
            };
            if keys.just_pressed(KeyCode::ArrowUp) {
                d += f;
            }
            if keys.just_pressed(KeyCode::ArrowDown) {
                d -= f;
            }
            if keys.just_pressed(KeyCode::ArrowRight) {
                d += r;
            }
            if keys.just_pressed(KeyCode::ArrowLeft) {
                d -= r;
            }
            if keys.just_pressed(KeyCode::PageUp) {
                d.y += 1;
            }
            if keys.just_pressed(KeyCode::PageDown) {
                d.y -= 1;
            }
            if d != IVec3::ZERO {
                let sel: Vec<Brick> = doc.model.bricks.iter().filter(|b| editor.selection.contains(&b.id)).copied().collect();
                let before = doc.model.clone();
                if try_move(&mut doc.model, &sel, d.x, d.y, d.z) {
                    let after = std::mem::replace(&mut doc.model, before);
                    doc.snapshot();
                    doc.model = after;
                    doc.changed();
                }
            }
        }
    }

    // ---- mouse ----
    if g.right_click {
        // Right-click never removes a whole baseplate (use Delete for that).
        let target = editor.hover.filter(|id| doc.model.bricks.iter().any(|b| b.id == *id && b.part != Part::Baseplate));
        if let Some(id) = target {
            doc.snapshot();
            doc.model.remove_ids(&[id]);
            doc.changed();
            if editor.selection.contains(&id) {
                let mut s = editor.selection.clone();
                s.remove(&id);
                editor.set_selection(s);
            }
        }
    }

    match editor.tool {
        Tool::Build => {
            if g.left_pressed {
                if let Some(gb) = editor.ghost {
                    doc.snapshot();
                    doc.model.add(gb);
                    doc.changed();
                }
            }
        }
        Tool::Select => {
            let hovered = editor
                .hover
                .and_then(|id| doc.model.bricks.iter().find(|b| b.id == id))
                .filter(|b| b.part != Part::Baseplate)
                .copied();
            if g.left_pressed {
                match hovered {
                    Some(b) => {
                        let mut sel = editor.selection.clone();
                        if shift(&keys) {
                            if !sel.remove(&b.id) {
                                sel.insert(b.id);
                            }
                        } else if !sel.contains(&b.id) {
                            sel = HashSet::from([b.id]);
                        }
                        editor.set_selection(sel);
                        if editor.selection.contains(&b.id) {
                            let plane_y = b.y as f32 * PLATE_H;
                            if let Some(start) = g.cursor.and_then(|c| plane_point(cam, gt, c, plane_y)) {
                                let originals = doc.model.bricks.iter().filter(|x| editor.selection.contains(&x.id)).copied().collect();
                                editor.drag = Some(MoveDrag { plane_y, start, originals, before: doc.model.clone(), moved: false });
                            }
                        }
                    }
                    None => {
                        if let Some(c) = g.cursor {
                            editor.box_select = Some((c, c));
                        }
                    }
                }
            }
            if g.left_down {
                if let (Some((a, _)), Some(c)) = (editor.box_select, g.cursor) {
                    editor.box_select = Some((a, c));
                }
                if let (Some(drag), Some(c)) = (editor.drag.as_mut(), g.cursor) {
                    if let Some(p) = plane_point(cam, gt, c, drag.plane_y) {
                        let dx = ((p.x - drag.start.x) / STUD).round() as i32;
                        let dz = ((p.z - drag.start.z) / STUD).round() as i32;
                        let originals = drag.originals.clone();
                        let current = doc.model.bricks.iter().find(|b| b.id == originals[0].id).copied();
                        let want = (originals[0].x + dx, originals[0].z + dz);
                        if current.is_some_and(|c| (c.x, c.z) != want) && try_move(&mut doc.model, &originals, dx, 0, dz) {
                            drag.moved = true;
                            doc.version += 1;
                        }
                    }
                }
            }
            if g.left_released {
                if let Some(drag) = editor.drag.take() {
                    if drag.moved {
                        let after = std::mem::replace(&mut doc.model, drag.before);
                        doc.snapshot();
                        doc.model = after;
                        doc.changed();
                    }
                }
                if let Some((a, b)) = editor.box_select.take() {
                    let min = a.min(b);
                    let max = a.max(b);
                    let mut sel = if shift(&keys) { editor.selection.clone() } else { HashSet::new() };
                    if (max - min).length() > 4.0 {
                        for br in doc.model.bricks.iter().filter(|b| b.part != Part::Baseplate) {
                            let o = br.world_origin();
                            let centre = Vec3::new(o[0], o[1] + br.part.height() as f32 * PLATE_H * 0.5, o[2]);
                            if let Ok(s) = cam.world_to_viewport(gt, centre) {
                                if s.x >= min.x && s.x <= max.x && s.y >= min.y && s.y <= max.y {
                                    sel.insert(br.id);
                                }
                            }
                        }
                    }
                    editor.set_selection(sel);
                }
            }
        }
    }
}

fn plane_point(cam: &Camera, gt: &GlobalTransform, cursor: Vec2, y: f32) -> Option<Vec3> {
    let ray = cam.viewport_to_world(gt, cursor).ok()?;
    let d = *ray.direction;
    if d.y.abs() < 1e-5 {
        return None;
    }
    let t = (y - ray.origin.y) / d.y;
    (t > 0.0).then(|| ray.origin + d * t)
}

/// Arrow keys and slider in the instruction viewer.
pub fn manual_keys(keys: Res<ButtonInput<KeyCode>>, ui: Res<Ui>, mut manual: ResMut<Manual>, mut editor: ResMut<Editor>, egui: Res<EguiWantsInput>) {
    if ui.screen != Screen::Instructions || egui.wants_any_keyboard_input() {
        return;
    }
    let n = manual.ins.len();
    if n == 0 {
        return;
    }
    let mut s = manual.step;
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::PageDown) {
        s = (s + 1).min(n - 1);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::PageUp) {
        s = s.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::Home) {
        s = 0;
    }
    if keys.just_pressed(KeyCode::End) {
        s = n - 1;
    }
    if s != manual.step {
        manual.step = s;
        editor.view_version += 1;
    }
}
