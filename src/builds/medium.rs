//! Medium: a striped lighthouse on a rocky island.

use super::kit::{self, hash, House};
use super::{Builder, Kind};
use crate::model::Model;
use crate::parts::col::*;
use crate::parts::Part;

pub fn build() -> Model {
    let mut b = Builder::new("Harbour Lighthouse", 1, 1, BLUE);
    b.period = 6;
    let (cx, cz) = (15.0, 15.0);
    let rock = |x: i32, z: i32, y: i32| if hash(x.div_euclid(4), y, z.div_euclid(4)) % 3 == 0 { DGRAY } else { LGRAY };

    // Island: three courses of rock, getting smaller.
    b.group = 1;
    for (k, r) in [9.5f32, 7.5, 6.5].iter().enumerate() {
        let y = 1 + k as i32 * 3;
        kit::ring(&mut b, cx, cz, 0.0, *r, y, Kind::Brick, |x, z| rock(x, z, y));
    }
    // Sandy top layer.
    kit::ring(&mut b, cx, cz, 0.0, 6.5, 10, Kind::Plate, |x, z| if hash(x, 7, z) % 5 == 0 { LIME } else { TAN });

    // Tower: 2-wide ring with red and white stripes.
    b.group = 2;
    let base = 11;
    let courses = 10;
    for c in 0..courses {
        let y = base + c * 3;
        let colr = if (c / 2) % 2 == 0 { RED } else { WHITE };
        kit::ring(&mut b, cx, cz, 2.0, 4.0, y, Kind::Brick, |_, _| colr);
    }
    // Door and small windows.
    b.paint(14, 18, 16, 19, base, base + 6, BROWN);
    for c in [3, 7] {
        b.paint(14, 18, 16, 19, base + c * 3, base + c * 3 + 1, SKY);
        b.paint(11, 14, 12, 16, base + (c + 1) * 3, base + (c + 1) * 3 + 1, SKY);
    }
    let mut y = base + courses * 3;

    // Gallery: two plate layers, wider than the tower.
    b.group = 3;
    kit::ring(&mut b, cx, cz, 0.0, 5.0, y, Kind::Plate, |_, _| BLACK);
    y += 1;
    // Railing.
    kit::ring(&mut b, cx, cz, 4.0, 5.0, y, Kind::Plate, |_, _| WHITE);
    // Lamp room: glass ring around a yellow light.
    for c in 0..2 {
        kit::ring(&mut b, cx, cz, 1.0, 3.0, y + c * 3, Kind::Brick, |_, _| SKY);
    }
    b.part(Part::Round { d: 2 }, 14, y, 14, 0, YELLOW);
    b.part(Part::Round { d: 2 }, 14, y + 3, 14, 0, YELLOW);
    y += 6;
    // Roof.
    kit::ring(&mut b, cx, cz, 0.0, 3.5, y, Kind::Plate, |_, _| RED);
    kit::ring(&mut b, cx, cz, 0.0, 2.5, y + 1, Kind::Brick, |_, _| RED);
    b.part(Part::Round { d: 2 }, 14, y + 4, 14, 0, DRED);
    b.part(Part::Round { d: 1 }, 14, y + 7, 14, 0, BLACK);
    b.part(Part::Round { d: 1 }, 15, y + 7, 15, 0, BLACK);
    b.part(Part::Plate { w: 2, l: 2 }, 14, y + 10, 14, 0, BLACK);

    // Keeper's cottage on the island.
    b.group = 4;
    kit::house(
        &mut b,
        &House { x: 7, z: 9, w: 4, d: 6, y: 11, wall: WHITE, roof: BLUE, trim: DGRAY, door_side: 3, ridge_x: false, ..Default::default() },
    );

    // Jetty on posts and a boat.
    b.group = 5;
    for x in [25, 28] {
        b.part(Part::Round { d: 2 }, x, 1, 18, 0, BROWN);
    }
    b.fill(24, 18, 31, 20, 4, Kind::Plate, BROWN);
    b.fill(23, 18, 31, 20, 5, Kind::Plate, TAN);
    kit::boat(&mut b, 21, 22, 1, RED);
    kit::boat(&mut b, 4, 26, 1, YELLOW);
    b.build()
}
