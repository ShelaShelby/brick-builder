//! Large: a castle gatehouse with two towers, curtain walls and a moat.

use super::kit::{self, hash, House};
use super::{Builder, Kind};
use crate::model::Model;
use crate::parts::col::*;
use crate::parts::Part;

fn stone(x: i32, y: i32, z: i32) -> u8 {
    if hash(x.div_euclid(3), y, z.div_euclid(3)) % 5 == 0 {
        DGRAY
    } else {
        LGRAY
    }
}

/// Square tower with 2-wide walls, arrow slits, a floor and battlements.
fn tower(b: &mut Builder, x0: i32, z0: i32, size: i32, courses: i32) {
    let (x1, z1) = (x0 + size, z0 + size);
    for c in 0..courses {
        let y = 1 + c * 3;
        for x in x0..x1 {
            for z in z0..z1 {
                if x < x0 + 2 || x >= x1 - 2 || z < z0 + 2 || z >= z1 - 2 {
                    b.brick(x, y, z, stone(x, y, z));
                }
            }
        }
    }
    // Arrow slits on all faces.
    let mid = size / 2;
    for c in [3, 4, 7, 8] {
        let y = 1 + c * 3;
        b.paint(x0 + mid, z1 - 1, x0 + mid + 1, z1, y, y + 1, BLACK);
        b.paint(x0 + mid - 1, z0, x0 + mid, z0 + 1, y, y + 1, BLACK);
        b.paint(x0, z0 + mid, x0 + 1, z0 + mid + 1, y, y + 1, BLACK);
        b.paint(x1 - 1, z0 + mid - 1, x1, z0 + mid, y, y + 1, BLACK);
    }
    // Upper ledge and battlements.
    let yt = 1 + courses * 3;
    b.fill(x0, z0, x1, z1, yt, Kind::Plate, DGRAY);
    kit::battlements(b, x0, z0, x1, z1, yt + 1, LGRAY);
    // Flag pole with a banner.
    let (fx, fz) = (x0 + mid - 1, z0 + mid - 1);
    for k in 0..3 {
        b.part(Part::Round { d: 2 }, fx, yt + 1 + k * 3, fz, 0, BLACK);
    }
    b.part(Part::Brick { w: 2, l: 4 }, fx, yt + 10, fz, 1, RED);
    b.part(Part::Brick { w: 2, l: 4 }, fx, yt + 13, fz, 1, YELLOW);
    b.part(Part::Tile { d: 2 }, fx, yt + 16, fz, 0, RED);
}

pub fn build() -> Model {
    let mut b = Builder::new("Castle Gate", 2, 1, GREEN);
    b.period = 6;

    // Moat and road.
    b.group = 1;
    b.fill(0, 22, 64, 26, 1, Kind::Plate, BLUE);
    b.fill(28, 26, 36, 32, 1, Kind::Plate, TAN);

    // Gatehouse: 16 x 6, two studs thick, gate arch in the middle.
    b.group = 2;
    let (gx0, gz0, gx1, gz1) = (24, 11, 40, 17);
    let gate_courses = 9;
    for c in 0..gate_courses {
        let y = 1 + c * 3;
        for x in gx0..gx1 {
            for z in gz0..gz1 {
                if x < gx0 + 2 || x >= gx1 - 2 || z < gz0 + 2 || z >= gz1 - 2 {
                    b.brick(x, y, z, stone(x, y, z));
                }
            }
        }
    }
    // Gate opening with a narrowing arch.
    b.carve(29, gz0, 35, gz1, 1, 13);
    b.carve(30, gz0, 34, gz1, 13, 16);
    // Portcullis half raised, hanging from the arch.
    b.part(Part::Brick { w: 1, l: 4 }, 30, 13, 15, 1, DGRAY);
    b.part(Part::Brick { w: 1, l: 4 }, 30, 10, 15, 1, BLACK);
    // Banners and windows above the gate.
    for (x, cc) in [(26, RED), (37, RED)] {
        b.paint(x, gz1 - 1, x + 1, gz1, 7, 19, cc);
    }
    for x in [30, 32] {
        b.paint(x, gz1 - 1, x + 2, gz1, 19, 22, BLACK);
    }
    let yg = 1 + gate_courses * 3;
    // Solid top courses: the first one crosses the depth so it rests on both walls.
    b.fill(gx0 + 2, gz0 + 2, gx1 - 2, gz1 - 2, yg - 6, Kind::Brick, LGRAY);
    b.fill(gx0 + 2, gz0 + 2, gx1 - 2, gz1 - 2, yg - 3, Kind::Brick, LGRAY);
    b.fill(gx0, gz0, gx1, gz1, yg, Kind::Plate, DGRAY);
    kit::battlements(&mut b, gx0, gz0, gx1, gz1, yg + 1, LGRAY);

    // Towers either side.
    b.group = 3;
    tower(&mut b, 14, 9, 10, 12);
    b.group = 4;
    tower(&mut b, 40, 9, 10, 12);

    // Curtain walls with a walkway and battlements.
    b.group = 5;
    for (x0, x1) in [(2, 14), (50, 62)] {
        for c in 0..7 {
            let y = 1 + c * 3;
            for x in x0..x1 {
                for z in 13..15 {
                    b.brick(x, y, z, stone(x, y, z));
                }
            }
        }
        b.fill(x0, 13, x1, 15, 22, Kind::Plate, DGRAY);
        for x in x0..x1 {
            if (x - x0).rem_euclid(4) < 2 {
                b.brick(x, 23, 14, LGRAY);
            }
        }
    }
    // Side walls running back to enclose the courtyard.
    for x0 in [2, 60] {
        for c in 0..5 {
            let y = 1 + c * 3;
            for x in x0..x0 + 2 {
                for z in 1..13 {
                    b.brick(x, y, z, stone(x, y, z));
                }
            }
        }
        b.fill(x0, 1, x0 + 2, 13, 16, Kind::Plate, DGRAY);
    }

    // Drawbridge across the moat.
    b.group = 6;
    b.fill(29, 17, 35, 22, 1, Kind::Plate, DGRAY);
    b.fill(29, 17, 35, 28, 2, Kind::Plate, BROWN);
    for z in [18, 20, 22, 24, 26] {
        b.fill(29, z, 35, z + 1, 3, Kind::Plate, DRED);
    }

    // Courtyard: barracks, stable and a well.
    b.group = 7;
    kit::house(
        &mut b,
        &House { x: 6, z: 2, w: 10, d: 6, y: 1, wall: TAN, roof: DRED, trim: BROWN, door_side: 0, ..Default::default() },
    );
    b.group = 8;
    kit::house(
        &mut b,
        &House { x: 46, z: 1, w: 10, d: 6, y: 1, wall: WHITE, roof: BLUE, trim: BROWN, door_side: 0, ..Default::default() },
    );
    b.group = 9;
    kit::ring(&mut b, 32.0, 5.0, 1.0, 3.0, 1, Kind::Brick, |_, _| DGRAY);
    kit::ring(&mut b, 32.0, 5.0, 1.0, 3.0, 4, Kind::Plate, |_, _| LGRAY);
    b.part(Part::Plate { w: 2, l: 2 }, 31, 1, 4, 0, BLUE);

    // Outside: trees, torches and a cart path.
    b.group = 10;
    kit::tree(&mut b, 6, 27, 1, false);
    kit::tree(&mut b, 54, 28, 1, true);
    kit::tree(&mut b, 16, 28, 1, false);
    kit::lamp(&mut b, 25, 19, 1);
    kit::lamp(&mut b, 37, 19, 1);
    kit::fence(&mut b, 40, 29, 1, 2, true, BROWN);
    b.build()
}
