//! App-wide resources: the open build, undo history, editor settings, screens.

use bevy::prelude::*;
use brick_builder::builds;
use brick_builder::connect::{self, Analysis};
use brick_builder::instructions::{self, Instructions};
use brick_builder::model::{Brick, Model};
use brick_builder::parts::{col, Part};
use brick_builder::raster;
use std::collections::HashSet;
use std::path::PathBuf;

const UNDO_LIMIT: usize = 300;

/// The build being edited, with undo/redo snapshots.
#[derive(Resource)]
pub struct Doc {
    pub model: Model,
    undo: Vec<Model>,
    redo: Vec<Model>,
    /// Bumped on every change so other systems can react.
    pub version: u64,
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

impl Doc {
    pub fn new(model: Model) -> Self {
        Doc { model, undo: Vec::new(), redo: Vec::new(), version: 1, path: None, dirty: false }
    }

    /// Call before changing the model.
    pub fn snapshot(&mut self) {
        self.undo.push(self.model.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Call after changing the model.
    pub fn changed(&mut self) {
        self.version += 1;
        self.dirty = true;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(m) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.model, m));
            self.changed();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(m) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.model, m));
            self.changed();
            true
        } else {
            false
        }
    }

    /// Replace the whole build (new / open / ready-made).
    pub fn replace(&mut self, model: Model, path: Option<PathBuf>) {
        self.model = model;
        self.undo.clear();
        self.redo.clear();
        self.path = path;
        self.version += 1;
        self.dirty = false;
    }
}

/// Connection check of the current build, recomputed when the build changes.
#[derive(Resource, Default)]
pub struct Checked {
    pub analysis: Analysis,
    pub version: u64,
}

pub fn update_check(doc: Res<Doc>, mut checked: ResMut<Checked>) {
    if checked.version != doc.version {
        checked.analysis = connect::analyze(&doc.model.bricks);
        checked.version = doc.version;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Build,
    Select,
}

/// Dragging the selection around.
pub struct MoveDrag {
    pub plane_y: f32,
    pub start: Vec3,
    pub originals: Vec<Brick>,
    pub before: Model,
    pub moved: bool,
}

#[derive(Resource)]
pub struct Editor {
    pub tool: Tool,
    pub part: Part,
    pub color: u8,
    pub rot: u8,
    pub selection: HashSet<u32>,
    pub hover: Option<u32>,
    pub ghost: Option<Brick>,
    pub box_select: Option<(Vec2, Vec2)>,
    pub drag: Option<MoveDrag>,
    /// Bumped when anything that changes how bricks look (selection, screen, step) changes.
    pub view_version: u64,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            tool: Tool::Build,
            part: Part::Brick { w: 2, l: 4 },
            color: col::RED,
            rot: 0,
            selection: HashSet::new(),
            hover: None,
            ghost: None,
            box_select: None,
            drag: None,
            view_version: 1,
        }
    }
}

impl Editor {
    pub fn set_selection(&mut self, sel: HashSet<u32>) {
        if sel != self.selection {
            self.selection = sel;
            self.view_version += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Picker,
    Editor,
    Instructions,
}

#[derive(Resource, Default)]
pub struct Ui {
    pub screen: Screen,
    pub show_help: bool,
}

/// Instructions being viewed.
#[derive(Resource, Default)]
pub struct Manual {
    pub ins: Instructions,
    pub step: usize,
    pub step_of: Vec<usize>,
    /// Screen to return to when closing the viewer.
    pub back_to: Screen,
}

impl Manual {
    pub fn open(&mut self, model: &Model, back_to: Screen) {
        self.ins = instructions::generate(&model.bricks);
        self.step_of = self.ins.step_of(model.bricks.len());
        self.step = 0;
        self.back_to = back_to;
    }
}

/// A ready-made build with its model and thumbnail.
pub struct LibraryEntry {
    pub name: &'static str,
    pub size: &'static str,
    pub blurb: &'static str,
    pub model: Model,
    pub steps: usize,
    pub thumb: raster::Image,
}

#[derive(Resource)]
pub struct Library {
    pub entries: Vec<LibraryEntry>,
}

impl Library {
    pub fn load() -> Self {
        let r = raster::Renderer::new();
        let entries = builds::catalog()
            .into_iter()
            .map(|info| {
                let model = (info.make)();
                let steps = instructions::generate(&model.bricks).len();
                let frame: Vec<Brick> = model.bricks.iter().filter(|b| b.part != Part::Baseplate).copied().collect();
                let looks = vec![raster::Look::Normal; model.bricks.len()];
                let view = raster::View { background: [0.93, 0.95, 0.98], margin: 0.02, ..Default::default() };
                let thumb = r.render(&model.bricks, &looks, &frame, 360, 250, &view);
                LibraryEntry { name: info.name, size: info.size, blurb: info.blurb, model, steps, thumb }
            })
            .collect();
        Library { entries }
    }
}

/// Short messages shown at the bottom of the screen.
#[derive(Resource, Default)]
pub struct Toasts {
    pub items: Vec<(String, f64)>,
}

impl Toasts {
    pub fn push(&mut self, now: f64, text: impl Into<String>) {
        self.items.push((text.into(), now + 3.5));
    }
}

/// Request to point the camera at the whole build.
#[derive(Resource, Default)]
pub struct FrameRequest(pub bool);

pub fn empty_build() -> Model {
    Model::with_baseplates("My build", 2, 2)
}
