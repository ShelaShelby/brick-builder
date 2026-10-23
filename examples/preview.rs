//! Developer tool: render every ready-made build to PNG and print stats.
//! `cargo run --release --example preview -- <out_dir> [index]`

use brick_builder::{builds, connect, instructions, raster};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = std::path::PathBuf::from(args.get(1).cloned().unwrap_or_else(|| ".".into()));
    let only: Option<usize> = args.get(2).and_then(|s| s.parse().ok());
    let r = raster::Renderer::new();
    for (i, info) in builds::catalog().iter().enumerate() {
        if only.is_some_and(|o| o != i) {
            continue;
        }
        let t = std::time::Instant::now();
        let m = (info.make)();
        let gen = t.elapsed();
        let a = connect::analyze(&m.bricks);
        let ins = instructions::generate(&m.bricks);
        let valid = instructions::validate(&m.bricks, &ins);
        println!(
            "{} [{}]: {} bricks, {} joints, weak {}, floating {}, {} steps, valid {:?}, gen {:?}",
            info.name, info.size, m.brick_count(), a.joints, a.weak, a.floating, ins.len(), valid, gen
        );
        for (i, a) in m.bricks.iter().enumerate() {
            for b in &m.bricks[i + 1..] {
                if a.overlaps(b) {
                    println!("  OVERLAP {:?}@{},{},{} r{} g{} <-> {:?}@{},{},{} r{} g{}", a.part, a.x, a.y, a.z, a.rot, a.group, b.part, b.x, b.y, b.z, b.rot, b.group);
                }
            }
        }
        let mut groups = std::collections::BTreeMap::new();
        for b in &m.bricks {
            *groups.entry(b.group).or_insert(0) += 1;
        }
        println!("  per group: {groups:?}");
        let looks = vec![raster::Look::Normal; m.bricks.len()];
        let view = raster::View::default();
        let img = r.render(&m.bricks, &looks, &m.bricks, 1200, 900, &view);
        img.save_png(&out.join(format!("build{i}.png"))).unwrap();
        let back = raster::View { eye: [-0.8, 0.9, -1.0], ..view };
        let img = r.render(&m.bricks, &looks, &m.bricks, 1200, 900, &back);
        img.save_png(&out.join(format!("build{i}_back.png"))).unwrap();
    }
}
