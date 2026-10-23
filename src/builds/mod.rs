//! Ready-made builds, generated in code.
//!
//! Designs are described as coloured voxels on the stud grid (one stud wide,
//! one brick or one plate tall) plus hand-placed detail parts. The voxels are
//! then turned into real bricks with a running-bond layout so courses
//! interlock, and the result is checked with the connection check.

mod kit;
mod large;
mod medium;
mod small;
mod xl;
mod xxl;

use crate::connect::{analyze, Status};
use crate::model::{Brick, Model};
use crate::parts::{Part, BASEPLATE_SIZE};
use std::collections::HashMap;

pub struct BuildInfo {
    pub name: &'static str,
    pub size: &'static str,
    pub blurb: &'static str,
    pub make: fn() -> Model,
}

pub fn catalog() -> Vec<BuildInfo> {
    vec![
        BuildInfo { name: "Cozy Cottage", size: "Small", blurb: "A little house with a garden.", make: small::build },
        BuildInfo { name: "Harbour Lighthouse", size: "Medium", blurb: "Striped lighthouse on a rocky island.", make: medium::build },
        BuildInfo { name: "Castle Gate", size: "Large", blurb: "Gatehouse with two towers and a moat.", make: large::build },
        BuildInfo { name: "Railway Station", size: "XL", blurb: "Station hall, platform and a train.", make: xl::build },
        BuildInfo { name: "Town Square", size: "XXL", blurb: "Clock tower, canal, bridges and houses.", make: xxl::build },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Brick,
    Plate,
}

impl Kind {
    fn height(self) -> i32 {
        match self {
            Kind::Brick => 3,
            Kind::Plate => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Cell {
    kind: Kind,
    color: u8,
    group: u16,
}

pub struct Builder {
    name: String,
    parts: Vec<Brick>,
    vox: HashMap<(i32, i32, i32), Cell>,
    /// Running bond period in studs (4 or 6).
    pub period: i32,
    /// Section tag applied to everything added from now on.
    pub group: u16,
    /// Number of bricks dropped by the final safety check (should be 0).
    pub pruned: usize,
}

impl Builder {
    /// New build on an `nx` x `nz` field of baseplates.
    pub fn new(name: &str, nx: i32, nz: i32, base_color: u8) -> Self {
        let mut b = Builder {
            name: name.into(),
            parts: Vec::new(),
            vox: HashMap::new(),
            period: 4,
            group: 0,
            pruned: 0,
        };
        let s = BASEPLATE_SIZE as i32;
        for i in 0..nx {
            for k in 0..nz {
                b.part(Part::Baseplate, i * s, 0, k * s, 0, base_color);
            }
        }
        b
    }

    pub fn part(&mut self, part: Part, x: i32, y: i32, z: i32, rot: u8, color: u8) {
        let mut br = Brick::new(part, x, y, z, rot, color);
        br.group = self.group;
        self.parts.push(br);
    }

    pub fn brick(&mut self, x: i32, y: i32, z: i32, color: u8) {
        self.set(x, y, z, Kind::Brick, color);
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, kind: Kind, color: u8) {
        self.vox.insert((x, y, z), Cell { kind, color, group: self.group });
    }

    pub fn clear(&mut self, x: i32, y: i32, z: i32) {
        self.vox.remove(&(x, y, z));
    }

    pub fn has(&self, x: i32, y: i32, z: i32) -> bool {
        self.vox.contains_key(&(x, y, z))
    }

    /// Fill the rectangle [x0, x1) x [z0, z1) at height y with one layer.
    pub fn fill(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y: i32, kind: Kind, color: u8) {
        for x in x0..x1 {
            for z in z0..z1 {
                self.set(x, y, z, kind, color);
            }
        }
    }

    /// Solid block of `n` brick courses; returns the top height.
    pub fn block(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y: i32, n: i32, color: u8) -> i32 {
        for c in 0..n {
            self.fill(x0, z0, x1, z1, y + c * 3, Kind::Brick, color);
        }
        y + n * 3
    }

    /// Hollow rectangular walls `t` studs thick, `n` brick courses; returns the top height.
    #[allow(clippy::too_many_arguments)]
    pub fn walls(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y: i32, n: i32, t: i32, color: u8) -> i32 {
        for c in 0..n {
            for x in x0..x1 {
                for z in z0..z1 {
                    if x < x0 + t || x >= x1 - t || z < z0 + t || z >= z1 - t {
                        self.set(x, y + c * 3, z, Kind::Brick, color);
                    }
                }
            }
        }
        y + n * 3
    }

    /// Recolour existing voxels in a box (used for windows, doors, stripes).
    #[allow(clippy::too_many_arguments)]
    pub fn paint(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y0: i32, y1: i32, color: u8) {
        for x in x0..x1 {
            for z in z0..z1 {
                for y in y0..y1 {
                    if let Some(c) = self.vox.get_mut(&(x, y, z)) {
                        c.color = color;
                    }
                }
            }
        }
    }

    /// Remove voxels in a box (openings).
    #[allow(clippy::too_many_arguments)]
    pub fn carve(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y0: i32, y1: i32) {
        for x in x0..x1 {
            for z in z0..z1 {
                for y in y0..y1 {
                    self.vox.remove(&(x, y, z));
                }
            }
        }
    }

    /// Turn voxels into bricks and run the connection check.
    pub fn finish(mut self) -> (Model, usize) {
        let explicit = std::mem::take(&mut self.parts);
        let occ = crate::model::Occupancy::build(&explicit);
        let mut bricks = Vec::new();
        // Brickify, then repair voxel bricks the connection check flags by
        // merging them into neighbours. A few rounds settle it.
        for round in 0..8 {
            let mut bottoms: std::collections::HashSet<(i32, i32, i32)> = std::collections::HashSet::new();
            for b in &explicit {
                for (x, z) in b.cells() {
                    bottoms.insert((x, b.y, z));
                }
            }
            bottoms.extend(self.vox.keys().copied());
            bricks = explicit.clone();
            bricks.extend(brickify(&self.vox, self.period, &bottoms));
            let a = analyze(&bricks);
            if a.flagged() == 0 || round == 7 {
                break;
            }
            let mut changed = false;
            for (i, b) in bricks.iter().enumerate().skip(explicit.len()) {
                if a.status[i] == Status::Ok {
                    continue;
                }
                for (x, z) in b.cells() {
                    changed |= self.repair_cell(x, b.y, z, &occ);
                }
            }
            if !changed {
                break;
            }
        }
        // Safety net: anything the connection check flags is dropped.
        let mut pruned = 0;
        loop {
            let a = analyze(&bricks);
            if a.flagged() == 0 {
                break;
            }
            if std::env::var_os("BRICK_DEBUG").is_some() {
                for (b, s) in bricks.iter().zip(a.status.iter()) {
                    if *s != Status::Ok {
                        eprintln!("  {:?} {:?} at x{} y{} z{} rot{} group{}", s, b.part, b.x, b.y, b.z, b.rot, b.group);
                    }
                }
            }
            let before = bricks.len();
            let keep: Vec<Brick> = bricks
                .iter()
                .zip(a.status.iter())
                .filter(|(_, s)| **s == Status::Ok)
                .map(|(b, _)| *b)
                .collect();
            bricks = keep;
            pruned += before - bricks.len();
        }
        if pruned > 0 && std::env::var_os("BRICK_DEBUG").is_some() {
            eprintln!("[{}] pruned {} flagged bricks", self.name, pruned);
        }
        let mut m = Model::new(&self.name);
        for b in bricks {
            m.add(b);
        }
        (m, pruned)
    }

    fn vox_free(&self, x: i32, y: i32, z: i32, h: i32, occ: &crate::model::Occupancy) -> bool {
        for yy in y - 2..y + h {
            if let Some(c) = self.vox.get(&(x, yy, z)) {
                if yy + c.kind.height() > y {
                    return false;
                }
            }
        }
        (y..y + h).all(|yy| !occ.cells.contains_key(&(x, z, yy)))
    }

    /// Try to tie a flagged voxel cell into its layer: adopt a neighbour's
    /// colour, or bridge a diagonal-only gap with an extra cell.
    fn repair_cell(&mut self, x: i32, y: i32, z: i32, occ: &crate::model::Occupancy) -> bool {
        let Some(c) = self.vox.get(&(x, y, z)).copied() else { return false };
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(n) = self.vox.get(&(x + dx, y, z + dz)).copied() {
                if n.kind == c.kind && n != c {
                    self.vox.insert((x, y, z), n);
                    return true;
                }
            }
        }
        for (dx, dz) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            if let Some(n) = self.vox.get(&(x + dx, y, z + dz)).copied() {
                if n.kind != c.kind {
                    continue;
                }
                for (bx, bz) in [(x + dx, z), (x, z + dz)] {
                    if !self.vox.contains_key(&(bx, y, bz)) && self.vox_free(bx, y, bz, c.kind.height(), occ) {
                        self.vox.insert((bx, y, bz), c);
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn build(self) -> Model {
        self.finish().0
    }
}

const ONE_WIDE: [i32; 5] = [1, 2, 3, 4, 6];

/// Split a run into pieces cut at a global running-bond grid.
fn split_run(start: i32, len: i32, period: i32, offset: i32) -> Vec<(i32, i32)> {
    let mut cuts = vec![start];
    for p in start + 1..start + len {
        if (p + offset).rem_euclid(period) == 0 {
            cuts.push(p);
        }
    }
    cuts.push(start + len);
    let mut pieces: Vec<i32> = cuts.windows(2).map(|w| w[1] - w[0]).collect();
    // No single-stud pieces when they can be merged into a neighbour.
    while pieces.len() > 1 {
        let Some(i) = pieces.iter().position(|&l| l == 1) else { break };
        let j = if i + 1 < pieces.len() { i + 1 } else { i - 1 };
        let merged = pieces[i] + pieces[j];
        let lo = i.min(j);
        pieces.remove(lo + 1);
        pieces[lo] = merged;
        if !ONE_WIDE.contains(&merged) {
            // 5 -> 3 + 2, 7 -> 4 + 3
            let a = if merged == 5 { 3 } else { 4 };
            pieces[lo] = a;
            pieces.insert(lo + 1, merged - a);
        }
    }
    // Lengths without a matching part (5) are split.
    let pieces: Vec<i32> = pieces.into_iter().flat_map(|l| if l == 5 { vec![3, 2] } else { vec![l] }).collect();
    let mut out = Vec::new();
    let mut p = start;
    for l in pieces {
        out.push((p, l));
        p += l;
    }
    out
}

/// `bottoms` holds (x, y, z) for every cell where some part starts at height y,
/// used to tell whether a voxel has anything sitting on it.
fn brickify(vox: &HashMap<(i32, i32, i32), Cell>, period: i32, bottoms: &std::collections::HashSet<(i32, i32, i32)>) -> Vec<Brick> {
    // Group by layer.
    let mut layers: HashMap<(i32, Kind), HashMap<(i32, i32), Cell>> = HashMap::new();
    for (&(x, y, z), &c) in vox {
        layers.entry((y, c.kind)).or_default().insert((x, z), c);
    }
    // A lone single cell with nothing on top would become a weak 1x1:
    // give it the colour of a neighbour so it merges into a bigger brick.
    for (&(y, kind), mask) in layers.iter_mut() {
        let mut lone: Vec<(i32, i32)> = mask
            .iter()
            .filter(|(&(x, z), c)| {
                [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|(dx, dz)| mask.get(&(x + dx, z + dz)) != Some(c))
                    && !bottoms.contains(&(x, y + kind.height(), z))
            })
            .map(|(k, _)| *k)
            .collect();
        lone.sort();
        for (x, z) in lone {
            let c = mask[&(x, z)];
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if let Some(n) = mask.get(&(x + dx, z + dz)) {
                    if n.kind == c.kind {
                        let n = *n;
                        mask.insert((x, z), n);
                        break;
                    }
                }
            }
        }
    }
    let mut keys: Vec<_> = layers.keys().copied().collect();
    keys.sort_by_key(|k| (k.0, k.1 == Kind::Plate));
    let mut out = Vec::new();
    for key in keys {
        let mask = &layers[&key];
        let (y, kind) = key;
        let parity = if kind == Kind::Brick { y.div_euclid(3) } else { y }.rem_euclid(2);
        let offset = parity * period / 2;
        let run = |x: i32, z: i32, dx: i32, dz: i32| -> i32 {
            let c = mask[&(x, z)];
            let mut n = 1;
            let mut k = 1;
            while mask.get(&(x + dx * k, z + dz * k)) == Some(&c) {
                n += 1;
                k += 1;
            }
            k = 1;
            while mask.get(&(x - dx * k, z - dz * k)) == Some(&c) {
                n += 1;
                k += 1;
            }
            n
        };
        // Direction per cell: true = along X.
        let mut dir: HashMap<(i32, i32), bool> = HashMap::new();
        for &(x, z) in mask.keys() {
            let rx = run(x, z, 1, 0);
            let rz = run(x, z, 0, 1);
            let pref_x = parity == 0;
            let (rp, ro) = if pref_x { (rx, rz) } else { (rz, rx) };
            let use_pref = rp >= 3 || rp >= ro;
            dir.insert((x, z), if use_pref { pref_x } else { !pref_x });
        }
        // Fix cells that would end up as a run of one.
        let mut sorted: Vec<(i32, i32)> = mask.keys().copied().collect();
        sorted.sort();
        for _ in 0..4 {
            let mut changed = false;
            for &(x, z) in &sorted {
                let d = dir[&(x, z)];
                let c = mask[&(x, z)];
                let axis = |ax: bool| if ax { [(x + 1, z), (x - 1, z)] } else { [(x, z + 1), (x, z - 1)] };
                let same = |n: &(i32, i32), dd: bool| mask.get(n) == Some(&c) && dir.get(n) == Some(&dd);
                if axis(d).iter().any(|n| same(n, d)) {
                    continue;
                }
                if axis(!d).iter().any(|n| same(n, !d)) {
                    dir.insert((x, z), !d);
                    changed = true;
                    continue;
                }
                if let Some(n) = axis(d).iter().find(|n| mask.get(n) == Some(&c)) {
                    dir.insert(*n, d);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // Collect pieces: (along_x, row, start, len, cell)
        let mut pieces: Vec<(bool, i32, i32, i32, Cell)> = Vec::new();
        for along_x in [true, false] {
            let mut cells: Vec<(i32, i32)> =
                dir.iter().filter(|(_, &d)| d == along_x).map(|(&(x, z), _)| if along_x { (z, x) } else { (x, z) }).collect();
            cells.sort();
            let mut i = 0;
            while i < cells.len() {
                let (row, s) = cells[i];
                let get = |row: i32, a: i32| if along_x { mask[&(a, row)] } else { mask[&(row, a)] };
                let c = get(row, s);
                let mut j = i + 1;
                while j < cells.len() && cells[j].0 == row && cells[j].1 == s + (j - i) as i32 && get(row, cells[j].1) == c {
                    j += 1;
                }
                let len = (j - i) as i32;
                for (ps, pl) in split_run(s, len, period, offset) {
                    pieces.push((along_x, row, ps, pl, c));
                }
                i = j;
            }
        }
        // Pair neighbouring identical pieces into two-wide bricks.
        pieces.sort_by_key(|p| (!p.0, p.1, p.2));
        let index: HashMap<(bool, i32, i32, i32, Cell), usize> =
            pieces.iter().enumerate().map(|(i, p)| (*p, i)).collect();
        let mut used = vec![false; pieces.len()];
        for i in 0..pieces.len() {
            if used[i] {
                continue;
            }
            used[i] = true;
            let (ax, row, s, l, c) = pieces[i];
            let mut wide = 1;
            if let Some(&j) = index.get(&(ax, row + 1, s, l, c)) {
                if !used[j] {
                    used[j] = true;
                    wide = 2;
                }
            }
            let (part, rot) = match (wide, l) {
                (1, l) => (Part::Brick { w: 1, l: l as u8 }, if ax { 1 } else { 0 }),
                (_, 1) => (Part::Brick { w: 1, l: 2 }, if ax { 0 } else { 1 }),
                (_, l) => (Part::Brick { w: 2, l: l as u8 }, if ax { 1 } else { 0 }),
            };
            let part = match (kind, part) {
                (Kind::Plate, Part::Brick { w, l }) => Part::Plate { w, l },
                (_, p) => p,
            };
            let (x, z) = if ax { (s, row) } else { (row, s) };
            let mut b = Brick::new(part, x, y, z, rot, c.color);
            b.group = c.group;
            out.push(b);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_runs_avoid_single_studs() {
        for len in 2..30 {
            for (period, off) in [(4, 0), (4, 2), (6, 0), (6, 3), (6, 1)] {
                let p = split_run(0, len, period, off);
                assert_eq!(p.iter().map(|x| x.1).sum::<i32>(), len);
                assert!(p.iter().all(|x| ONE_WIDE.contains(&x.1) && x.1 > 1), "{len} {period} {off} {p:?}");
            }
        }
        assert_eq!(split_run(5, 1, 4, 0), vec![(5, 1)]);
    }

    #[test]
    fn brickify_covers_cells() {
        let mut b = Builder::new("t", 1, 1, 0);
        b.walls(2, 2, 12, 10, 1, 4, 1, 1);
        b.block(14, 14, 20, 20, 1, 2, 2);
        let cells = b.vox.len();
        let bricks = brickify(&b.vox, 4, &Default::default());
        let covered: usize = bricks.iter().map(|b| b.cells().count()).sum();
        assert_eq!(covered, cells);
    }
}
