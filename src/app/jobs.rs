//! File dialogs and PDF export run on background threads so the app never freezes.

use super::state::{Doc, Editor, FrameRequest, Screen, Toasts, Ui};
use bevy::prelude::*;
use brick_builder::instructions::Instructions;
use brick_builder::model::Model;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub enum JobResult {
    Opened(PathBuf, Result<Model, String>),
    Saved(PathBuf, Result<(), String>),
    Pdf(Result<PathBuf, String>),
    Cancelled,
}

#[derive(Resource)]
pub struct Jobs {
    tx: Sender<JobResult>,
    rx: Mutex<Receiver<JobResult>>,
    /// Text describing a running job, if any.
    pub busy: Option<String>,
    pub progress: Arc<AtomicUsize>,
    pub total: usize,
}

impl Default for Jobs {
    fn default() -> Self {
        let (tx, rx) = channel();
        Jobs { tx, rx: Mutex::new(rx), busy: None, progress: Arc::new(AtomicUsize::new(0)), total: 0 }
    }
}

const EXT: &str = "bricks";

fn safe_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
    let n = n.trim().to_string();
    if n.is_empty() {
        "My build".into()
    } else {
        n
    }
}

impl Jobs {
    pub fn progress_text(&self) -> Option<String> {
        let b = self.busy.as_ref()?;
        if self.total > 0 {
            Some(format!("{b} {}/{}", self.progress.load(Ordering::Relaxed).min(self.total), self.total))
        } else {
            Some(b.clone())
        }
    }

    pub fn open(&mut self) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some("Opening...".into());
        self.total = 0;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new().add_filter("Brick Builder build", &[EXT]).set_title("Open a build").pick_file();
            let r = match file {
                Some(p) => {
                    let res = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| Model::from_json(&s));
                    JobResult::Opened(p, res)
                }
                None => JobResult::Cancelled,
            };
            let _ = tx.send(r);
        });
    }

    pub fn save(&mut self, model: &Model, current: Option<PathBuf>) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some("Saving...".into());
        self.total = 0;
        let tx = self.tx.clone();
        let json = model.to_json();
        let name = format!("{}.{EXT}", safe_name(&model.name));
        std::thread::spawn(move || {
            let mut d = rfd::FileDialog::new().add_filter("Brick Builder build", &[EXT]).set_title("Save build").set_file_name(&name);
            if let Some(dir) = current.as_ref().and_then(|p| p.parent()) {
                d = d.set_directory(dir);
            }
            let r = match d.save_file() {
                Some(mut p) => {
                    if p.extension().is_none() {
                        p.set_extension(EXT);
                    }
                    let res = std::fs::write(&p, json).map_err(|e| e.to_string());
                    JobResult::Saved(p, res)
                }
                None => JobResult::Cancelled,
            };
            let _ = tx.send(r);
        });
    }

    pub fn export_pdf(&mut self, model: &Model, ins: &Instructions) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some("Making PDF...".into());
        self.total = ins.len() + 1;
        self.progress.store(0, Ordering::Relaxed);
        let tx = self.tx.clone();
        let model = model.clone();
        let ins = ins.clone();
        let progress = self.progress.clone();
        let name = format!("{} - instructions.pdf", safe_name(&model.name));
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_title("Export instructions as PDF").set_file_name(&name).save_file();
            let r = match file {
                Some(mut p) => {
                    if p.extension().is_none() {
                        p.set_extension("pdf");
                    }
                    let bytes = brick_builder::pdf::manual_pdf(&model, &ins, Some(&progress));
                    JobResult::Pdf(std::fs::write(&p, bytes).map(|_| p).map_err(|e| e.to_string()))
                }
                None => JobResult::Cancelled,
            };
            let _ = tx.send(r);
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn poll_jobs(
    mut jobs: ResMut<Jobs>,
    mut doc: ResMut<Doc>,
    mut toasts: ResMut<Toasts>,
    mut ui: ResMut<Ui>,
    mut editor: ResMut<Editor>,
    mut frame: ResMut<FrameRequest>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let results: Vec<JobResult> = {
        let rx = jobs.rx.lock().unwrap();
        rx.try_iter().collect()
    };
    for r in results {
        jobs.busy = None;
        jobs.total = 0;
        match r {
            JobResult::Opened(p, Ok(mut m)) => {
                if m.name.is_empty() {
                    m.name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                }
                doc.replace(m, Some(p));
                editor.selection.clear();
                editor.view_version += 1;
                ui.screen = Screen::Editor;
                frame.0 = true;
                toasts.push(now, "Build opened");
            }
            JobResult::Opened(_, Err(e)) => toasts.push(now, format!("Could not open: {e}")),
            JobResult::Saved(p, Ok(())) => {
                doc.dirty = false;
                if let Some(stem) = p.file_stem() {
                    doc.model.name = stem.to_string_lossy().to_string();
                }
                toasts.push(now, format!("Saved {}", p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()));
                doc.path = Some(p);
            }
            JobResult::Saved(_, Err(e)) => toasts.push(now, format!("Could not save: {e}")),
            JobResult::Pdf(Ok(p)) => toasts.push(now, format!("PDF saved: {}", p.display())),
            JobResult::Pdf(Err(e)) => toasts.push(now, format!("Could not write PDF: {e}")),
            JobResult::Cancelled => {}
        }
    }
}
