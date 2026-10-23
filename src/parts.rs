//! Part library: generic brick shapes measured in studs (x/z) and plates (y).

use serde::{Deserialize, Serialize};

/// One stud pitch in world units.
pub const STUD: f32 = 1.0;
/// Height of one plate in world units (a brick is three plates tall).
pub const PLATE_H: f32 = 0.4;
/// Size of the square baseplate in studs.
pub const BASEPLATE_SIZE: u8 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Part {
    /// Standard brick, `w` x `l` studs, three plates tall.
    Brick { w: u8, l: u8 },
    /// Flat plate, one plate tall.
    Plate { w: u8, l: u8 },
    /// Round brick, `d` x `d` studs (1 or 2).
    Round { d: u8 },
    /// 45 degree slope, `w` wide and two studs deep; studs only on the high back row.
    Slope { w: u8 },
    /// Smooth tile without studs, `d` x `d`.
    Tile { d: u8 },
    /// Big flat baseplate. Baseplates are the ground every build is anchored to.
    Baseplate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Box,
    Round,
    Slope,
    Tile,
    Baseplate,
}

pub const BRICK_SIZES: [(u8, u8); 10] = [
    (1, 1),
    (1, 2),
    (1, 3),
    (1, 4),
    (1, 6),
    (2, 2),
    (2, 3),
    (2, 4),
    (2, 6),
    (2, 8),
];

impl Part {
    /// Every part in the library, in palette order.
    pub fn all() -> Vec<Part> {
        let mut v = Vec::new();
        for (w, l) in BRICK_SIZES {
            v.push(Part::Brick { w, l });
        }
        for (w, l) in BRICK_SIZES {
            v.push(Part::Plate { w, l });
        }
        v.push(Part::Round { d: 1 });
        v.push(Part::Round { d: 2 });
        v.push(Part::Slope { w: 1 });
        v.push(Part::Slope { w: 2 });
        v.push(Part::Tile { d: 1 });
        v.push(Part::Tile { d: 2 });
        v.push(Part::Baseplate);
        v
    }

    /// Footprint before rotation: (width along local X, length along local Z).
    pub fn size(self) -> (i32, i32) {
        match self {
            Part::Brick { w, l } | Part::Plate { w, l } => (w as i32, l as i32),
            Part::Round { d } | Part::Tile { d } => (d as i32, d as i32),
            Part::Slope { w } => (w as i32, 2),
            Part::Baseplate => (BASEPLATE_SIZE as i32, BASEPLATE_SIZE as i32),
        }
    }

    /// Height in plates.
    pub fn height(self) -> i32 {
        match self {
            Part::Brick { .. } | Part::Round { .. } | Part::Slope { .. } => 3,
            Part::Plate { .. } | Part::Tile { .. } | Part::Baseplate => 1,
        }
    }

    pub fn shape(self) -> Shape {
        match self {
            Part::Brick { .. } | Part::Plate { .. } => Shape::Box,
            Part::Round { .. } => Shape::Round,
            Part::Slope { .. } => Shape::Slope,
            Part::Tile { .. } => Shape::Tile,
            Part::Baseplate => Shape::Baseplate,
        }
    }

    /// Local cells (i along X, j along Z) that carry a stud on top.
    pub fn stud_cells(self) -> Vec<(i32, i32)> {
        let (w, l) = self.size();
        match self.shape() {
            Shape::Tile => Vec::new(),
            Shape::Slope => (0..w).map(|i| (i, l - 1)).collect(),
            _ => cells(w, l),
        }
    }

    /// Local cells whose underside accepts studs from below.
    pub fn socket_cells(self) -> Vec<(i32, i32)> {
        match self {
            Part::Baseplate => Vec::new(),
            _ => {
                let (w, l) = self.size();
                cells(w, l)
            }
        }
    }

    pub fn name(self) -> String {
        match self {
            Part::Brick { w, l } => format!("Brick {w}x{l}"),
            Part::Plate { w, l } => format!("Plate {w}x{l}"),
            Part::Round { d } => format!("Round brick {d}x{d}"),
            Part::Slope { w } => format!("Slope {w}x2"),
            Part::Tile { d } => format!("Tile {d}x{d}"),
            Part::Baseplate => format!("Baseplate {0}x{0}", BASEPLATE_SIZE),
        }
    }

    pub fn short_name(self) -> String {
        match self {
            Part::Brick { w, l } | Part::Plate { w, l } => format!("{w}x{l}"),
            Part::Round { d } | Part::Tile { d } => format!("{d}x{d}"),
            Part::Slope { w } => format!("{w}x2"),
            Part::Baseplate => "32x32".into(),
        }
    }
}

fn cells(w: i32, l: i32) -> Vec<(i32, i32)> {
    let mut v = Vec::with_capacity((w * l) as usize);
    for i in 0..w {
        for j in 0..l {
            v.push((i, j));
        }
    }
    v
}

/// A colour in the palette.
#[derive(Clone, Copy, Debug)]
pub struct PaletteColor {
    pub name: &'static str,
    pub rgb: [u8; 3],
}

pub const PALETTE: [PaletteColor; 14] = [
    PaletteColor { name: "White", rgb: [244, 244, 240] },
    PaletteColor { name: "Light grey", rgb: [160, 165, 169] },
    PaletteColor { name: "Dark grey", rgb: [99, 95, 97] },
    PaletteColor { name: "Black", rgb: [33, 33, 36] },
    PaletteColor { name: "Red", rgb: [201, 26, 9] },
    PaletteColor { name: "Dark red", rgb: [114, 14, 15] },
    PaletteColor { name: "Orange", rgb: [254, 138, 24] },
    PaletteColor { name: "Yellow", rgb: [242, 205, 55] },
    PaletteColor { name: "Tan", rgb: [228, 205, 158] },
    PaletteColor { name: "Brown", rgb: [88, 57, 39] },
    PaletteColor { name: "Green", rgb: [35, 120, 65] },
    PaletteColor { name: "Lime", rgb: [165, 202, 24] },
    PaletteColor { name: "Blue", rgb: [0, 85, 191] },
    PaletteColor { name: "Sky blue", rgb: [120, 191, 234] },
];

/// Palette indices by name, used by the build generators.
pub mod col {
    pub const WHITE: u8 = 0;
    pub const LGRAY: u8 = 1;
    pub const DGRAY: u8 = 2;
    pub const BLACK: u8 = 3;
    pub const RED: u8 = 4;
    pub const DRED: u8 = 5;
    pub const ORANGE: u8 = 6;
    pub const YELLOW: u8 = 7;
    pub const TAN: u8 = 8;
    pub const BROWN: u8 = 9;
    pub const GREEN: u8 = 10;
    pub const LIME: u8 = 11;
    pub const BLUE: u8 = 12;
    pub const SKY: u8 = 13;
}

pub fn color_rgb(c: u8) -> [u8; 3] {
    PALETTE[(c as usize).min(PALETTE.len() - 1)].rgb
}

pub fn color_name(c: u8) -> &'static str {
    PALETTE[(c as usize).min(PALETTE.len() - 1)].name
}

/// Linear-ish float colour (sRGB 0..1).
pub fn color_f32(c: u8) -> [f32; 3] {
    let [r, g, b] = color_rgb(c);
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}
