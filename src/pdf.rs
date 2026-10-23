//! Minimal PDF writer for instruction manuals: one page per step with the
//! step picture and the parts needed.

use crate::instructions::Instructions;
use crate::model::{parts_list, Model};
use crate::parts::{color_name, color_rgb, Part};
use crate::raster::{Look, Renderer, View};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Writer {
    objects: Vec<Option<Vec<u8>>>,
}

impl Writer {
    fn new() -> Self {
        // Object 0 is the free-list head.
        Writer { objects: vec![None] }
    }
    fn reserve(&mut self) -> usize {
        self.objects.push(None);
        self.objects.len() - 1
    }
    fn set(&mut self, id: usize, body: Vec<u8>) {
        self.objects[id] = Some(body);
    }
    fn add(&mut self, body: Vec<u8>) -> usize {
        let id = self.reserve();
        self.set(id, body);
        id
    }
    fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
        let mut v = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        v.extend_from_slice(data);
        v.extend_from_slice(b"\nendstream");
        v
    }
    fn jpeg(&mut self, w: u32, h: u32, data: &[u8]) -> usize {
        let dict = format!(
            "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"
        );
        self.add(Self::stream(&dict, data))
    }
    fn finish(self, root: usize) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = vec![0usize; self.objects.len()];
        for (id, obj) in self.objects.iter().enumerate().skip(1) {
            offsets[id] = out.len();
            out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
            out.extend_from_slice(obj.as_deref().unwrap_or(b"null"));
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", self.objects.len()).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets.iter().skip(1) {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size {} /Root {root} 0 R >>\nstartxref\n{xref}\n%%EOF\n", self.objects.len()).as_bytes(),
        );
        out
    }
}

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii())
        .flat_map(|c| match c {
            '(' | ')' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect()
}

struct Page {
    content: String,
    images: Vec<(String, usize)>,
}

const PW: f32 = 842.0;
const PH: f32 = 595.0;

fn text(s: &mut String, font: &str, size: f32, x: f32, y: f32, t: &str) {
    s.push_str(&format!("BT /{font} {size} Tf {x:.1} {y:.1} Td ({}) Tj ET\n", esc(t)));
}

fn rgb_fill(s: &mut String, c: [u8; 3]) {
    s.push_str(&format!("{:.3} {:.3} {:.3} rg\n", c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0));
}

fn place_image(s: &mut String, name: &str, x: f32, y: f32, w: f32, h: f32) {
    s.push_str(&format!("q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /{name} Do Q\n"));
}

/// Build the manual PDF. `progress` counts finished pages.
pub fn manual_pdf(model: &Model, ins: &Instructions, progress: Option<&AtomicUsize>) -> Vec<u8> {
    let renderer = Renderer::new();
    let bricks = &model.bricks;
    let step_of = ins.step_of(bricks.len());
    let n = ins.steps.len();
    let (img_w, img_h) = (1000u32, 860u32);
    let view = View::default();
    let title = if model.name.is_empty() { "My build".to_string() } else { model.name.clone() };

    // Frame on the bricks (not the whole baseplate) so details stay big.
    let mut frame: Vec<_> = bricks.iter().filter(|b| b.part != Part::Baseplate).copied().collect();
    if frame.len() < 3 {
        frame = bricks.clone();
    }
    // Step pictures, rendered in parallel (limited pool to bound memory).
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build().expect("thread pool");
    let pictures: Vec<(u32, u32, Vec<u8>)> = pool.install(|| {
        (0..=n)
            .into_par_iter()
            .map(|s| {
                // s == n is the cover picture (finished model).
                let looks: Vec<Look> = step_of
                    .iter()
                    .map(|&k| {
                        if s == n || k == s {
                            Look::Normal
                        } else if k < s {
                            Look::Dim
                        } else {
                            Look::Hidden
                        }
                    })
                    .collect();
                let img = renderer.render(bricks, &looks, &frame, img_w, img_h, &view);
                if let Some(p) = progress {
                    p.fetch_add(1, Ordering::Relaxed);
                }
                (img.width, img.height, img.to_jpeg(85))
            })
            .collect()
    });

    let mut w = Writer::new();
    let catalog = w.reserve();
    let pages_id = w.reserve();
    let font = w.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    let font_b = w.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());

    let mut icons: HashMap<(Part, u8), usize> = HashMap::new();
    let mut icon_for = |w: &mut Writer, p: Part, c: u8| -> usize {
        *icons.entry((p, c)).or_insert_with(|| {
            let img = renderer.part_icon(p, c, 96, [1.0, 1.0, 1.0]);
            w.jpeg(img.width, img.height, &img.to_jpeg(90))
        })
    };

    let mut pages: Vec<Page> = Vec::new();

    // Cover.
    {
        let (iw, ih, data) = &pictures[n];
        let im = w.jpeg(*iw, *ih, data);
        let mut c = String::new();
        rgb_fill(&mut c, [30, 60, 120]);
        c.push_str(&format!("0 {:.1} {PW} 70 re f\n", PH - 70.0));
        rgb_fill(&mut c, [255, 255, 255]);
        text(&mut c, "F2", 30.0, 36.0, PH - 47.0, &title);
        rgb_fill(&mut c, [40, 40, 40]);
        let ah = PH - 140.0;
        let aw = ah * *iw as f32 / *ih as f32;
        place_image(&mut c, "Im0", (PW - aw) * 0.5, 50.0, aw, ah);
        text(
            &mut c,
            "F1",
            14.0,
            36.0,
            24.0,
            &format!("{} bricks  -  {} steps  -  Brick Builder instructions", model.brick_count(), n),
        );
        pages.push(Page { content: c, images: vec![("Im0".into(), im)] });
    }

    for (s, step) in ins.steps.iter().enumerate() {
        let (iw, ih, data) = &pictures[s];
        let im = w.jpeg(*iw, *ih, data);
        let mut images = vec![("Im0".to_string(), im)];
        let mut c = String::new();
        // Header.
        rgb_fill(&mut c, [235, 240, 248]);
        c.push_str(&format!("0 {:.1} {PW} 56 re f\n", PH - 56.0));
        rgb_fill(&mut c, [20, 40, 90]);
        text(&mut c, "F2", 26.0, 30.0, PH - 40.0, &format!("Step {} / {}", s + 1, n));
        rgb_fill(&mut c, [90, 90, 90]);
        text(&mut c, "F1", 12.0, PW - 260.0, PH - 36.0, &title);
        // Picture.
        let ah = PH - 90.0;
        let aw = (ah * *iw as f32 / *ih as f32).min(560.0);
        let ah2 = aw * *ih as f32 / *iw as f32;
        place_image(&mut c, "Im0", 20.0, 20.0 + (ah - ah2) * 0.5, aw, ah2);
        // Parts box.
        let bx = 600.0;
        rgb_fill(&mut c, [246, 246, 246]);
        c.push_str(&format!("{bx} 20 {:.1} {:.1} re f\n", PW - bx - 20.0, PH - 96.0));
        rgb_fill(&mut c, [30, 30, 30]);
        text(&mut c, "F2", 14.0, bx + 12.0, PH - 100.0, "Parts for this step");
        let list = parts_list(step.iter().map(|&i| &bricks[i]));
        let rows = list.len();
        let row_h = if rows > 14 { (PH - 150.0) / rows as f32 } else { 30.0 };
        let icon = (row_h - 3.0).min(28.0);
        for (k, (p, col, cnt)) in list.iter().enumerate() {
            let y = PH - 132.0 - k as f32 * row_h;
            let id = icon_for(&mut w, *p, *col);
            let name = format!("Ic{k}");
            images.push((name.clone(), id));
            place_image(&mut c, &name, bx + 10.0, y - icon * 0.3, icon, icon);
            rgb_fill(&mut c, [20, 20, 20]);
            let fs = (row_h * 0.45).clamp(7.0, 13.0);
            text(&mut c, "F2", fs, bx + 16.0 + icon, y, &format!("{cnt}x"));
            rgb_fill(&mut c, color_rgb(*col));
            c.push_str(&format!("{:.1} {:.1} {:.1} {:.1} re f\n", bx + 46.0 + icon, y - 1.0, fs, fs));
            rgb_fill(&mut c, [50, 50, 50]);
            text(&mut c, "F1", fs, bx + 52.0 + icon + fs, y, &format!("{} {}", color_name(*col), p.name()));
        }
        pages.push(Page { content: c, images });
    }

    let mut kids = Vec::new();
    for p in pages {
        let content = w.add(Writer::stream("", p.content.as_bytes()));
        let xo: String = p.images.iter().map(|(n, id)| format!("/{n} {id} 0 R ")).collect();
        let page = w.add(
            format!(
                "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {PW} {PH}] /Contents {content} 0 R /Resources << /Font << /F1 {font} 0 R /F2 {font_b} 0 R >> /XObject << {xo}>> >> >>"
            )
            .into_bytes(),
        );
        kids.push(page);
    }
    let kids_s: String = kids.iter().map(|k| format!("{k} 0 R ")).collect();
    w.set(pages_id, format!("<< /Type /Pages /Kids [{kids_s}] /Count {} >>", kids.len()).into_bytes());
    w.set(catalog, format!("<< /Type /Catalog /Pages {pages_id} 0 R >>").into_bytes());
    w.finish(catalog)
}
