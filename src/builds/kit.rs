//! Reusable pieces for the ready-made builds.

use super::{Builder, Kind};
use crate::parts::col::*;
use crate::parts::Part;

pub struct House {
    pub x: i32,
    pub z: i32,
    /// Size along X.
    pub w: i32,
    /// Size along Z (even).
    pub d: i32,
    pub y: i32,
    pub floors: i32,
    pub wall: u8,
    pub roof: u8,
    pub trim: u8,
    pub door: u8,
    /// Ridge runs along X (otherwise along Z).
    pub ridge_x: bool,
    /// Which side has the door: 0 = +Z (front), 1 = -Z, 2 = +X, 3 = -X.
    pub door_side: u8,
    /// Plate band on top of each floor.
    pub band: bool,
}

impl Default for House {
    fn default() -> Self {
        House {
            x: 0,
            z: 0,
            w: 8,
            d: 6,
            y: 1,
            floors: 1,
            wall: WHITE,
            roof: RED,
            trim: DGRAY,
            door: BROWN,
            ridge_x: true,
            door_side: 0,
            band: true,
        }
    }
}

/// Is position `i` along a wall of length `len` part of a window column?
fn window_at(i: i32, len: i32) -> bool {
    if i <= 0 || i >= len - 1 {
        return false;
    }
    // Centre a pattern of 2-wide windows with 2-wide piers.
    let inner = len - 2;
    let n = (inner + 2) / 4; // number of windows
    if n == 0 {
        return false;
    }
    let used = n * 4 - 2;
    let start = 1 + (inner - used) / 2;
    let k = i - start;
    k >= 0 && k < used && k % 4 < 2
}

/// Walls with windows and a door, a trim band per floor and a gable roof.
/// Returns the height of the roof ridge top.
pub fn house(b: &mut Builder, h: &House) -> i32 {
    let (x0, z0, x1, z1) = (h.x, h.z, h.x + h.w, h.z + h.d);
    let mut y = h.y;
    for f in 0..h.floors {
        b.walls(x0, z0, x1, z1, y, 3, 1, h.wall);
        // Windows on all four walls, courses 1 and 2.
        for c in 1..3 {
            let yc = y + c * 3;
            for i in 0..h.w {
                if window_at(i, h.w) {
                    b.paint(x0 + i, z0, x0 + i + 1, z0 + 1, yc, yc + 1, SKY);
                    b.paint(x0 + i, z1 - 1, x0 + i + 1, z1, yc, yc + 1, SKY);
                }
            }
            for i in 0..h.d {
                if window_at(i, h.d) {
                    b.paint(x0, z0 + i, x0 + 1, z0 + i + 1, yc, yc + 1, SKY);
                    b.paint(x1 - 1, z0 + i, x1, z0 + i + 1, yc, yc + 1, SKY);
                }
            }
        }
        if f == 0 {
            // Door: two studs wide, two courses tall, in the middle of the door side.
            let (dx0, dz0, dx1, dz1) = match h.door_side {
                0 => (x0 + h.w / 2 - 1, z1 - 1, x0 + h.w / 2 + 1, z1),
                1 => (x0 + h.w / 2 - 1, z0, x0 + h.w / 2 + 1, z0 + 1),
                2 => (x1 - 1, z0 + h.d / 2 - 1, x1, z0 + h.d / 2 + 1),
                _ => (x0, z0 + h.d / 2 - 1, x0 + 1, z0 + h.d / 2 + 1),
            };
            b.paint(dx0, dz0, dx1, dz1, y, y + 6, h.door);
        }
        y += 9;
        if !h.band {
            continue;
        }
        // Trim band of plates on top of each floor.
        for x in x0..x1 {
            for z in z0..z1 {
                if x == x0 || x == x1 - 1 || z == z0 || z == z1 - 1 {
                    b.set(x, y, z, Kind::Plate, h.trim);
                }
            }
        }
        y += 1;
    }
    if h.ridge_x {
        roof(b, x0, z0, h.w, h.d, y, h.roof, h.wall, true)
    } else {
        roof(b, x0, z0, h.d, h.w, y, h.roof, h.wall, false)
    }
}

/// Gable roof of slopes. `len` runs along the ridge, `depth` across (even).
/// With `ridge_x` the ridge runs along X starting at (x0, z0).
#[allow(clippy::too_many_arguments)]
pub fn roof(b: &mut Builder, x0: i32, z0: i32, len: i32, depth: i32, y: i32, color: u8, gable: u8, ridge_x: bool) -> i32 {
    let levels = depth / 2 - 1;
    for k in 0..levels {
        let yk = y + k * 3;
        let mut u = 0;
        while u < len {
            let w = if len - u >= 2 { 2 } else { 1 };
            let part = Part::Slope { w: w as u8 };
            if ridge_x {
                b.part(part, x0 + u, yk, z0 + k, 0, color);
                b.part(part, x0 + u, yk, z0 + depth - 2 - k, 2, color);
            } else {
                b.part(part, x0 + k, yk, z0 + u, 1, color);
                b.part(part, x0 + depth - 2 - k, yk, z0 + u, 3, color);
            }
            u += w;
        }
        let a0 = k + 2;
        let a1 = depth - 2 - k;
        for v in a0..a1 {
            for u in 0..len {
                let (x, z) = if ridge_x { (x0 + u, z0 + v) } else { (x0 + v, z0 + u) };
                let c = if u == 0 || u == len - 1 { gable } else { color };
                b.brick(x, yk, z, c);
            }
        }
    }
    // Ridge: two rows of plates on the top studs.
    let yr = y + levels * 3;
    for v in levels..levels + 2 {
        for u in 0..len {
            let (x, z) = if ridge_x { (x0 + u, z0 + v) } else { (x0 + v, z0 + u) };
            b.set(x, yr, z, Kind::Plate, color);
        }
    }
    yr + 1
}

/// Round-topped tree: trunk of 2x2 round bricks and a leafy crown.
pub fn tree(b: &mut Builder, x: i32, z: i32, y: i32, big: bool) {
    b.part(Part::Round { d: 2 }, x, y, z, 0, BROWN);
    b.part(Part::Round { d: 2 }, x, y + 3, z, 0, BROWN);
    let mut yy = y + 6;
    if big {
        b.part(Part::Round { d: 2 }, x, yy, z, 0, BROWN);
        yy += 3;
        b.fill(x - 1, z - 1, x + 3, z + 3, yy, Kind::Brick, GREEN);
        b.fill(x - 2, z - 1, x + 4, z + 3, yy + 3, Kind::Brick, GREEN);
        b.fill(x - 1, z - 2, x + 3, z + 4, yy + 6, Kind::Brick, GREEN);
        b.fill(x - 1, z - 1, x + 3, z + 3, yy + 9, Kind::Brick, LIME);
        b.part(Part::Round { d: 2 }, x, yy + 12, z, 0, GREEN);
    } else {
        b.fill(x - 1, z - 1, x + 3, z + 3, yy, Kind::Brick, GREEN);
        b.fill(x - 1, z - 1, x + 3, z + 3, yy + 3, Kind::Brick, GREEN);
        b.part(Part::Round { d: 2 }, x, yy + 6, z, 0, LIME);
    }
}

/// Small conifer made of stacked plates getting narrower.
pub fn pine(b: &mut Builder, x: i32, z: i32, y: i32) {
    b.part(Part::Round { d: 2 }, x, y, z, 0, BROWN);
    let mut yy = y + 3;
    for (r, n) in [(2, 2), (1, 2)] {
        for _ in 0..n {
            b.fill(x - r, z - r, x + 2 + r, z + 2 + r, yy, Kind::Plate, GREEN);
            yy += 1;
        }
        b.fill(x - r + 1, z - r + 1, x + 1 + r, z + 1 + r, yy, Kind::Brick, GREEN);
        yy += 3;
    }
    b.part(Part::Round { d: 2 }, x, yy, z, 0, GREEN);
}

/// Street lamp: black pole of 2x2 round bricks with a yellow light.
pub fn lamp(b: &mut Builder, x: i32, z: i32, y: i32) {
    for k in 0..3 {
        b.part(Part::Round { d: 2 }, x, y + k * 3, z, 0, BLACK);
    }
    b.part(Part::Round { d: 2 }, x, y + 9, z, 0, YELLOW);
    b.part(Part::Tile { d: 2 }, x, y + 12, z, 0, BLACK);
}

/// Picket fence along X (or Z): pairs of posts joined by a rail plate.
pub fn fence(b: &mut Builder, x: i32, z: i32, y: i32, segments: i32, along_x: bool, color: u8) {
    for s in 0..segments {
        let o = s * 4;
        let (ax, az, bx, bz) = if along_x { (x + o, z, x + o + 2, z) } else { (x, z + o, x, z + o + 2) };
        b.part(Part::Brick { w: 1, l: 1 }, ax, y, az, 0, color);
        b.part(Part::Brick { w: 1, l: 1 }, bx, y, bz, 0, color);
        if along_x {
            b.part(Part::Plate { w: 1, l: 4 }, x + o, y + 3, z, 1, color);
        } else {
            b.part(Part::Plate { w: 1, l: 4 }, x, y + 3, z + o, 0, color);
        }
    }
}

/// Bench: two legs and a seat plate (along X).
pub fn bench(b: &mut Builder, x: i32, z: i32, y: i32) {
    b.part(Part::Plate { w: 1, l: 2 }, x, y, z, 0, DGRAY);
    b.part(Part::Plate { w: 1, l: 2 }, x + 3, y, z, 0, DGRAY);
    b.part(Part::Brick { w: 1, l: 2 }, x, y + 1, z, 0, DGRAY);
    b.part(Part::Brick { w: 1, l: 2 }, x + 3, y + 1, z, 0, DGRAY);
    b.part(Part::Plate { w: 2, l: 4 }, x, y + 4, z, 1, BROWN);
}

/// Flower bed: a green plate with coloured round flowers.
pub fn flowers(b: &mut Builder, x: i32, z: i32, y: i32, c: u8) {
    b.part(Part::Plate { w: 2, l: 4 }, x, y, z, 1, GREEN);
    b.part(Part::Round { d: 2 }, x, y + 1, z, 0, c);
}

/// Crenellations on top of a wall ring: two-stud merlons with gaps.
pub fn battlements(b: &mut Builder, x0: i32, z0: i32, x1: i32, z1: i32, y: i32, color: u8) {
    let edge = |x: i32, z: i32| (x == x0 || x == x1 - 1 || z == z0 || z == z1 - 1) && x >= x0 && x < x1 && z >= z0 && z < z1;
    let corner = |x: i32, z: i32| (x == x0 || x == x1 - 1) && (z == z0 || z == z1 - 1);
    let mut cells = Vec::new();
    for x in x0..x1 {
        for z in z0..z1 {
            if !edge(x, z) {
                continue;
            }
            let along = if z == z0 || z == z1 - 1 { x - x0 } else { z - z0 };
            if corner(x, z) || along.rem_euclid(4) < 2 {
                cells.push((x, z));
            }
        }
    }
    let set: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let mut extra = Vec::new();
    for &(x, z) in &cells {
        let nbs = [(x + 1, z), (x - 1, z), (x, z + 1), (x, z - 1)];
        if !nbs.iter().any(|n| set.contains(n)) {
            if let Some(n) = nbs.iter().find(|n| edge(n.0, n.1)) {
                extra.push(*n);
            }
        }
    }
    for (x, z) in cells.into_iter().chain(extra) {
        b.brick(x, y, z, color);
    }
}

/// Small car along X, 4 wide (z) and 8 long, standing on its own plate.
pub fn car(b: &mut Builder, x: i32, z: i32, y: i32, body: u8) {
    // wheels
    for dx in [1, 5] {
        b.part(Part::Brick { w: 1, l: 2 }, x + dx, y, z, 1, BLACK);
        b.part(Part::Brick { w: 1, l: 2 }, x + dx, y, z + 3, 1, BLACK);
    }
    b.fill(x, z, x + 8, z + 4, y + 3, Kind::Plate, body);
    b.fill(x, z, x + 8, z + 4, y + 4, Kind::Brick, body);
    b.paint(x, z + 1, x + 1, z + 3, y + 4, y + 5, YELLOW);
    b.paint(x + 7, z + 1, x + 8, z + 3, y + 4, y + 5, RED);
    b.fill(x + 2, z, x + 6, z + 4, y + 7, Kind::Brick, SKY);
    b.part(Part::Slope { w: 2 }, x, y + 7, z, 1, SKY);
    b.part(Part::Slope { w: 2 }, x, y + 7, z + 2, 1, SKY);
    b.part(Part::Slope { w: 2 }, x + 6, y + 7, z, 3, body);
    b.part(Part::Slope { w: 2 }, x + 6, y + 7, z + 2, 3, body);
    b.fill(x + 2, z, x + 6, z + 4, y + 10, Kind::Plate, body);
}

/// Deterministic hash for colour variation.
pub fn hash(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}

/// Is cell (x, z) inside the ring rin <= r < rout around (cx, cz)?
pub fn in_ring(x: i32, z: i32, cx: f32, cz: f32, rin: f32, rout: f32) -> bool {
    let dx = x as f32 + 0.5 - cx;
    let dz = z as f32 + 0.5 - cz;
    let d2 = dx * dx + dz * dz;
    d2 >= rin * rin && d2 < rout * rout
}

/// Fill a ring (or disc when rin = 0) of voxels with a colour function.
#[allow(clippy::too_many_arguments)]
pub fn ring(b: &mut Builder, cx: f32, cz: f32, rin: f32, rout: f32, y: i32, kind: Kind, color: impl Fn(i32, i32) -> u8) {
    let r = rout.ceil() as i32 + 1;
    for x in (cx as i32 - r)..=(cx as i32 + r) {
        for z in (cz as i32 - r)..=(cz as i32 + r) {
            if in_ring(x, z, cx, cz, rin, rout) {
                b.set(x, y, z, kind, color(x, z));
            }
        }
    }
}

/// Small rowing boat along X on the water.
pub fn boat(b: &mut Builder, x: i32, z: i32, y: i32, hull: u8) {
    b.part(Part::Slope { w: 2 }, x, y, z, 1, hull);
    b.fill(x + 2, z, x + 6, z + 2, y, Kind::Brick, hull);
    b.part(Part::Slope { w: 2 }, x + 6, y, z, 3, hull);
    b.fill(x + 1, z, x + 7, z + 2, y + 3, Kind::Plate, BROWN);
}
