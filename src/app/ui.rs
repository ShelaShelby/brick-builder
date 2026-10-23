//! The egui user interface: build picker, editor panels, instruction viewer.

use super::jobs::Jobs;
use super::state::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use brick_builder::model::parts_list;
use brick_builder::parts::{color_name, color_rgb, Part, PALETTE};
use brick_builder::raster;
use egui::{Color32, RichText, Vec2 as V2};
use std::collections::HashMap;

/// Textures created from software-rendered images.
#[derive(Resource, Default)]
pub struct UiCache {
    styled: bool,
    thumbs: Vec<egui::TextureHandle>,
    new_thumb: Option<egui::TextureHandle>,
    icons: HashMap<(Part, u8), egui::TextureHandle>,
    renderer: Option<raster::Renderer>,
}

impl UiCache {
    fn icon(&mut self, ctx: &egui::Context, part: Part, color: u8) -> egui::TextureHandle {
        let r = self.renderer.get_or_insert_with(raster::Renderer::new);
        self.icons
            .entry((part, color))
            .or_insert_with(|| {
                let img = r.part_icon(part, color, 72, [1.0, 1.0, 1.0]);
                tex(ctx, &format!("icon{part:?}{color}"), &img)
            })
            .clone()
    }
}

fn tex(ctx: &egui::Context, name: &str, img: &raster::Image) -> egui::TextureHandle {
    let ci = egui::ColorImage::from_rgba_premultiplied([img.width as usize, img.height as usize], &img.rgba_premul);
    ctx.load_texture(name, ci, egui::TextureOptions::LINEAR)
}

/// Action waiting for "discard unsaved changes?" confirmation.
#[derive(Clone)]
pub enum Pending {
    New,
    LoadBuild(usize, bool),
}

#[derive(Resource, Default)]
pub struct Confirm(pub Option<Pending>);

#[derive(SystemParam)]
pub struct AppState<'w> {
    doc: ResMut<'w, Doc>,
    editor: ResMut<'w, Editor>,
    ui: ResMut<'w, Ui>,
    manual: ResMut<'w, Manual>,
    jobs: ResMut<'w, Jobs>,
    frame: ResMut<'w, FrameRequest>,
    toasts: ResMut<'w, Toasts>,
    checked: Res<'w, Checked>,
    time: Res<'w, Time>,
    library: Res<'w, Library>,
    confirm: ResMut<'w, Confirm>,
}

const ACCENT: Color32 = Color32::from_rgb(30, 110, 220);

fn style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());
    ctx.style_mut(|s| {
        use egui::{FontFamily::Proportional, FontId, TextStyle::*};
        s.text_styles = [
            (Heading, FontId::new(24.0, Proportional)),
            (Body, FontId::new(16.0, Proportional)),
            (Button, FontId::new(16.0, Proportional)),
            (Small, FontId::new(13.0, Proportional)),
            (Monospace, FontId::new(14.0, egui::FontFamily::Monospace)),
        ]
        .into();
        s.spacing.button_padding = V2::new(12.0, 7.0);
        s.spacing.item_spacing = V2::new(8.0, 8.0);
        s.visuals.selection.bg_fill = ACCENT;
        s.visuals.widgets.inactive.corner_radius = 6.0.into();
        s.visuals.widgets.hovered.corner_radius = 6.0.into();
        s.visuals.widgets.active.corner_radius = 6.0.into();
    });
}

fn big_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(enabled, egui::Button::new(RichText::new(text).size(17.0)).min_size(V2::new(0.0, 38.0)))
}

fn accent_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(17.0).color(Color32::WHITE).strong())
            .fill(ACCENT)
            .min_size(V2::new(0.0, 38.0)),
    )
}

impl AppState<'_> {
    fn now(&self) -> f64 {
        self.time.elapsed_secs_f64()
    }

    fn do_new(&mut self) {
        self.doc.replace(empty_build(), None);
        self.editor.selection.clear();
        self.editor.view_version += 1;
        self.ui.screen = Screen::Editor;
        self.frame.0 = true;
    }

    fn load_build(&mut self, i: usize, instructions: bool) {
        let e = &self.library.entries[i];
        let model = e.model.clone();
        self.doc.replace(model, None);
        self.editor.selection.clear();
        if instructions {
            self.manual.open(&self.doc.model, Screen::Picker);
            self.ui.screen = Screen::Instructions;
        } else {
            self.ui.screen = Screen::Editor;
        }
        self.editor.view_version += 1;
        self.frame.0 = true;
    }

    fn run(&mut self, p: Pending) {
        match p {
            Pending::New => self.do_new(),
            Pending::LoadBuild(i, ins) => self.load_build(i, ins),
        }
    }

    /// Run now, or ask first if there are unsaved changes.
    fn request(&mut self, p: Pending) {
        if self.doc.dirty && self.doc.model.brick_count() > 0 {
            self.confirm.0 = Some(p);
        } else {
            self.run(p);
        }
    }

    fn make_instructions(&mut self) {
        if self.doc.model.brick_count() == 0 {
            let now = self.now();
            self.toasts.push(now, "Add some bricks first");
            return;
        }
        self.manual.open(&self.doc.model, Screen::Editor);
        self.ui.screen = Screen::Instructions;
        self.editor.selection.clear();
        self.editor.view_version += 1;
        self.frame.0 = true;
    }

    fn set_step(&mut self, s: usize) {
        if s != self.manual.step {
            self.manual.step = s;
            self.editor.view_version += 1;
        }
    }
}

pub fn ui(mut contexts: EguiContexts, mut st: AppState, mut cache: ResMut<UiCache>) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    if !cache.styled {
        style(&ctx);
        cache.styled = true;
    }
    if cache.thumbs.is_empty() {
        cache.thumbs = st.library.entries.iter().enumerate().map(|(i, e)| tex(&ctx, &format!("thumb{i}"), &e.thumb)).collect();
        let r = cache.renderer.get_or_insert_with(raster::Renderer::new);
        use brick_builder::model::Brick;
        use brick_builder::parts::col::*;
        let mut demo = brick_builder::model::Model::with_baseplates("", 1, 1);
        for (p, x, y, z, rot, c) in [
            (Part::Brick { w: 2, l: 4 }, 12, 1, 14, 1, RED),
            (Part::Brick { w: 2, l: 4 }, 14, 4, 14, 1, YELLOW),
            (Part::Brick { w: 2, l: 2 }, 15, 7, 14, 0, BLUE),
            (Part::Slope { w: 2 }, 15, 10, 14, 0, RED),
            (Part::Brick { w: 1, l: 4 }, 20, 1, 12, 0, GREEN),
            (Part::Plate { w: 2, l: 6 }, 19, 4, 12, 0, WHITE),
            (Part::Round { d: 2 }, 9, 1, 19, 0, ORANGE),
            (Part::Tile { d: 2 }, 22, 1, 20, 0, SKY),
        ] {
            demo.add(Brick::new(p, x, y, z, rot, c));
        }
        let frame: Vec<_> = demo.bricks.clone();
        let looks = vec![raster::Look::Normal; demo.bricks.len()];
        let view = raster::View { background: [0.93, 0.95, 0.98], margin: 0.08, ..Default::default() };
        let img = r.render(&demo.bricks, &looks, &frame, 360, 250, &view);
        cache.new_thumb = Some(tex(&ctx, "newthumb", &img));
    }

    // Global shortcuts.
    if !ctx.wants_keyboard_input() {
        let (ctrl, s, o, n) = ctx.input(|i| (i.modifiers.command, i.key_pressed(egui::Key::S), i.key_pressed(egui::Key::O), i.key_pressed(egui::Key::N)));
        if ctrl && s && st.ui.screen == Screen::Editor {
            let path = st.doc.path.clone();
            let model = st.doc.model.clone();
            st.jobs.save(&model, path);
        }
        if ctrl && o {
            st.jobs.open();
        }
        if ctrl && n {
            st.request(Pending::New);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && st.ui.screen == Screen::Instructions {
            st.ui.screen = st.manual.back_to;
            st.editor.view_version += 1;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            st.ui.show_help = !st.ui.show_help;
        }
    }

    match st.ui.screen {
        Screen::Picker => picker(&ctx, &mut st, &mut cache),
        Screen::Editor => editor_screen(&ctx, &mut st, &mut cache),
        Screen::Instructions => instructions_screen(&ctx, &mut st, &mut cache),
    }

    toasts(&ctx, &mut st);
    confirm_dialog(&ctx, &mut st);
    help_window(&ctx, &mut st);
    Ok(())
}

fn picker(ctx: &egui::Context, st: &mut AppState, cache: &mut UiCache) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_rgb(236, 241, 248)).inner_margin(24.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Brick Builder").size(34.0).strong().color(Color32::from_rgb(25, 50, 100)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if st.doc.model.brick_count() > 0 && accent_button(ui, "Back to my build").clicked() {
                        st.ui.screen = Screen::Editor;
                        st.editor.view_version += 1;
                    }
                    if big_button(ui, "Open file...", st.jobs.busy.is_none()).clicked() {
                        st.jobs.open();
                    }
                });
            });
            ui.label(RichText::new("Start a new build, or pick a ready-made one to build or follow step by step.").size(17.0));
            ui.add_space(12.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let card_w = 386.0;
                let gap = 18.0;
                let cols = (((ui.available_width() + gap) / (card_w + gap)).floor() as usize).max(1);
                let total = st.library.entries.len() + 1;
                let mut action = None;
                for row in 0..total.div_ceil(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = V2::new(gap, gap);
                        for k in row * cols..((row + 1) * cols).min(total) {
                            let a = if k == 0 {
                                card(ui, cache.new_thumb.as_ref(), "Empty baseplate", "Free build", "Start from scratch on a big baseplate.", |ui| {
                                    accent_button(ui, "Start building").clicked().then_some(Pending::New)
                                })
                            } else {
                                let i = k - 1;
                                let e = &st.library.entries[i];
                                let meta = format!("{}  -  {} bricks  -  {} steps", e.size, e.model.brick_count(), e.steps);
                                card(ui, cache.thumbs.get(i), e.name, &meta, e.blurb, |ui| {
                                    let mut a = None;
                                    if accent_button(ui, "Instructions").clicked() {
                                        a = Some(Pending::LoadBuild(i, true));
                                    }
                                    if big_button(ui, "Open in editor", true).clicked() {
                                        a = Some(Pending::LoadBuild(i, false));
                                    }
                                    a
                                })
                            };
                            if a.is_some() {
                                action = a;
                            }
                        }
                    });
                    ui.add_space(gap);
                }
                if let Some(p) = action {
                    st.request(p);
                }
            });
        });
}

fn card(
    ui: &mut egui::Ui,
    thumb: Option<&egui::TextureHandle>,
    title: &str,
    meta: &str,
    blurb: &str,
    buttons: impl FnOnce(&mut egui::Ui) -> Option<Pending>,
) -> Option<Pending> {
    let mut out = None;
    egui::Frame::new()
        .fill(Color32::WHITE)
        .corner_radius(12.0)
        .inner_margin(12.0)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(215)))
        .show(ui, |ui| {
            ui.set_width(360.0);
            ui.vertical(|ui| {
                if let Some(t) = thumb {
                    ui.add(egui::Image::new(t).fit_to_exact_size(V2::new(360.0, 250.0)).corner_radius(8.0));
                }
                ui.label(RichText::new(title).size(22.0).strong());
                ui.label(RichText::new(meta).size(15.0).color(Color32::from_gray(90)));
                ui.label(blurb);
                ui.horizontal(|ui| {
                    out = buttons(ui);
                });
            });
        });
    out
}

fn editor_screen(ctx: &egui::Context, st: &mut AppState, cache: &mut UiCache) {
    egui::TopBottomPanel::top("top").exact_height(56.0).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.label(RichText::new("Brick Builder").size(22.0).strong().color(Color32::from_rgb(25, 50, 100)));
            ui.separator();
            if big_button(ui, "New", true).on_hover_text("Start an empty build (Ctrl+N)").clicked() {
                st.request(Pending::New);
            }
            let idle = st.jobs.busy.is_none();
            if big_button(ui, "Open", idle).on_hover_text("Open a saved build (Ctrl+O)").clicked() {
                st.jobs.open();
            }
            if big_button(ui, "Save", idle).on_hover_text("Save this build (Ctrl+S)").clicked() {
                let (m, p) = (st.doc.model.clone(), st.doc.path.clone());
                st.jobs.save(&m, p);
            }
            ui.separator();
            if big_button(ui, "Undo", st.doc.can_undo()).on_hover_text("Ctrl+Z").clicked() {
                st.doc.undo();
                st.editor.view_version += 1;
            }
            if big_button(ui, "Redo", st.doc.can_redo()).on_hover_text("Ctrl+Y").clicked() {
                st.doc.redo();
                st.editor.view_version += 1;
            }
            ui.separator();
            if big_button(ui, "Builds", true).on_hover_text("Ready-made builds").clicked() {
                st.ui.screen = Screen::Picker;
            }
            if accent_button(ui, "Make instructions").on_hover_text("Turn your build into step-by-step instructions").clicked() {
                st.make_instructions();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if big_button(ui, "Help", true).on_hover_text("Controls (F1)").clicked() {
                    st.ui.show_help = !st.ui.show_help;
                }
                if let Some(t) = st.jobs.progress_text() {
                    ui.spinner();
                    ui.label(t);
                }
            });
        });
    });

    status_bar(ctx, st);

    egui::SidePanel::left("parts").exact_width(288.0).resizable(false).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let w = 120.0;
                for (tool, label) in [(Tool::Build, "Build  (B)"), (Tool::Select, "Select  (S)")] {
                    let on = st.editor.tool == tool;
                    let b = egui::Button::new(RichText::new(label).size(17.0).color(if on { Color32::WHITE } else { Color32::from_gray(40) }))
                        .fill(if on { ACCENT } else { Color32::from_gray(230) })
                        .min_size(V2::new(w, 40.0));
                    if ui.add(b).clicked() {
                        st.editor.tool = tool;
                    }
                }
            });
            ui.add_space(4.0);
            if st.editor.tool == Tool::Select {
                select_panel(ui, st);
                ui.separator();
            }
            ui.label(RichText::new("Colour").strong().size(18.0));
            colour_grid(ui, st);
            ui.add_space(4.0);
            ui.label(RichText::new("Part").strong().size(18.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", color_name(st.editor.color), st.editor.part.name())).color(Color32::from_gray(80)));
            });
            if ui.add(egui::Button::new("Rotate part  (R)").min_size(V2::new(244.0, 32.0))).clicked() {
                st.editor.rot = (st.editor.rot + 1) % 4;
            }
            let groups: [(&str, Vec<Part>); 5] = [
                ("Bricks", Part::all().into_iter().filter(|p| matches!(p, Part::Brick { .. })).collect()),
                ("Plates", Part::all().into_iter().filter(|p| matches!(p, Part::Plate { .. })).collect()),
                ("Round and slopes", Part::all().into_iter().filter(|p| matches!(p, Part::Round { .. } | Part::Slope { .. })).collect()),
                ("Tiles", Part::all().into_iter().filter(|p| matches!(p, Part::Tile { .. })).collect()),
                ("Baseplate", vec![Part::Baseplate]),
            ];
            for (name, parts) in groups {
                ui.label(RichText::new(name).size(14.0).color(Color32::from_gray(100)));
                egui::Grid::new(name).spacing(V2::new(5.0, 5.0)).show(ui, |ui| {
                    for (k, p) in parts.iter().enumerate() {
                        let t = cache.icon(ui.ctx(), *p, st.editor.color);
                        let on = st.editor.part == *p;
                        if part_tile(ui, &t, &p.short_name(), on).on_hover_text(p.name()).clicked() {
                            st.editor.part = *p;
                            st.editor.tool = Tool::Build;
                        }
                        if k % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
            }
            ui.add_space(10.0);
        });
    });

    // Box selection rectangle.
    if let Some((a, b)) = st.editor.box_select {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("boxsel")));
        let r = egui::Rect::from_two_pos(egui::pos2(a.x, a.y), egui::pos2(b.x, b.y));
        painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(30, 140, 255, 40));
        painter.rect_stroke(r, 0.0, egui::Stroke::new(1.5_f32, Color32::from_rgb(30, 140, 255)), egui::StrokeKind::Inside);
    }
}

/// A square part button: icon with the size underneath.
fn part_tile(ui: &mut egui::Ui, t: &egui::TextureHandle, label: &str, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(V2::new(78.0, 74.0), egui::Sense::click());
    let fill = if on {
        Color32::from_rgb(205, 225, 252)
    } else if resp.hovered() {
        Color32::from_gray(232)
    } else {
        Color32::from_gray(244)
    };
    let p = ui.painter();
    p.rect_filled(rect, 8.0, fill);
    if on {
        p.rect_stroke(rect, 8.0, egui::Stroke::new(2.0_f32, ACCENT), egui::StrokeKind::Inside);
    }
    let img = egui::Rect::from_center_size(rect.center() - V2::new(0.0, 8.0), V2::splat(50.0));
    egui::Image::new(t).paint_at(ui, img);
    p.text(rect.center_bottom() - V2::new(0.0, 9.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0), Color32::from_gray(50));
    resp
}

fn colour_grid(ui: &mut egui::Ui, st: &mut AppState) {
    egui::Grid::new("colours").min_col_width(28.0).spacing(V2::new(4.0, 4.0)).show(ui, |ui| {
        for (i, c) in PALETTE.iter().enumerate() {
            let [r, g, b] = c.rgb;
            let on = st.editor.color == i as u8;
            let (rect, resp) = ui.allocate_exact_size(V2::splat(28.0), egui::Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, 6.0, Color32::from_rgb(r, g, b));
            let stroke = if on {
                egui::Stroke::new(3.0_f32, ACCENT)
            } else if resp.hovered() {
                egui::Stroke::new(2.0_f32, Color32::from_gray(120))
            } else {
                egui::Stroke::new(1.0_f32, Color32::from_gray(170))
            };
            p.rect_stroke(rect, 6.0, stroke, egui::StrokeKind::Outside);
            let resp = resp.on_hover_text(c.name);
            if resp.clicked() {
                st.editor.color = i as u8;
                // With bricks selected, a colour click repaints them.
                if st.editor.tool == Tool::Select && !st.editor.selection.is_empty() {
                    st.doc.snapshot();
                    let sel = st.editor.selection.clone();
                    for b in st.doc.model.bricks.iter_mut().filter(|b| sel.contains(&b.id)) {
                        b.color = i as u8;
                    }
                    st.doc.changed();
                }
            }
            if i % 7 == 6 {
                ui.end_row();
            }
        }
    });
}

fn select_panel(ui: &mut egui::Ui, st: &mut AppState) {
    let n = st.editor.selection.len();
    if n == 0 {
        ui.label("Click a brick, or drag a box around several bricks.");
        return;
    }
    ui.label(RichText::new(format!("{n} selected")).strong());
    ui.label(RichText::new("Drag to move. Arrow keys nudge, PgUp/PgDn lift. Click a colour to repaint.").size(14.0));
    ui.horizontal(|ui| {
        if ui.button("Delete").clicked() {
            let ids: Vec<u32> = st.editor.selection.iter().copied().collect();
            st.doc.snapshot();
            st.doc.model.remove_ids(&ids);
            st.doc.changed();
            st.editor.set_selection(Default::default());
        }
        if ui.button("Deselect").clicked() {
            st.editor.set_selection(Default::default());
        }
    });
}

fn status_bar(ctx: &egui::Context, st: &mut AppState) {
    egui::TopBottomPanel::bottom("status").exact_height(40.0).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let a = &st.checked.analysis;
            let stat = |ui: &mut egui::Ui, label: &str, v: usize, c: Color32| {
                ui.label(RichText::new(label).color(Color32::from_gray(90)));
                ui.label(RichText::new(v.to_string()).strong().color(c));
                ui.add_space(10.0);
            };
            stat(ui, "Bricks", st.doc.model.brick_count(), Color32::from_gray(20));
            stat(ui, "Stud joints", a.joints, Color32::from_gray(20));
            stat(ui, "Weak", a.weak, if a.weak > 0 { Color32::from_rgb(230, 120, 0) } else { Color32::from_gray(20) });
            stat(ui, "Floating", a.floating, if a.floating > 0 { Color32::from_rgb(210, 20, 20) } else { Color32::from_gray(20) });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let hint = match st.editor.tool {
                    Tool::Build => "Left-click: place   R: rotate   Right-click: remove   Right-drag: orbit   Middle-drag: pan   Wheel: zoom",
                    Tool::Select => "Click / drag a box: select   Drag: move   Delete: remove   R: rotate   Right-drag: orbit",
                };
                ui.label(RichText::new(hint).size(13.0).color(Color32::from_gray(100)));
            });
        });
    });
}

fn instructions_screen(ctx: &egui::Context, st: &mut AppState, cache: &mut UiCache) {
    let n = st.manual.ins.len();
    egui::TopBottomPanel::top("top").exact_height(56.0).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let back = if st.manual.back_to == Screen::Picker { "< Builds" } else { "< Back to editor" };
            if big_button(ui, back, true).clicked() {
                st.ui.screen = st.manual.back_to;
                st.editor.view_version += 1;
            }
            ui.separator();
            let name = if st.doc.model.name.is_empty() { "My build".to_string() } else { st.doc.model.name.clone() };
            ui.label(RichText::new(format!("{name} - instructions")).size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if big_button(ui, "Help", true).clicked() {
                    st.ui.show_help = !st.ui.show_help;
                }
                if big_button(ui, "Export PDF", st.jobs.busy.is_none()).on_hover_text("One page per step with picture and parts list").clicked() {
                    let (m, ins) = (st.doc.model.clone(), st.manual.ins.clone());
                    st.jobs.export_pdf(&m, &ins);
                }
                if let Some(t) = st.jobs.progress_text() {
                    ui.spinner();
                    ui.label(t);
                }
                if st.manual.back_to == Screen::Picker && big_button(ui, "Open in editor", true).clicked() {
                    st.ui.screen = Screen::Editor;
                    st.editor.view_version += 1;
                }
            });
        });
    });

    egui::TopBottomPanel::bottom("nav").exact_height(70.0).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let step = st.manual.step;
            if ui
                .add_enabled(step > 0, egui::Button::new(RichText::new("<  Back").size(20.0)).min_size(V2::new(130.0, 48.0)))
                .clicked()
            {
                st.set_step(step - 1);
            }
            let mut s1 = step + 1;
            let w = (ui.available_width() - 160.0).max(100.0);
            ui.spacing_mut().slider_width = w;
            if ui.add(egui::Slider::new(&mut s1, 1..=n.max(1)).show_value(false)).changed() {
                st.set_step(s1 - 1);
            }
            let last = step + 1 >= n;
            let next = egui::Button::new(RichText::new("Next  >").size(20.0).color(Color32::WHITE).strong()).fill(ACCENT).min_size(V2::new(130.0, 48.0));
            if ui.add_enabled(!last, next).clicked() {
                st.set_step(step + 1);
            }
        });
    });

    egui::SidePanel::left("stepparts").exact_width(288.0).resizable(false).show(ctx, |ui| {
        ui.add_space(8.0);
        ui.label(RichText::new(format!("Step {} / {}", st.manual.step + 1, n)).size(30.0).strong().color(Color32::from_rgb(25, 50, 100)));
        ui.label(RichText::new("Left / Right arrow keys change step").size(13.0).color(Color32::from_gray(110)));
        ui.add_space(6.0);
        ui.label(RichText::new("Parts needed this step").strong().size(18.0));
        let Some(step) = st.manual.ins.steps.get(st.manual.step) else { return };
        let list = parts_list(step.iter().map(|&i| &st.doc.model.bricks[i]));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (p, c, k) in list {
                ui.horizontal(|ui| {
                    let t = cache.icon(ui.ctx(), p, c);
                    ui.add(egui::Image::new(&t).fit_to_exact_size(V2::splat(48.0)));
                    ui.label(RichText::new(format!("{k}x")).size(22.0).strong());
                    ui.vertical(|ui| {
                        ui.label(RichText::new(p.name()).size(15.0));
                        ui.horizontal(|ui| {
                            let [r, g, b] = color_rgb(c);
                            let (rect, _) = ui.allocate_exact_size(V2::splat(12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(r, g, b));
                            ui.label(RichText::new(color_name(c)).size(13.0).color(Color32::from_gray(90)));
                        });
                    });
                });
            }
            let done: usize = st.manual.ins.steps[..st.manual.step].iter().map(|s| s.len()).sum();
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{} of {} parts placed before this step", done, st.doc.model.bricks.len())).size(13.0).color(Color32::from_gray(110)));
        });
    });
}

fn toasts(ctx: &egui::Context, st: &mut AppState) {
    let now = st.now();
    st.toasts.items.retain(|(_, t)| *t > now);
    if st.toasts.items.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("toasts"))
        .anchor(egui::Align2::CENTER_BOTTOM, V2::new(0.0, -90.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            for (t, _) in &st.toasts.items {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(30, 35, 45, 230))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(t).color(Color32::WHITE).size(16.0));
                    });
            }
        });
    ctx.request_repaint();
}

fn confirm_dialog(ctx: &egui::Context, st: &mut AppState) {
    let Some(p) = st.confirm.0.clone() else { return };
    egui::Window::new("Unsaved changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, V2::ZERO)
        .show(ctx, |ui| {
            ui.label("Your current build has unsaved changes.\nReplace it anyway?");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if accent_button(ui, "Replace").clicked() {
                    st.confirm.0 = None;
                    st.run(p);
                }
                if big_button(ui, "Keep my build", true).clicked() {
                    st.confirm.0 = None;
                }
            });
        });
}

fn help_window(ctx: &egui::Context, st: &mut AppState) {
    let mut open = st.ui.show_help;
    egui::Window::new("Controls").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
        egui::Grid::new("help").spacing(V2::new(24.0, 6.0)).show(ui, |ui| {
            for (k, v) in [
                ("Left-click", "Place the part (Build tool)"),
                ("R", "Rotate part / selection"),
                ("Right-click or Delete", "Remove a brick"),
                ("Right-drag", "Orbit the view"),
                ("Middle-drag / Shift+Right-drag", "Pan"),
                ("Mouse wheel", "Zoom"),
                ("Q / E", "Turn the view"),
                ("B / S", "Build tool / Select tool"),
                ("Select tool: drag a box", "Select several bricks"),
                ("Select tool: drag a brick", "Move the selection"),
                ("Arrows, PgUp/PgDn", "Nudge the selection"),
                ("Ctrl+Z / Ctrl+Y", "Undo / Redo"),
                ("Ctrl+N / Ctrl+O / Ctrl+S", "New / Open / Save"),
                ("Instructions: Left / Right", "Previous / next step"),
            ] {
                ui.label(RichText::new(k).strong());
                ui.label(v);
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Red = floating (not connected to the baseplate).\nOrange = weak (hangs on a single stud).").size(14.0));
    });
    st.ui.show_help = open;
}

