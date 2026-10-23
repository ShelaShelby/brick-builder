//! Small CPU renderer for thumbnails, part icons and PDF step pictures.
//!
//! Orthographic view, z-buffer, simple directional lighting and outlines
//! between parts, which gives a clean "instruction booklet" look.

use crate::meshgen::{part_mesh, Lod, MeshData};
use crate::model::Brick;
use crate::parts::{color_f32, Part};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Normal,
    Dim,
    Hidden,
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    /// RGB, row-major.
    pub rgb: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct View {
    /// Direction from the target towards the camera.
    pub eye: [f32; 3],
    pub background: [f32; 3],
    pub margin: f32,
    pub supersample: u32,
}

impl Default for View {
    fn default() -> Self {
        View { eye: [0.75, 0.9, 1.0], background: [1.0, 1.0, 1.0], margin: 0.04, supersample: 2 }
    }
}

pub struct Renderer {
    meshes: HashMap<Part, MeshData>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

struct Cam {
    r: [f32; 3],
    u: [f32; 3],
    f: [f32; 3],
    scale: f32,
    ox: f32,
    oy: f32,
}

impl Cam {
    fn project(&self, p: [f32; 3]) -> [f32; 3] {
        [
            dot(p, self.r) * self.scale + self.ox,
            -dot(p, self.u) * self.scale + self.oy,
            dot(p, self.f),
        ]
    }
}

impl Renderer {
    pub fn new() -> Self {
        let meshes = Part::all().into_iter().map(|p| (p, part_mesh(p, Lod::Low))).collect();
        Renderer { meshes }
    }

    /// Render `bricks` (with per-brick looks) framed on `frame` bricks.
    pub fn render(
        &self,
        bricks: &[Brick],
        looks: &[Look],
        frame: &[Brick],
        width: u32,
        height: u32,
        view: &View,
    ) -> Image {
        let ss = view.supersample.max(1);
        let w = (width * ss) as usize;
        let h = (height * ss) as usize;
        let e = norm(view.eye);
        let f = [-e[0], -e[1], -e[2]];
        let r = norm(cross(f, [0.0, 1.0, 0.0]));
        let u = cross(r, f);
        let mut cam = Cam { r, u, f, scale: 1.0, ox: 0.0, oy: 0.0 };

        // Fit the frame bricks into the image.
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for b in frame.iter() {
            let (mn, mx) = b.aabb();
            for k in 0..8 {
                let p = [
                    if k & 1 == 0 { mn[0] } else { mx[0] },
                    if k & 2 == 0 { mn[1] } else { mx[1] + 0.2 },
                    if k & 4 == 0 { mn[2] } else { mx[2] },
                ];
                let q = cam.project(p);
                x0 = x0.min(q[0]);
                x1 = x1.max(q[0]);
                y0 = y0.min(q[1]);
                y1 = y1.max(q[1]);
            }
        }
        if x0 > x1 {
            x0 = -1.0;
            x1 = 1.0;
            y0 = -1.0;
            y1 = 1.0;
        }
        let m = view.margin;
        let sx = w as f32 * (1.0 - 2.0 * m) / (x1 - x0).max(1e-3);
        let sy = h as f32 * (1.0 - 2.0 * m) / (y1 - y0).max(1e-3);
        cam.scale = sx.min(sy);
        cam.ox = w as f32 * 0.5 - (x0 + x1) * 0.5 * cam.scale;
        cam.oy = h as f32 * 0.5 - (y0 + y1) * 0.5 * cam.scale;

        let mut zbuf = vec![f32::MAX; w * h];
        let mut ids = vec![u32::MAX; w * h];
        let mut col = vec![view.background; w * h];
        let light = norm([0.3, 1.0, 0.75]);

        for (bi, b) in bricks.iter().enumerate() {
            let look = looks.get(bi).copied().unwrap_or(Look::Normal);
            if look == Look::Hidden {
                continue;
            }
            let mesh = &self.meshes[&b.part];
            let o = b.world_origin();
            let (sa, ca) = b.angle().sin_cos();
            let tf = |p: [f32; 3]| [ca * p[0] + sa * p[2] + o[0], p[1] + o[1], -sa * p[0] + ca * p[2] + o[2]];
            let tn = |n: [f32; 3]| [ca * n[0] + sa * n[2], n[1], -sa * n[0] + ca * n[2]];
            let mut base = color_f32(b.color);
            if look == Look::Dim {
                for c in &mut base {
                    *c = *c * 0.35 + 0.62;
                }
            }
            let ambient = if look == Look::Dim { 0.8 } else { 0.5 };
            let pts: Vec<[f32; 3]> = mesh.positions.iter().map(|p| cam.project(tf(*p))).collect();
            let mut group = 0u32;
            let mut mark = 0usize;
            for (t, tri) in mesh.indices.chunks_exact(3).enumerate() {
                while mark < mesh.group_marks.len() && mesh.group_marks[mark] as usize <= t * 3 {
                    mark += 1;
                    group = mark as u32;
                }
                let (a, bb, c) = (pts[tri[0] as usize], pts[tri[1] as usize], pts[tri[2] as usize]);
                // Back-face cull (screen y points down, so CCW becomes negative area).
                let area = (bb[0] - a[0]) * (c[1] - a[1]) - (bb[1] - a[1]) * (c[0] - a[0]);
                if area >= 0.0 {
                    continue;
                }
                let n = tn(norm([
                    mesh.normals[tri[0] as usize][0] + mesh.normals[tri[1] as usize][0] + mesh.normals[tri[2] as usize][0],
                    mesh.normals[tri[0] as usize][1] + mesh.normals[tri[1] as usize][1] + mesh.normals[tri[2] as usize][1],
                    mesh.normals[tri[0] as usize][2] + mesh.normals[tri[1] as usize][2] + mesh.normals[tri[2] as usize][2],
                ]));
                let lit = ambient + (1.0 - ambient) * dot(n, light).max(0.0);
                let shade = [base[0] * lit, base[1] * lit, base[2] * lit];
                let id = (bi as u32) * 64 + group.min(63);
                raster_tri(a, bb, c, w, h, &mut zbuf, &mut ids, &mut col, shade, id);
            }
        }

        // Outlines where the part id changes.
        let mut out = col.clone();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let id = ids[i];
                let mut edge = false;
                let mut nearest = i;
                for (dx, dy) in [(1i32, 0i32), (0, 1), (-1, 0), (0, -1)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if ids[j] != id {
                        // Draw the line on the nearer of the two pixels.
                        if zbuf[i] <= zbuf[j] {
                            edge = true;
                            nearest = i;
                        }
                    }
                }
                if edge && id != u32::MAX {
                    let dim = bricks
                        .get((id / 64) as usize)
                        .map(|_| looks.get((id / 64) as usize).copied() == Some(Look::Dim))
                        .unwrap_or(false);
                    let k = if dim { 0.78 } else { 0.25 };
                    let c = col[nearest];
                    out[i] = [c[0] * k, c[1] * k, c[2] * k];
                }
            }
        }

        // Downsample.
        let mut rgb = vec![0u8; (width * height * 3) as usize];
        let inv = 1.0 / (ss * ss) as f32;
        for y in 0..height as usize {
            for x in 0..width as usize {
                let mut acc = [0.0f32; 3];
                for sy in 0..ss as usize {
                    for sx in 0..ss as usize {
                        let c = out[(y * ss as usize + sy) * w + x * ss as usize + sx];
                        acc[0] += c[0];
                        acc[1] += c[1];
                        acc[2] += c[2];
                    }
                }
                let o = (y * width as usize + x) * 3;
                for k in 0..3 {
                    rgb[o + k] = (acc[k] * inv * 255.0).clamp(0.0, 255.0) as u8;
                }
            }
        }
        Image { width, height, rgb }
    }

    /// Icon of a single part in a colour.
    pub fn part_icon(&self, part: Part, color: u8, size: u32, background: [f32; 3]) -> Image {
        let b = Brick::new(part, 0, 0, 0, 0, color);
        let view = View { background, margin: 0.08, ..Default::default() };
        self.render(&[b], &[Look::Normal], &[b], size, size, &view)
    }
}

#[allow(clippy::too_many_arguments)]
fn raster_tri(
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    w: usize,
    h: usize,
    zbuf: &mut [f32],
    ids: &mut [u32],
    col: &mut [[f32; 3]],
    shade: [f32; 3],
    id: u32,
) {
    let minx = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i32;
    let maxx = a[0].max(b[0]).max(c[0]).ceil().min(w as f32 - 1.0) as i32;
    let miny = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i32;
    let maxy = a[1].max(b[1]).max(c[1]).ceil().min(h as f32 - 1.0) as i32;
    if minx > maxx || miny > maxy {
        return;
    }
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if area.abs() < 1e-8 {
        return;
    }
    let inv = 1.0 / area;
    for y in miny..=maxy {
        let py = y as f32 + 0.5;
        for x in minx..=maxx {
            let px = x as f32 + 0.5;
            let w0 = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) * inv;
            let w1 = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) * inv;
            let w2 = 1.0 - w0 - w1;
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let z = w0 * a[2] + w1 * b[2] + w2 * c[2];
            let i = y as usize * w + x as usize;
            if z < zbuf[i] {
                zbuf[i] = z;
                ids[i] = id;
                col[i] = shade;
            }
        }
    }
}

impl Image {
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.rgb.len() / 3 * 4);
        for p in self.rgb.chunks_exact(3) {
            v.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        v
    }

    pub fn to_jpeg(&self, quality: u8) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        enc.encode(&self.rgb, self.width, self.height, image::ExtendedColorType::Rgb8)
            .expect("jpeg encode");
        out
    }

    pub fn save_png(&self, path: &std::path::Path) -> Result<(), String> {
        image::save_buffer(path, &self.rgb, self.width, self.height, image::ExtendedColorType::Rgb8)
            .map_err(|e| e.to_string())
    }
}
