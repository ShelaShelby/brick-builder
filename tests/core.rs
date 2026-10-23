use brick_builder::builds;
use brick_builder::connect::{analyze, Status};
use brick_builder::instructions;
use brick_builder::model::{Brick, Model, Occupancy};
use brick_builder::parts::{Part, BASEPLATE_SIZE};

fn bp() -> Brick {
    Brick::new(Part::Baseplate, 0, 0, 0, 0, 0)
}
fn brick(w: u8, l: u8, x: i32, y: i32, z: i32, rot: u8) -> Brick {
    Brick::new(Part::Brick { w, l }, x, y, z, rot, 4)
}
fn model(bricks: &[Brick]) -> Vec<Brick> {
    let mut m = Model::new("t");
    for b in bricks {
        m.add(*b);
    }
    m.bricks
}

#[test]
fn stacked_bricks_connect() {
    let b = model(&[bp(), brick(2, 4, 0, 1, 0, 0), brick(2, 2, 0, 4, 0, 0)]);
    let a = analyze(&b);
    assert_eq!(a.status, vec![Status::Ok; 3]);
    assert_eq!(a.joints, 8 + 4);
}

#[test]
fn side_by_side_holds_nothing() {
    // Second brick touches the first only at the side, at the same height, off the baseplate.
    let b = model(&[bp(), brick(1, 2, 0, 1, 0, 0), brick(1, 2, 40, 1, 0, 0), brick(1, 2, 41, 1, 0, 0)]);
    let a = analyze(&b);
    assert_eq!(a.status[1], Status::Ok);
    assert_eq!(a.status[2], Status::Floating);
    assert_eq!(a.status[3], Status::Floating);
    assert_eq!(a.floating, 2);
}

#[test]
fn vertical_gap_holds_nothing() {
    // A brick hovering one plate above another has no joints.
    let b = model(&[bp(), brick(2, 2, 0, 1, 0, 0), brick(2, 2, 0, 5, 0, 0)]);
    let a = analyze(&b);
    assert_eq!(a.status[2], Status::Floating);
}

#[test]
fn single_stud_is_weak() {
    // 1x4 brick resting on one stud of a tall column.
    let b = model(&[bp(), brick(2, 2, 0, 1, 0, 0), brick(1, 4, 1, 4, 1, 0)]);
    let a = analyze(&b);
    assert_eq!(a.status[1], Status::Ok);
    assert_eq!(a.status[2], Status::Weak);
    assert_eq!(a.weak, 1);
}

#[test]
fn weak_group_flags_everything_behind_the_joint() {
    // Column on one stud, with more bricks stacked firmly on it.
    let b = model(&[
        bp(),
        brick(2, 2, 0, 1, 0, 0),
        brick(1, 4, 1, 4, 1, 0), // one stud on the 2x2
        brick(1, 4, 1, 7, 1, 0), // fully on the 1x4
        brick(2, 2, 5, 1, 5, 0), // unrelated, fine
    ]);
    let a = analyze(&b);
    assert_eq!(a.status, vec![Status::Ok, Status::Ok, Status::Weak, Status::Weak, Status::Ok]);
}

#[test]
fn two_single_joints_are_not_weak() {
    // A 1x4 bridging two columns by one stud each is held twice.
    let b = model(&[bp(), brick(1, 1, 0, 1, 0, 0), brick(1, 1, 3, 1, 0, 0), brick(1, 4, 0, 4, 0, 1)]);
    let a = analyze(&b);
    assert_eq!(a.status[3], Status::Ok);
    // ...but each 1x1 column foot sits on one stud and is also held through the bridge.
    assert_eq!(a.status[1], Status::Ok);
    assert_eq!(a.weak, 0);
}

#[test]
fn tiles_and_slopes_have_limited_studs() {
    let tile = Brick::new(Part::Tile { d: 2 }, 0, 1, 0, 0, 0);
    let on_tile = brick(2, 2, 0, 2, 0, 0);
    let a = analyze(&model(&[bp(), tile, on_tile]));
    assert_eq!(a.status[2], Status::Floating);

    // Slope 2x2 facing -z: studs only on the back row (z = 1).
    let slope = Brick::new(Part::Slope { w: 2 }, 0, 1, 0, 0, 0);
    assert_eq!(slope.stud_world(), vec![(0, 1), (1, 1)]);
    let front = brick(1, 2, 0, 4, 0, 1); // over the sloped front row
    let back = brick(1, 2, 0, 4, 1, 1); // over the studded back row
    let a = analyze(&model(&[bp(), slope, front]));
    assert_eq!(a.status[2], Status::Floating);
    let a = analyze(&model(&[bp(), slope, back]));
    assert_eq!(a.status[2], Status::Ok);
}

#[test]
fn rotation_maps_footprint() {
    for rot in 0..4u8 {
        let b = brick(2, 4, 3, 1, 5, rot);
        let (fw, fl) = b.footprint();
        let mut studs = b.stud_world();
        studs.sort();
        let mut cells: Vec<_> = b.cells().collect();
        cells.sort();
        assert_eq!(studs, cells, "rot {rot}");
        assert_eq!(studs.len(), 8);
        assert_eq!((fw * fl), 8);
    }
    // Slope facing +z after a half turn: studs on the low-z row.
    let s = Brick::new(Part::Slope { w: 2 }, 0, 1, 0, 2, 0);
    let mut st = s.stud_world();
    st.sort();
    assert_eq!(st, vec![(0, 0), (1, 0)]);
}

#[test]
fn save_and_load_round_trip() {
    let m = (builds::catalog()[0].make)();
    let json = m.to_json();
    let back = Model::from_json(&json).unwrap();
    assert_eq!(back.bricks, m.bricks);
    assert!(Model::from_json("{\"nope\":1}").is_err());
}

#[test]
fn instructions_handle_floating_user_bricks() {
    let b = model(&[bp(), brick(2, 2, 0, 1, 0, 0), brick(2, 2, 10, 9, 10, 0)]);
    let ins = instructions::generate(&b);
    let total: usize = ins.steps.iter().map(|s| s.len()).sum();
    assert_eq!(total, 3);
}

#[test]
fn all_builds_are_valid() {
    let ranges = [(40, 80), (200, 400), (700, 1000), (1500, 2000), (2800, usize::MAX)];
    for (info, (lo, hi)) in builds::catalog().iter().zip(ranges) {
        let m = (info.make)();
        let n = m.brick_count();
        assert!(n >= lo && n <= hi, "{}: {} bricks not in {}..{}", info.name, n, lo, hi);

        // Zero flagged bricks.
        let a = analyze(&m.bricks);
        assert_eq!(a.weak, 0, "{} has weak bricks", info.name);
        assert_eq!(a.floating, 0, "{} has floating bricks", info.name);

        // No two parts share space, and everything sits on the baseplates.
        let occ = Occupancy::build(&m.bricks);
        let volume: usize = m.bricks.iter().map(|b| b.cells().count() * b.part.height() as usize).sum();
        assert_eq!(occ.cells.len(), volume, "{} has overlapping parts", info.name);
        let s = BASEPLATE_SIZE as i32;
        let bases: Vec<_> = m.bricks.iter().filter(|b| b.part == Part::Baseplate).collect();
        for b in &m.bricks {
            for (x, z) in b.cells() {
                assert!(
                    bases.iter().any(|p| x >= p.x && x < p.x + s && z >= p.z && z < p.z + s),
                    "{}: part off the baseplates at {x},{z}",
                    info.name
                );
            }
        }

        // Instructions: every brick once, each supported when placed.
        let ins = instructions::generate(&m.bricks);
        instructions::validate(&m.bricks, &ins).unwrap_or_else(|e| panic!("{}: {e}", info.name));
        let per_step = instructions::step_size(n);
        assert!(ins.steps.iter().skip(1).all(|s| s.len() <= per_step), "{}", info.name);
        assert!(ins.len() >= 10, "{} has only {} steps", info.name, ins.len());
    }
}

#[test]
fn pdf_has_a_page_per_step() {
    let m = (builds::catalog()[0].make)();
    let ins = instructions::generate(&m.bricks);
    let pdf = brick_builder::pdf::manual_pdf(&m, &ins, None);
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains(&format!("/Count {}", ins.len() + 1)));
    assert!(text.trim_end().ends_with("%%EOF"));
}

