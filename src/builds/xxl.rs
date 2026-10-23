//! XXL: a town square with a clock tower, fountain, market, canal and bridges.

use super::kit::{self, House};
use super::{Builder, Kind};
use crate::model::Model;
use crate::parts::col::*;
use crate::parts::Part;

const WALLS: [u8; 6] = [WHITE, TAN, YELLOW, SKY, ORANGE, LGRAY];
const ROOFS: [u8; 6] = [RED, BLUE, DGRAY, DRED, GREEN, BROWN];

/// Clock tower with clock faces on all sides, belfry and roof.
fn clock_tower(b: &mut Builder, x0: i32, z0: i32, s: i32, courses: i32) {
    let (x1, z1) = (x0 + s, z0 + s);
    for c in 0..courses {
        let y = 1 + c * 3;
        let col = if c % 4 == 3 { LGRAY } else { TAN };
        for x in x0..x1 {
            for z in z0..z1 {
                if x < x0 + 2 || x >= x1 - 2 || z < z0 + 2 || z >= z1 - 2 {
                    b.brick(x, y, z, col);
                }
            }
        }
    }
    let m = s / 2;
    // Door and small windows.
    b.paint(x0 + m - 1, z1 - 1, x0 + m + 1, z1, 1, 7, BROWN);
    for c in [4, 8] {
        let y = 1 + c * 3;
        b.paint(x0 + m - 1, z1 - 1, x0 + m + 1, z1, y, y + 3, SKY);
        b.paint(x0 + m - 1, z0, x0 + m + 1, z0 + 1, y, y + 3, SKY);
        b.paint(x0, z0 + m - 1, x0 + 1, z0 + m + 1, y, y + 3, SKY);
        b.paint(x1 - 1, z0 + m - 1, x1, z0 + m + 1, y, y + 3, SKY);
    }
    // Clock faces: white square with black hands and a yellow frame.
    let yc = 1 + (courses - 4) * 3;
    let faces: [(i32, i32, i32, i32); 4] = [
        (x0 + m - 3, z1 - 1, x0 + m + 3, z1),
        (x0 + m - 3, z0, x0 + m + 3, z0 + 1),
        (x0, z0 + m - 3, x0 + 1, z0 + m + 3),
        (x1 - 1, z0 + m - 3, x1, z0 + m + 3),
    ];
    for (ax, az, bx, bz) in faces {
        b.paint(ax, az, bx, bz, yc - 3, yc + 6, YELLOW);
        let (ix0, iz0, ix1, iz1) = if az + 1 == bz { (ax + 1, az, bx - 1, bz) } else { (ax, az + 1, bx, bz - 1) };
        b.paint(ix0, iz0, ix1, iz1, yc, yc + 6, WHITE);
        // Hands: a vertical one in the middle and a short one to the side.
        let along_x = az + 1 == bz;
        let at = |k: i32| if along_x { (ax + k, az) } else { (ax, az + k) };
        let (hx, hz) = at(3);
        b.paint(hx, hz, hx + 1, hz + 1, yc, yc + 6, BLACK);
        let (mx, mz) = at(4);
        b.paint(mx, mz, mx + 1, mz + 1, yc, yc + 3, BLACK);
    }
    // Belfry openings in the top course.
    let yb = 1 + (courses - 1) * 3;
    b.carve(x0 + m - 1, z0, x0 + m + 1, z1, yb, yb + 3);
    b.carve(x0, z0 + m - 1, x1, z0 + m + 1, yb, yb + 3);
    let yt = 1 + courses * 3;
    b.fill(x0, z0, x1, z1, yt, Kind::Plate, DGRAY);
    let top = kit::roof(b, x0, z0, s, s, yt + 1, DGRAY, TAN, true);
    b.part(Part::Round { d: 2 }, x0 + m - 1, top, z0 + m - 1, 0, YELLOW);
    b.part(Part::Round { d: 1 }, x0 + m - 1, top + 3, z0 + m - 1, 0, YELLOW);
    b.part(Part::Round { d: 1 }, x0 + m, top + 3, z0 + m, 0, YELLOW);
    b.part(Part::Tile { d: 2 }, x0 + m - 1, top + 6, z0 + m - 1, 0, YELLOW);
}

fn fountain(b: &mut Builder, cx: f32, cz: f32) {
    kit::ring(b, cx, cz, 0.0, 5.0, 1, Kind::Plate, |_, _| BLUE);
    kit::ring(b, cx, cz, 4.0, 6.0, 1, Kind::Brick, |_, _| LGRAY);
    kit::ring(b, cx, cz, 4.0, 6.0, 4, Kind::Plate, |_, _| WHITE);
    let (x, z) = (cx as i32 - 1, cz as i32 - 1);
    b.part(Part::Round { d: 2 }, x, 2, z, 0, WHITE);
    b.part(Part::Round { d: 2 }, x, 5, z, 0, WHITE);
    b.part(Part::Plate { w: 2, l: 2 }, x, 8, z, 0, SKY);
    b.part(Part::Round { d: 2 }, x, 9, z, 0, SKY);
    b.part(Part::Round { d: 2 }, x, 12, z, 0, WHITE);
    b.part(Part::Tile { d: 2 }, x, 15, z, 0, SKY);
}

/// Market stall facing +Z: counter and a striped roof on four posts.
fn stall(b: &mut Builder, x: i32, z: i32, c: u8) {
    for (dx, dz) in [(0, 0), (4, 0), (0, 4), (4, 4)] {
        for k in 0..3 {
            b.part(Part::Round { d: 2 }, x + dx, 1 + k * 3, z + dz, 0, WHITE);
        }
    }
    b.part(Part::Brick { w: 2, l: 2 }, x + 2, 1, z + 4, 0, BROWN);
    b.part(Part::Plate { w: 2, l: 2 }, x + 2, 4, z + 4, 0, if c == RED { YELLOW } else { ORANGE });
    for k in 0..3 {
        let cc = if k % 2 == 0 { c } else { WHITE };
        b.part(Part::Plate { w: 2, l: 6 }, x + k * 2, 10, z, 0, cc);
    }
    b.part(Part::Plate { w: 2, l: 6 }, x, 11, z, 1, c);
    b.part(Part::Plate { w: 2, l: 6 }, x, 11, z + 4, 1, c);
}

/// Bridge across the canal along Z (6 wide, from z0 to z0 + 12).
fn bridge(b: &mut Builder, x: i32, z0: i32) {
    for dx in (0..6).step_by(2) {
        b.part(Part::Plate { w: 2, l: 6 }, x + dx, 4, z0, 0, DGRAY);
        b.part(Part::Plate { w: 2, l: 6 }, x + dx, 4, z0 + 6, 0, DGRAY);
        b.part(Part::Plate { w: 2, l: 3 }, x + dx, 5, z0, 0, TAN);
        b.part(Part::Plate { w: 2, l: 6 }, x + dx, 5, z0 + 3, 0, TAN);
        b.part(Part::Plate { w: 2, l: 3 }, x + dx, 5, z0 + 9, 0, TAN);
    }
    for z in z0..z0 + 12 {
        b.brick(x, 6, z, WHITE);
        b.brick(x + 5, 6, z, WHITE);
    }
    for z in z0..z0 + 12 {
        b.set(x, 9, z, Kind::Plate, RED);
        b.set(x + 5, 9, z, Kind::Plate, RED);
    }
}

pub fn build() -> Model {
    let mut b = Builder::new("Town Square", 3, 3, GREEN);
    b.period = 6;

    // Back street and the row of houses behind it.
    b.group = 1;
    b.fill(0, 22, 96, 30, 1, Kind::Plate, DGRAY);
    for x in (2..96).step_by(8) {
        b.paint(x, 25, x + 4, 27, 1, 2, WHITE);
    }
    b.fill(0, 30, 96, 36, 1, Kind::Plate, LGRAY);
    let mut g = 2;
    for (i, x) in [2, 18, 34, 50, 66, 82].iter().enumerate() {
        b.group = g;
        g += 1;
        let floors = if i == 1 || i == 4 { 3 } else { 2 };
        kit::house(
            &mut b,
            &House {
                x: *x,
                z: 6,
                w: 12,
                d: 12,
                y: 1,
                floors,
                wall: WALLS[i % 6],
                roof: ROOFS[i % 6],
                trim: if i % 2 == 0 { WHITE } else { DGRAY },
                door_side: 0,
                ..Default::default()
            },
        );
    }
    b.group = g;
    g += 1;
    kit::car(&mut b, 12, 23, 2, RED);
    kit::car(&mut b, 52, 23, 2, BLUE);
    kit::car(&mut b, 76, 26, 2, LIME);
    for x in [8, 40, 72] {
        kit::lamp(&mut b, x, 31, 2);
    }

    // Canal with quay walls, water, boats and two bridges.
    b.group = g;
    g += 1;
    for (z0, z1) in [(36, 38), (46, 48)] {
        for x in 0..96 {
            for z in z0..z1 {
                b.brick(x, 1, z, DGRAY);
            }
        }
    }
    b.fill(0, 38, 96, 46, 1, Kind::Plate, BLUE);
    b.group = g;
    g += 1;
    kit::boat(&mut b, 6, 39, 2, RED);
    kit::boat(&mut b, 40, 43, 2, YELLOW);
    kit::boat(&mut b, 82, 40, 2, WHITE);
    for x in [22, 62] {
        b.carve(x, 36, x + 6, 48, 4, 7);
        bridge(&mut b, x, 36);
    }

    // Plaza paving.
    b.group = g;
    g += 1;
    b.fill(0, 48, 96, 52, 1, Kind::Plate, LGRAY);
    let fountain_c = (48.0, 74.0);
    let tower = (42, 52, 12);
    let mut x = 18;
    while x < 78 {
        let mut z = 52;
        while z < 96 {
            let in_fountain = kit::in_ring(x, z, fountain_c.0, fountain_c.1, 0.0, 7.0)
                || kit::in_ring(x + 1, z + 1, fountain_c.0, fountain_c.1, 0.0, 7.0)
                || kit::in_ring(x + 1, z, fountain_c.0, fountain_c.1, 0.0, 7.0)
                || kit::in_ring(x, z + 1, fountain_c.0, fountain_c.1, 0.0, 7.0);
            let in_tower = x >= tower.0 && x < tower.0 + tower.2 && z >= tower.1 && z < tower.1 + tower.2;
            let stall_spot = (z >= 80 && z < 88) && ((x >= 22 && x < 30) || (x >= 66 && x < 74));
            let tree_spot = z >= 90 && (x - 18) % 12 == 4;
            let bench_spot = z == 66 && ((34..38).contains(&x) || (58..62).contains(&x));
            if bench_spot {
                z += 2;
                continue;
            }
            if !in_fountain && !in_tower && !stall_spot && !tree_spot {
                let checker = ((x / 2) + (z / 2)) % 2 == 0;
                b.part(Part::Tile { d: 2 }, x, 1, z, 0, if checker { TAN } else { WHITE });
            }
            z += 2;
        }
        x += 2;
    }

    // Clock tower at the head of the square.
    b.group = g;
    g += 1;
    clock_tower(&mut b, tower.0, tower.1, tower.2, 18);

    b.group = g;
    g += 1;
    fountain(&mut b, fountain_c.0, fountain_c.1);

    b.group = g;
    g += 1;
    stall(&mut b, 23, 81, RED);
    stall(&mut b, 67, 81, BLUE);
    for x in (22..78).step_by(12) {
        kit::tree(&mut b, x, 91, 1, false);
    }

    // Houses along both sides of the square.
    let side = [(2, 54), (2, 68), (2, 82), (82, 54), (82, 68), (82, 82)];
    for (i, (x, z)) in side.iter().enumerate() {
        b.group = g;
        g += 1;
        kit::house(
            &mut b,
            &House {
                x: *x,
                z: *z,
                w: 12,
                d: 12,
                y: 1,
                floors: 2,
                wall: WALLS[(i + 2) % 6],
                roof: ROOFS[(i + 3) % 6],
                trim: WHITE,
                door_side: if *x < 48 { 2 } else { 3 },
                ridge_x: false,
                ..Default::default()
            },
        );
    }
    b.group = g;
    for z in [56, 70, 84] {
        kit::lamp(&mut b, 15, z, 1);
        kit::lamp(&mut b, 79, z, 1);
    }

    kit::bench(&mut b, 34, 66, 1);
    kit::bench(&mut b, 58, 66, 1);
    b.build()
}
