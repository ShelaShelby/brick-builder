//! XL: a railway station with a platform canopy, two tracks and trains.

use super::kit::{self, House};
use super::{Builder, Kind};
use crate::model::Model;
use crate::parts::col::*;
use crate::parts::Part;

/// Straight track along X: ballast, sleepers and two rails (z0..z0+6).
fn track(b: &mut Builder, x0: i32, x1: i32, z0: i32) {
    b.fill(x0, z0, x1, z0 + 6, 1, Kind::Plate, DGRAY);
    b.fill(x0, z0 + 1, x1, z0 + 2, 2, Kind::Plate, LGRAY);
    b.fill(x0, z0 + 4, x1, z0 + 5, 2, Kind::Plate, LGRAY);
    let mut x = x0;
    while x < x1 {
        b.part(Part::Plate { w: 1, l: 2 }, x, 2, z0 + 2, 0, BROWN);
        x += 4;
    }
}

/// Steam engine facing +X, 4 wide, standing on the rails at height y.
fn engine(b: &mut Builder, x: i32, z: i32, y: i32) {
    b.fill(x, z, x + 14, z + 4, y, Kind::Plate, BLACK);
    // Cab.
    b.fill(x, z, x + 5, z + 4, y + 1, Kind::Brick, RED);
    b.fill(x, z, x + 5, z + 4, y + 4, Kind::Brick, RED);
    b.fill(x, z, x + 5, z + 4, y + 7, Kind::Brick, RED);
    b.paint(x + 1, z, x + 4, z + 1, y + 4, y + 5, SKY);
    b.paint(x + 1, z + 3, x + 4, z + 4, y + 4, y + 5, SKY);
    b.fill(x, z, x + 5, z + 4, y + 10, Kind::Plate, BLACK);
    b.part(Part::Tile { d: 2 }, x + 1, y + 11, z + 1, 0, DGRAY);
    // Boiler.
    b.fill(x + 5, z, x + 12, z + 4, y + 1, Kind::Brick, BLACK);
    b.fill(x + 5, z, x + 12, z + 4, y + 4, Kind::Brick, DGRAY);
    b.paint(x + 11, z + 1, x + 12, z + 3, y + 1, y + 2, YELLOW);
    b.paint(x + 7, z, x + 8, z + 4, y + 4, y + 5, RED);
    b.part(Part::Round { d: 2 }, x + 9, y + 7, z + 1, 0, BLACK);
    b.part(Part::Round { d: 2 }, x + 9, y + 10, z + 1, 0, DGRAY);
    b.part(Part::Round { d: 2 }, x + 6, y + 7, z + 1, 0, YELLOW);
    // Buffer beam.
    b.part(Part::Slope { w: 2 }, x + 12, y + 1, z, 3, RED);
    b.part(Part::Slope { w: 2 }, x + 12, y + 1, z + 2, 3, RED);
}

/// Passenger carriage along X.
fn carriage(b: &mut Builder, x: i32, z: i32, y: i32, len: i32, body: u8) {
    b.fill(x, z, x + len, z + 4, y, Kind::Plate, BLACK);
    for c in 0..3 {
        b.fill(x, z, x + len, z + 4, y + 1 + c * 3, Kind::Brick, body);
    }
    for i in 1..len - 1 {
        if (i - 1) % 3 < 2 {
            b.paint(x + i, z, x + i + 1, z + 1, y + 4, y + 5, SKY);
            b.paint(x + i, z + 3, x + i + 1, z + 4, y + 4, y + 5, SKY);
        }
    }
    b.paint(x, z + 1, x + 1, z + 3, y + 1, y + 7, BROWN);
    b.fill(x, z, x + len, z + 4, y + 10, Kind::Plate, LGRAY);
    b.fill(x + 1, z + 1, x + len - 1, z + 3, y + 11, Kind::Plate, DGRAY);
}

/// Open freight wagon along X.
fn wagon(b: &mut Builder, x: i32, z: i32, y: i32, len: i32, body: u8, load: u8) {
    b.fill(x, z, x + len, z + 4, y, Kind::Plate, BLACK);
    b.walls(x, z, x + len, z + 4, y + 1, 2, 1, body);
    b.fill(x + 1, z + 1, x + len - 1, z + 3, y + 1, Kind::Brick, load);
    b.fill(x + 1, z + 1, x + len - 1, z + 3, y + 4, Kind::Plate, load);
}

pub fn build() -> Model {
    let mut b = Builder::new("Railway Station", 3, 2, GREEN);
    b.period = 6;

    // Street behind the station.
    b.group = 1;
    b.fill(0, 2, 96, 10, 1, Kind::Plate, DGRAY);
    for x in (2..96).step_by(8) {
        b.paint(x, 5, x + 4, 7, 1, 2, WHITE);
    }
    b.fill(0, 10, 96, 14, 1, Kind::Plate, LGRAY);
    kit::car(&mut b, 10, 3, 2, BLUE);
    kit::car(&mut b, 60, 3, 2, YELLOW);
    for x in [6, 30, 66, 90] {
        kit::lamp(&mut b, x, 11, 2);
    }

    // Station building: two wings and a taller hall with a clock.
    b.group = 2;
    kit::house(
        &mut b,
        &House { x: 20, z: 16, w: 20, d: 12, y: 1, floors: 1, wall: TAN, roof: DRED, trim: WHITE, door_side: 0, ..Default::default() },
    );
    b.group = 3;
    kit::house(
        &mut b,
        &House { x: 56, z: 16, w: 20, d: 12, y: 1, floors: 1, wall: TAN, roof: DRED, trim: WHITE, door_side: 0, ..Default::default() },
    );
    b.group = 4;
    kit::house(
        &mut b,
        &House { x: 40, z: 16, w: 16, d: 12, y: 1, floors: 2, wall: WHITE, roof: DGRAY, trim: DRED, door_side: 0, ridge_x: false, ..Default::default() },
    );
    // Big doors on both sides and a clock face.
    b.paint(46, 27, 50, 28, 1, 7, BROWN);
    b.paint(46, 16, 50, 17, 1, 7, BROWN);
    b.paint(46, 27, 50, 28, 14, 20, WHITE);
    b.paint(48, 27, 49, 28, 14, 20, BLACK);
    b.paint(49, 27, 50, 28, 14, 15, BLACK);
    b.paint(45, 27, 46, 28, 14, 20, YELLOW);
    b.paint(50, 27, 51, 28, 14, 20, YELLOW);

    // Platform with tiles, a yellow edge and a canopy on columns.
    b.group = 5;
    let (px0, px1, pz0, pz1) = (16, 80, 30, 38);
    b.fill(px0, pz0, px1, pz1, 1, Kind::Brick, LGRAY);
    let column = |x: i32, z: i32| (x - px0).rem_euclid(8) >= 4 && (x - px0).rem_euclid(8) < 6 && (z == pz0 + 2 || z == pz0 + 3);
    let mut x = px0;
    while x < px1 {
        let mut z = pz0;
        while z < pz1 {
            if !column(x, z) {
                let c = if z == pz1 - 2 { YELLOW } else { WHITE };
                b.part(Part::Tile { d: 2 }, x, 4, z, 0, c);
            }
            z += 2;
        }
        x += 2;
    }
    b.group = 6;
    let mut k = 0;
    while px0 + k * 8 < px1 {
        let cx = px0 + k * 8 + 4;
        for c in 0..4 {
            b.part(Part::Round { d: 2 }, cx, 4 + c * 3, pz0 + 2, 0, DRED);
        }
        k += 1;
    }
    let yc = 16;
    let mut x = px0;
    while x < px1 {
        b.part(Part::Plate { w: 2, l: 8 }, x, yc, pz0, 0, DRED);
        x += 2;
    }
    let mut x = px0;
    while x < px1 {
        for z in (pz0..pz1).step_by(2) {
            b.part(Part::Plate { w: 2, l: 8 }, x, yc + 1, z, 1, DGRAY);
        }
        x += 8;
    }

    // Two tracks with trains.
    b.group = 7;
    track(&mut b, 0, 96, 40);
    track(&mut b, 0, 96, 50);
    b.group = 8;
    engine(&mut b, 62, 41, 3);
    carriage(&mut b, 47, 41, 3, 14, GREEN);
    carriage(&mut b, 32, 41, 3, 14, GREEN);
    b.group = 9;
    wagon(&mut b, 10, 51, 3, 10, BROWN, DGRAY);
    wagon(&mut b, 22, 51, 3, 10, BLUE, TAN);
    wagon(&mut b, 34, 51, 3, 10, BROWN, ORANGE);

    // Greenery at the front.
    b.group = 10;
    for (i, x) in [4, 20, 36, 52, 68, 84].iter().enumerate() {
        if i % 2 == 0 {
            kit::tree(&mut b, *x, 59, 1, true);
        } else {
            kit::pine(&mut b, *x, 59, 1);
        }
    }
    for x in [8, 40, 72] {
        kit::fence(&mut b, x, 56, 1, 4, true, WHITE);
    }
    kit::flowers(&mut b, 2, 30, 1, RED);
    kit::flowers(&mut b, 88, 30, 1, YELLOW);
    kit::bench(&mut b, 86, 34, 1);
    b.build()
}
