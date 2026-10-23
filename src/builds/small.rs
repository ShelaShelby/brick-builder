//! Small: a cottage with a garden.

use super::kit::{self, House};
use super::{Builder, Kind};
use crate::model::Model;
use crate::parts::col::*;
use crate::parts::Part;

pub fn build() -> Model {
    let mut b = Builder::new("Cozy Cottage", 1, 1, GREEN);
    b.period = 6;
    b.group = 1;
    kit::house(
        &mut b,
        &House { x: 10, z: 10, w: 8, d: 6, y: 1, wall: TAN, roof: RED, trim: WHITE, door: BROWN, band: false, ..Default::default() },
    );
    b.group = 2;
    // Garden path from the door to the edge.
    for k in 0..3 {
        b.part(Part::Tile { d: 2 }, 13, 1, 16 + k * 2, 0, LGRAY);
    }
    kit::tree(&mut b, 22, 12, 1, false);
    kit::flowers(&mut b, 6, 18, 1, RED);
    kit::flowers(&mut b, 18, 18, 1, YELLOW);
    kit::fence(&mut b, 8, 24, 1, 1, true, WHITE);
    kit::fence(&mut b, 17, 24, 1, 1, true, WHITE);
    let _ = Kind::Brick;
    b.build()
}
