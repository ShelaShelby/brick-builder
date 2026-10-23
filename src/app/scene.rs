//! 3D scene: part meshes, materials and keeping one entity per brick in sync.
//!
//! All bricks of the same part and colour share one mesh and one material, so
//! Bevy batches them into instanced draws even for very large builds.

use super::camera::OrbitCam;
use super::state::{Checked, Doc, Editor, Manual, Screen, Ui};
use bevy::asset::RenderAssetUsages;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use brick_builder::connect::Status;
use brick_builder::meshgen::{part_mesh, Lod};
use brick_builder::model::Brick;
use brick_builder::parts::{color_rgb, Part, PLATE_H, STUD};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Vis {
    Normal,
    Weak,
    Floating,
    Selected,
    Dim,
    New,
}

#[derive(Resource)]
pub struct BrickAssets {
    pub meshes: HashMap<Part, Handle<Mesh>>,
    mats: HashMap<(u8, Vis), Handle<StandardMaterial>>,
    ghost: HashMap<u8, Handle<StandardMaterial>>,
}

fn to_mesh(part: Part) -> Mesh {
    let d = part_mesh(part, Lod::High);
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals)
        .with_inserted_indices(Indices::U32(d.indices))
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

impl BrickAssets {
    pub fn material(&mut self, mats: &mut Assets<StandardMaterial>, color: u8, vis: Vis) -> Handle<StandardMaterial> {
        self.mats
            .entry((color, vis))
            .or_insert_with(|| {
                let [r, g, b] = color_rgb(color);
                let base = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
                let (c, emissive) = match vis {
                    Vis::Normal => (base, [0.0; 3]),
                    Vis::New => (base, [base[0] * 0.08, base[1] * 0.08, base[2] * 0.08]),
                    Vis::Dim => (mix(base, [0.94, 0.95, 0.97], 0.62), [0.0; 3]),
                    Vis::Weak => (mix(base, [1.0, 0.55, 0.0], 0.6), [0.35, 0.16, 0.0]),
                    Vis::Floating => (mix(base, [1.0, 0.05, 0.05], 0.65), [0.4, 0.0, 0.0]),
                    Vis::Selected => (mix(base, [0.2, 0.8, 1.0], 0.35), [0.0, 0.12, 0.2]),
                };
                mats.add(StandardMaterial {
                    base_color: Color::srgb(c[0], c[1], c[2]),
                    emissive: LinearRgba::rgb(emissive[0], emissive[1], emissive[2]),
                    perceptual_roughness: 0.38,
                    reflectance: 0.35,
                    ..default()
                })
            })
            .clone()
    }

    pub fn ghost(&mut self, mats: &mut Assets<StandardMaterial>, color: u8) -> Handle<StandardMaterial> {
        self.ghost
            .entry(color)
            .or_insert_with(|| {
                let [r, g, b] = color_rgb(color);
                mats.add(StandardMaterial {
                    base_color: Color::srgba_u8(r, g, b, 140),
                    alpha_mode: AlphaMode::Blend,
                    emissive: LinearRgba::rgb(0.08, 0.08, 0.08),
                    ..default()
                })
            })
            .clone()
    }
}

#[derive(Component)]
pub struct BrickEnt;

#[derive(Component)]
pub struct Ghost;

pub fn brick_transform(b: &Brick) -> Transform {
    let o = b.world_origin();
    Transform::from_xyz(o[0], o[1], o[2]).with_rotation(Quat::from_rotation_y(b.angle()))
}

pub fn setup_scene(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mut assets = BrickAssets { meshes: HashMap::new(), mats: HashMap::new(), ghost: HashMap::new() };
    for p in Part::all() {
        assets.meshes.insert(p, meshes.add(to_mesh(p)));
    }
    // Ghost preview entity.
    let ghost_mat = assets.ghost(&mut mats, 4);
    commands.spawn((
        Mesh3d(assets.meshes[&Part::Brick { w: 2, l: 4 }].clone()),
        MeshMaterial3d(ghost_mat),
        Transform::default(),
        Visibility::Hidden,
        Ghost,
        bevy::pbr::NotShadowCaster,
    ));
    commands.insert_resource(assets);

    let cam = OrbitCam { focus: Vec3::new(32.0, 0.0, 32.0), yaw: 0.6, pitch: 0.6, dist: 70.0 };
    commands.spawn((Camera3d::default(), cam.transform(), cam, Msaa::Sample4));
    commands.spawn((
        DirectionalLight { illuminance: 6500.0, shadows_enabled: true, ..default() },
        Transform::from_xyz(-30.0, 80.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        CascadeShadowConfigBuilder { first_cascade_far_bound: 60.0, maximum_distance: 400.0, ..default() }.build(),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 1800.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(60.0, 40.0, -40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(AmbientLight { color: Color::WHITE, brightness: 450.0, ..default() });
    commands.insert_resource(ClearColor(Color::srgb(0.84, 0.89, 0.95)));
}

#[derive(Resource, Default)]
pub struct Spawned {
    map: HashMap<u32, (Entity, Brick, Vis, bool)>,
    doc_v: u64,
    view_v: u64,
    check_v: u64,
}

/// Desired look and visibility of brick `i`.
fn visual(i: usize, b: &Brick, ui: &Ui, editor: &Editor, checked: &Checked, manual: &Manual) -> (Vis, bool) {
    if ui.screen == Screen::Instructions {
        let s = manual.step_of.get(i).copied().unwrap_or(0);
        return if s < manual.step {
            (Vis::Dim, true)
        } else if s == manual.step {
            (Vis::New, true)
        } else {
            (Vis::Normal, false)
        };
    }
    if editor.selection.contains(&b.id) {
        return (Vis::Selected, true);
    }
    match checked.analysis.status.get(i) {
        Some(Status::Floating) => (Vis::Floating, true),
        Some(Status::Weak) => (Vis::Weak, true),
        _ => (Vis::Normal, true),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sync_scene(
    mut commands: Commands,
    doc: Res<Doc>,
    checked: Res<Checked>,
    editor: Res<Editor>,
    ui: Res<Ui>,
    manual: Res<Manual>,
    mut spawned: ResMut<Spawned>,
    mut assets: ResMut<BrickAssets>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    if spawned.doc_v == doc.version && spawned.view_v == editor.view_version && spawned.check_v == checked.version {
        return;
    }
    // Wait for the connection check of this version so flags never lag behind.
    if checked.version != doc.version {
        return;
    }
    spawned.doc_v = doc.version;
    spawned.view_v = editor.view_version;
    spawned.check_v = checked.version;

    let mut seen = std::collections::HashSet::with_capacity(doc.model.bricks.len());
    for (i, b) in doc.model.bricks.iter().enumerate() {
        seen.insert(b.id);
        let (vis, visible) = visual(i, b, &ui, &editor, &checked, &manual);
        match spawned.map.get_mut(&b.id) {
            Some((e, old, old_vis, old_visible)) => {
                let mut ec = commands.entity(*e);
                if old.part != b.part {
                    ec.insert(Mesh3d(assets.meshes[&b.part].clone()));
                }
                if old.x != b.x || old.y != b.y || old.z != b.z || old.rot != b.rot || old.part != b.part {
                    ec.insert(brick_transform(b));
                }
                if old.color != b.color || *old_vis != vis {
                    ec.insert(MeshMaterial3d(assets.material(&mut mats, b.color, vis)));
                }
                if *old_visible != visible {
                    ec.insert(if visible { Visibility::Inherited } else { Visibility::Hidden });
                }
                *old = *b;
                *old_vis = vis;
                *old_visible = visible;
            }
            None => {
                let e = commands
                    .spawn((
                        Mesh3d(assets.meshes[&b.part].clone()),
                        MeshMaterial3d(assets.material(&mut mats, b.color, vis)),
                        brick_transform(b),
                        if visible { Visibility::Inherited } else { Visibility::Hidden },
                        BrickEnt,
                    ))
                    .id();
                spawned.map.insert(b.id, (e, *b, vis, visible));
            }
        }
    }
    let gone: Vec<u32> = spawned.map.keys().filter(|id| !seen.contains(id)).copied().collect();
    for id in gone {
        if let Some((e, ..)) = spawned.map.remove(&id) {
            commands.entity(e).despawn();
        }
    }
}

pub fn update_ghost(
    editor: Res<Editor>,
    ui: Res<Ui>,
    mut assets: ResMut<BrickAssets>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(&mut Transform, &mut Visibility, &mut Mesh3d, &mut MeshMaterial3d<StandardMaterial>), With<Ghost>>,
) {
    let Ok((mut tf, mut vis, mut mesh, mut mat)) = q.single_mut() else { return };
    match (&editor.ghost, ui.screen) {
        (Some(g), Screen::Editor) => {
            *tf = brick_transform(g);
            // Lift a hair so the ghost does not z-fight with bricks it touches.
            tf.translation.y += 0.002;
            if mesh.0 != assets.meshes[&g.part] {
                mesh.0 = assets.meshes[&g.part].clone();
            }
            let m = assets.ghost(&mut mats, g.color);
            if mat.0 != m {
                mat.0 = m;
            }
            *vis = Visibility::Inherited;
        }
        _ => *vis = Visibility::Hidden,
    }
}

/// Grid lines on the baseplates and outlines for flagged, selected and new bricks.
pub fn draw_gizmos(mut gizmos: Gizmos, doc: Res<Doc>, checked: Res<Checked>, editor: Res<Editor>, ui: Res<Ui>, manual: Res<Manual>) {
    let outline = |gizmos: &mut Gizmos, b: &Brick, c: Color, grow: f32| {
        let (mn, mx) = b.aabb();
        let mn = Vec3::from(mn) - Vec3::splat(grow);
        let mx = Vec3::from(mx) + Vec3::splat(grow);
        gizmos.cuboid(Transform::from_translation((mn + mx) * 0.5).with_scale(mx - mn), c);
    };
    // Baseplate grid: a line every 4 studs.
    for b in doc.model.bricks.iter().filter(|b| b.part == Part::Baseplate) {
        let (mn, mx) = b.aabb();
        let y = mx[1] + 0.003;
        let c = Color::srgba(1.0, 1.0, 1.0, 0.1);
        let mut x = mn[0];
        while x <= mx[0] + 0.01 {
            gizmos.line(Vec3::new(x, y, mn[2]), Vec3::new(x, y, mx[2]), c);
            x += 4.0 * STUD;
        }
        let mut z = mn[2];
        while z <= mx[2] + 0.01 {
            gizmos.line(Vec3::new(mn[0], y, z), Vec3::new(mx[0], y, z), c);
            z += 4.0 * STUD;
        }
    }
    match ui.screen {
        Screen::Editor => {
            for (i, b) in doc.model.bricks.iter().enumerate() {
                match checked.analysis.status.get(i) {
                    Some(Status::Floating) if checked.version == doc.version => {
                        outline(&mut gizmos, b, Color::srgb(1.0, 0.1, 0.1), 0.03)
                    }
                    Some(Status::Weak) if checked.version == doc.version => outline(&mut gizmos, b, Color::srgb(1.0, 0.55, 0.0), 0.03),
                    _ => {}
                }
                if editor.selection.contains(&b.id) {
                    outline(&mut gizmos, b, Color::srgb(0.1, 0.75, 1.0), 0.05);
                }
            }
            if let Some(h) = editor.hover.and_then(|id| doc.model.bricks.iter().find(|b| b.id == id)) {
                if h.part != Part::Baseplate {
                    outline(&mut gizmos, h, Color::srgba(1.0, 1.0, 1.0, 0.8), 0.02);
                }
            }
        }
        Screen::Instructions => {
            for (i, b) in doc.model.bricks.iter().enumerate() {
                if manual.step_of.get(i) == Some(&manual.step) && b.part != Part::Baseplate {
                    outline(&mut gizmos, b, Color::srgb(1.0, 0.85, 0.0), 0.03);
                }
            }
        }
        Screen::Picker => {}
    }
    let _ = PLATE_H;
}
