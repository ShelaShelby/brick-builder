//! Procedural part meshes (pure data, shared by the 3D view and the software renderer).
//!
//! Meshes are centred on the footprint at the bottom of the part, unrotated:
//! width along X, length along Z, height along +Y.

use crate::parts::{Part, Shape, PLATE_H, STUD};
use std::f32::consts::{FRAC_PI_2, TAU};

pub const GAP: f32 = 0.012;
pub const STUD_R: f32 = 0.3;
pub const STUD_H: f32 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lod {
    /// Smooth, for the 3D view.
    High,
    /// Few triangles, for the software renderer.
    Low,
}

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// Index offsets where a new sub-part (a stud) starts; used for outlines.
    pub group_marks: Vec<u32>,
}

impl MeshData {
    fn vert(&mut self, p: [f32; 3], n: [f32; 3]) -> u32 {
        self.positions.push(p);
        self.normals.push(n);
        (self.positions.len() - 1) as u32
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    /// Flat-shaded convex polygon, oriented to face away from `inside`.
    fn polygon(&mut self, pts: &[[f32; 3]], inside: [f32; 3]) {
        let mut pts = pts.to_vec();
        let mut n = normal_of(pts[0], pts[1], pts[2]);
        let k = pts.len() as f32;
        let fc = pts.iter().fold([0.0; 3], |a, p| [a[0] + p[0] / k, a[1] + p[1] / k, a[2] + p[2] / k]);
        let d = [fc[0] - inside[0], fc[1] - inside[1], fc[2] - inside[2]];
        if n[0] * d[0] + n[1] * d[1] + n[2] * d[2] < 0.0 {
            pts.reverse();
            n = [-n[0], -n[1], -n[2]];
        }
        let base = self.positions.len() as u32;
        for p in &pts {
            self.vert(*p, n);
        }
        for k in 1..pts.len() as u32 - 1 {
            self.tri(base, base + k, base + k + 1);
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Box with rounded edges between `min` and `max`.
    pub fn rounded_box(&mut self, min: [f32; 3], max: [f32; 3], r: f32, seg: u32) {
        let c = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5, (min[2] + max[2]) * 0.5];
        let h = [(max[0] - min[0]) * 0.5, (max[1] - min[1]) * 0.5, (max[2] - min[2]) * 0.5];
        let r = r.min(h[0] * 0.95).min(h[1] * 0.95).min(h[2] * 0.95).max(0.0);
        let inner = [h[0] - r, h[1] - r, h[2] - r];
        let seg = if r <= 0.0 { 0 } else { seg };
        let lists: Vec<Vec<f32>> = (0..3).map(|a| axis_points(h[a], inner[a], r, seg)).collect();
        for a in 0..3 {
            for s in [1.0f32, -1.0] {
                let (b, cax) = if s > 0.0 { ((a + 1) % 3, (a + 2) % 3) } else { ((a + 2) % 3, (a + 1) % 3) };
                let pb = &lists[b];
                let pc = &lists[cax];
                let base = self.positions.len() as u32;
                for &vb in pb {
                    for &vc in pc {
                        let mut p = [0.0; 3];
                        p[a] = s * h[a];
                        p[b] = vb;
                        p[cax] = vc;
                        let q = [
                            p[0].clamp(-inner[0], inner[0]),
                            p[1].clamp(-inner[1], inner[1]),
                            p[2].clamp(-inner[2], inner[2]),
                        ];
                        let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
                        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                        let (pos, n) = if len > 1e-6 && r > 0.0 {
                            let n = [d[0] / len, d[1] / len, d[2] / len];
                            ([q[0] + n[0] * r, q[1] + n[1] * r, q[2] + n[2] * r], n)
                        } else {
                            let mut n = [0.0; 3];
                            n[a] = s;
                            (p, n)
                        };
                        self.vert([pos[0] + c[0], pos[1] + c[1], pos[2] + c[2]], n);
                    }
                }
                let nb = pb.len() as u32;
                let nc = pc.len() as u32;
                for i in 0..nb - 1 {
                    for j in 0..nc - 1 {
                        let v00 = base + i * nc + j;
                        let v10 = base + (i + 1) * nc + j;
                        let v11 = base + (i + 1) * nc + j + 1;
                        let v01 = base + i * nc + j + 1;
                        self.tri(v00, v10, v11);
                        self.tri(v00, v11, v01);
                    }
                }
            }
        }
    }

    /// Vertical cylinder centred at (cx, cz) from y0 to y1.
    pub fn cylinder(&mut self, cx: f32, cz: f32, y0: f32, y1: f32, rad: f32, segs: u32, bottom: bool) {
        let base = self.positions.len() as u32;
        for k in 0..=segs {
            let a = k as f32 / segs as f32 * TAU;
            let (s, c) = a.sin_cos();
            let n = [c, 0.0, s];
            self.vert([cx + c * rad, y0, cz + s * rad], n);
            self.vert([cx + c * rad, y1, cz + s * rad], n);
        }
        for k in 0..segs {
            let b0 = base + k * 2;
            let t0 = b0 + 1;
            let b1 = b0 + 2;
            let t1 = b0 + 3;
            self.tri(b0, t0, t1);
            self.tri(b0, t1, b1);
        }
        // top cap
        let centre = self.vert([cx, y1, cz], [0.0, 1.0, 0.0]);
        let ring = self.positions.len() as u32;
        for k in 0..segs {
            let a = k as f32 / segs as f32 * TAU;
            let (s, c) = a.sin_cos();
            self.vert([cx + c * rad, y1, cz + s * rad], [0.0, 1.0, 0.0]);
        }
        for k in 0..segs {
            self.tri(centre, ring + (k + 1) % segs, ring + k);
        }
        if bottom {
            let centre = self.vert([cx, y0, cz], [0.0, -1.0, 0.0]);
            let ring = self.positions.len() as u32;
            for k in 0..segs {
                let a = k as f32 / segs as f32 * TAU;
                let (s, c) = a.sin_cos();
                self.vert([cx + c * rad, y0, cz + s * rad], [0.0, -1.0, 0.0]);
            }
            for k in 0..segs {
                self.tri(centre, ring + k, ring + (k + 1) % segs);
            }
        }
    }
}

fn axis_points(h: f32, inner: f32, r: f32, seg: u32) -> Vec<f32> {
    let mut v = Vec::new();
    if seg == 0 {
        return vec![-h, h];
    }
    for k in 0..=seg {
        let t = k as f32 / seg as f32 * FRAC_PI_2;
        v.push(-inner - r * t.cos());
    }
    if inner > 1e-5 {
        for k in (0..=seg).rev() {
            let t = k as f32 / seg as f32 * FRAC_PI_2;
            v.push(inner + r * t.cos());
        }
    } else {
        for k in (0..seg).rev() {
            let t = k as f32 / seg as f32 * FRAC_PI_2;
            v.push(inner + r * t.cos());
        }
    }
    v
}

fn normal_of(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
    [n[0] / l, n[1] / l, n[2] / l]
}

/// Build the mesh of one part.
pub fn part_mesh(part: Part, lod: Lod) -> MeshData {
    let mut m = MeshData::default();
    let (w, l) = part.size();
    let hw = w as f32 * STUD * 0.5;
    let hl = l as f32 * STUD * 0.5;
    let top = part.height() as f32 * PLATE_H;
    let (round_r, round_seg, stud_segs) = match lod {
        Lod::High => (0.05, 2, 16),
        Lod::Low => (0.0, 0, 8),
    };
    let stud_segs = if part == Part::Baseplate {
        if lod == Lod::High {
            12
        } else {
            6
        }
    } else {
        stud_segs
    };
    match part.shape() {
        Shape::Box | Shape::Tile => {
            m.rounded_box([-hw + GAP, 0.0, -hl + GAP], [hw - GAP, top - GAP * 0.5, hl - GAP], round_r, round_seg);
        }
        Shape::Baseplate => {
            m.rounded_box([-hw, 0.0, -hl], [hw, top, hl], round_r * 0.5, round_seg.min(1));
        }
        Shape::Round => {
            let rad = w as f32 * 0.5 - GAP;
            let segs = match lod {
                Lod::High => 32,
                Lod::Low => 14,
            };
            m.cylinder(0.0, 0.0, 0.0, top - GAP * 0.5, rad, segs, true);
            if w >= 2 && lod == Lod::High {
                // A thin ring that reads as a rounded top edge.
                m.cylinder(0.0, 0.0, top - GAP * 0.5, top - GAP * 0.5 + 0.02, rad * 0.94, segs, false);
            }
        }
        Shape::Slope => {
            let x0 = -hw + GAP;
            let x1 = hw - GAP;
            let zf = -hl + GAP; // low front edge
            let zm = -hl + STUD; // where the slope reaches full height
            let zb = hl - GAP; // back
            let lip = PLATE_H * 0.7;
            let t = top - GAP * 0.5;
            let inside = [0.0, t * 0.4, 0.3];
            // front lip
            m.polygon(&[[x0, 0.0, zf], [x1, 0.0, zf], [x1, lip, zf], [x0, lip, zf]], inside);
            // slope face
            m.polygon(&[[x0, lip, zf], [x1, lip, zf], [x1, t, zm], [x0, t, zm]], inside);
            // flat top
            m.polygon(&[[x0, t, zm], [x1, t, zm], [x1, t, zb], [x0, t, zb]], inside);
            // back
            m.polygon(&[[x1, 0.0, zb], [x0, 0.0, zb], [x0, t, zb], [x1, t, zb]], inside);
            // bottom
            m.polygon(&[[x0, 0.0, zf], [x0, 0.0, zb], [x1, 0.0, zb], [x1, 0.0, zf]], inside);
            // sides
            m.polygon(&[[x1, 0.0, zf], [x1, 0.0, zb], [x1, t, zb], [x1, t, zm], [x1, lip, zf]], inside);
            m.polygon(&[[x0, 0.0, zb], [x0, 0.0, zf], [x0, lip, zf], [x0, t, zm], [x0, t, zb]], inside);
        }
    }
    let stud_base = if part.shape() == Shape::Baseplate { top } else { top - GAP * 0.5 };
    for (i, j) in part.stud_cells() {
        let cx = (i as f32 + 0.5) * STUD - hw;
        let cz = (j as f32 + 0.5) * STUD - hl;
        m.group_marks.push(m.indices.len() as u32);
        m.cylinder(cx, cz, stud_base - 0.01, stud_base + STUD_H, STUD_R, stud_segs, false);
    }
    m
}
