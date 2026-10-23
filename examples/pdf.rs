//! Developer tool: `cargo run --release --example pdf -- <index> <out.pdf>`
use brick_builder::{builds, instructions, pdf};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let i: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let out = a.get(2).cloned().unwrap_or_else(|| "manual.pdf".into());
    let m = (builds::catalog()[i].make)();
    let ins = instructions::generate(&m.bricks);
    let t = std::time::Instant::now();
    let bytes = pdf::manual_pdf(&m, &ins, None);
    std::fs::write(&out, &bytes).unwrap();
    println!("{} pages, {} KB, {:?}", ins.len() + 1, bytes.len() / 1024, t.elapsed());
}
