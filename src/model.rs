//! A build: a list of placed bricks on the stud grid.

use crate::parts::{Part, PLATE_H, STUD};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A placed part. `(x, z)` is the minimum corner of the rotated footprint in
/// studs, `y` is the bottom in plates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Brick {
    pub id: u32,
    pub part: Part,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Quarter turns around the vertical axis (0..4).
    pub rot: u8,
    pub color: u8,
    /// Section of a ready-made build (used to order instructions); 0 for user bricks.
    #[serde(default)]
    pub group: u16,
}

impl Brick {
    pub fn new(part: Part, x: i32, y: i32, z: i32, rot: u8, color: u8) -> Self {
        Brick { id: 0, part, x, y, z, rot: rot % 4, color, group: 0 }
    }

    /// Footprint size after rotation.
    pub fn footprint(&self) -> (i32, i32) {
        let (w, l) = self.part.size();
        if self.rot % 2 == 0 {
            (w, l)
        } else {
            (l, w)
        }
    }

    pub fn top(&self) -> i32 {
        self.y + self.part.height()
    }

    /// Map a local cell of the unrotated part to a world grid cell.
    pub fn local_to_world(&self, i: i32, j: i32) -> (i32, i32) {
        let (w, l) = self.part.size();
        match self.rot % 4 {
            0 => (self.x + i, self.z + j),
            1 => (self.x + j, self.z + w - 1 - i),
            2 => (self.x + w - 1 - i, self.z + l - 1 - j),
            _ => (self.x + l - 1 - j, self.z + i),
        }
    }

    /// All world cells covered by the footprint.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (fw, fl) = self.footprint();
        (0..fw).flat_map(move |a| (0..fl).map(move |b| (self.x + a, self.z + b)))
    }

    pub fn stud_world(&self) -> Vec<(i32, i32)> {
        self.part
            .stud_cells()
            .into_iter()
            .map(|(i, j)| self.local_to_world(i, j))
            .collect()
    }

    pub fn socket_world(&self) -> Vec<(i32, i32)> {
        self.part
            .socket_cells()
            .into_iter()
            .map(|(i, j)| self.local_to_world(i, j))
            .collect()
    }

    /// World-space centre of the footprint at the bottom of the part.
    pub fn world_origin(&self) -> [f32; 3] {
        let (fw, fl) = self.footprint();
        [
            (self.x as f32 + fw as f32 * 0.5) * STUD,
            self.y as f32 * PLATE_H,
            (self.z as f32 + fl as f32 * 0.5) * STUD,
        ]
    }

    /// Rotation angle around +Y in radians.
    pub fn angle(&self) -> f32 {
        self.rot as f32 * std::f32::consts::FRAC_PI_2
    }

    /// Axis aligned bounds in world units (min, max).
    pub fn aabb(&self) -> ([f32; 3], [f32; 3]) {
        let (fw, fl) = self.footprint();
        (
            [self.x as f32 * STUD, self.y as f32 * PLATE_H, self.z as f32 * STUD],
            [
                (self.x + fw) as f32 * STUD,
                self.top() as f32 * PLATE_H,
                (self.z + fl) as f32 * STUD,
            ],
        )
    }

    /// Does this brick share any volume with `o`?
    pub fn overlaps(&self, o: &Brick) -> bool {
        let (aw, al) = self.footprint();
        let (bw, bl) = o.footprint();
        self.x < o.x + bw
            && o.x < self.x + aw
            && self.z < o.z + bl
            && o.z < self.z + al
            && self.y < o.top()
            && o.y < self.top()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub name: String,
    pub bricks: Vec<Brick>,
    #[serde(default)]
    pub next_id: u32,
}

/// Saved file wrapper so the format can evolve.
#[derive(Serialize, Deserialize)]
struct SaveFile {
    format: String,
    version: u32,
    model: Model,
}

impl Model {
    pub fn new(name: &str) -> Self {
        Model { name: name.to_string(), bricks: Vec::new(), next_id: 1 }
    }

    /// Empty build with a 2x2 field of baseplates.
    pub fn with_baseplates(name: &str, nx: i32, nz: i32) -> Self {
        let mut m = Model::new(name);
        let s = crate::parts::BASEPLATE_SIZE as i32;
        for i in 0..nx {
            for k in 0..nz {
                m.add(Brick::new(Part::Baseplate, i * s, 0, k * s, 0, crate::parts::col::GREEN));
            }
        }
        m
    }

    pub fn add(&mut self, mut b: Brick) -> u32 {
        if self.next_id == 0 {
            self.next_id = 1;
        }
        b.id = self.next_id;
        self.next_id += 1;
        self.bricks.push(b);
        b.id
    }

    pub fn remove_ids(&mut self, ids: &[u32]) {
        self.bricks.retain(|b| !ids.contains(&b.id));
    }

    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.bricks.iter().position(|b| b.id == id)
    }

    /// Number of parts excluding baseplates.
    pub fn brick_count(&self) -> usize {
        self.bricks.iter().filter(|b| b.part != Part::Baseplate).count()
    }

    /// Would `b` collide with any brick (ignoring ids in `ignore`)?
    pub fn collides(&self, b: &Brick, ignore: &[u32]) -> bool {
        self.bricks.iter().any(|o| !ignore.contains(&o.id) && o.overlaps(b))
    }

    /// Ensure ids are unique and next_id is valid (after loading files).
    pub fn normalize_ids(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut next = self.bricks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        for b in &mut self.bricks {
            if b.id == 0 || !seen.insert(b.id) {
                b.id = next;
                seen.insert(next);
                next += 1;
            }
        }
        self.next_id = self.next_id.max(next);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&SaveFile {
            format: "brick-builder".into(),
            version: 1,
            model: self.clone(),
        })
        .expect("serialize")
    }

    pub fn from_json(s: &str) -> Result<Model, String> {
        let f: SaveFile = serde_json::from_str(s).map_err(|e| format!("Not a valid build file: {e}"))?;
        if f.format != "brick-builder" {
            return Err("Not a Brick Builder file".into());
        }
        let mut m = f.model;
        m.normalize_ids();
        Ok(m)
    }

    /// Bounds of all bricks in world units.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.bricks.iter();
        let first = it.next()?.aabb();
        Some(it.fold(first, |(mn, mx), b| {
            let (a, c) = b.aabb();
            (
                [mn[0].min(a[0]), mn[1].min(a[1]), mn[2].min(a[2])],
                [mx[0].max(c[0]), mx[1].max(c[1]), mx[2].max(c[2])],
            )
        }))
    }
}

/// Spatial index of occupied volume cells (x, z, plate) -> brick index.
pub struct Occupancy {
    pub cells: HashMap<(i32, i32, i32), usize>,
}

impl Occupancy {
    pub fn build(bricks: &[Brick]) -> Self {
        let mut cells = HashMap::new();
        for (i, b) in bricks.iter().enumerate() {
            for (x, z) in b.cells() {
                for y in b.y..b.top() {
                    cells.insert((x, z, y), i);
                }
            }
        }
        Occupancy { cells }
    }

    pub fn is_free(&self, b: &Brick) -> bool {
        b.cells().all(|(x, z)| (b.y..b.top()).all(|y| !self.cells.contains_key(&(x, z, y))))
    }
}

/// Count parts by (part, colour), sorted for display.
pub fn parts_list<'a>(bricks: impl Iterator<Item = &'a Brick>) -> Vec<(Part, u8, usize)> {
    let mut map: HashMap<(Part, u8), usize> = HashMap::new();
    for b in bricks {
        *map.entry((b.part, b.color)).or_default() += 1;
    }
    let mut v: Vec<_> = map.into_iter().map(|((p, c), n)| (p, c, n)).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    v
}
