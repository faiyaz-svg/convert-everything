//! The job queue: runs conversions in the background, a few at a time, and reports
//! progress to the window with "job-update" events.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::core::convert::{self, pdf::PagePlan};
use crate::core::engines::Engines;
use crate::core::formats::{self, Format};
use crate::core::fsutil;
use crate::core::options::Opts;
use crate::core::run::{Ctx, ProgressFn, is_canceled};

use super::store::{HistoryEntry, Store};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobUpdate {
    pub id: String,
    /// queued, running, done, failed or canceled
    pub state: &'static str,
    pub progress: Option<f64>,
    pub message: Option<String>,
    pub outputs: Vec<String>,
    pub log: Option<String>,
}

impl JobUpdate {
    fn new(id: &str, state: &'static str) -> Self {
        JobUpdate { id: id.to_string(), state, progress: None, message: None, outputs: vec![], log: None }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
    pub id: String,
    pub input: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub options: Map<String, Value>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "tool", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PdfRequest {
    Merge { id: String, inputs: Vec<String>, name: String },
    Split { id: String, input: String, ranges: String },
    Arrange { id: String, input: String, pages: Vec<PagePlan> },
    ImagesToPdf { id: String, inputs: Vec<String>, name: String },
}

pub struct Jobs {
    pub engines: Arc<Engines>,
    sem: Arc<Semaphore>,
    permits: Mutex<usize>,
    cancels: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

fn jobs_root() -> PathBuf {
    std::env::temp_dir().join("convert-everything").join("jobs")
}

fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", b / 1024.0 / 1024.0 / 1024.0)
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{bytes} bytes")
    }
}

/// A file name for a merged PDF: what the user typed, cleaned, ending in .pdf.
fn pdf_name(name: &str, fallback: &str) -> String {
    let cleaned: String = name.trim().chars().filter(|c| !"<>:\"/\\|?*".contains(*c)).collect();
    let base = if cleaned.is_empty() { fallback.to_string() } else { cleaned };
    if base.to_lowercase().ends_with(".pdf") { base } else { format!("{base}.pdf") }
}

impl Jobs {
    pub fn new(engines: Arc<Engines>, parallel: usize) -> Self {
        let n = parallel.max(1);
        // Clear folders left behind by a previous run that was closed mid-job.
        let _ = std::fs::remove_dir_all(jobs_root());
        Jobs { engines, sem: Arc::new(Semaphore::new(n)), permits: Mutex::new(n), cancels: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn set_parallel(&self, n: usize) {
        let n = n.clamp(1, 32);
        let mut cur = self.permits.lock().unwrap();
        if n > *cur {
            self.sem.add_permits(n - *cur);
        } else if n < *cur {
            self.sem.forget_permits(*cur - n);
        }
        *cur = n;
    }

    pub fn cancel(&self, id: &str) {
        if let Some(t) = self.cancels.lock().unwrap().get(id) {
            t.cancel();
        }
    }

    fn destination(app: &AppHandle, input: &Path) -> PathBuf {
        let settings = app.state::<Store>().settings();
        if settings.output_mode == "folder" && !settings.output_folder.trim().is_empty() {
            PathBuf::from(settings.output_folder.trim())
        } else {
            input.parent().map(Path::to_path_buf).unwrap_or_else(std::env::temp_dir)
        }
    }

    pub fn start_convert(&self, app: AppHandle, req: ConvertRequest) {
        let input = PathBuf::from(&req.input);
        let dest = Self::destination(&app, &input);
        let (from, to) = match (formats::by_id(&req.from), formats::by_id(&req.to)) {
            (Some(f), Some(t)) => (f, t),
            _ => {
                let mut u = JobUpdate::new(&req.id, "failed");
                u.message = Some("Unknown format.".into());
                let _ = app.emit("job-update", u);
                return;
            }
        };
        let opts = Opts::new(req.options);
        let in_size = fsutil::dir_size(&input);
        let entry = HistoryEntry::new(&input, from.id, to.id);
        self.spawn(app, req.id, dest, Some((entry, in_size)), move |ctx| async move {
            convert::convert(&ctx, &input, from, to, &opts).await
        });
    }

    pub fn start_pdf(&self, app: AppHandle, req: PdfRequest) {
        let pdf: &'static Format = formats::by_id("pdf").unwrap();
        match req {
            PdfRequest::Merge { id, inputs, name } | PdfRequest::ImagesToPdf { id, inputs, name } => {
                let inputs: Vec<PathBuf> = inputs.iter().map(PathBuf::from).collect();
                let Some(first) = inputs.first().cloned() else { return };
                let dest = Self::destination(&app, &first);
                let file = pdf_name(&name, "Merged");
                self.spawn(app, id, dest, None, move |ctx| async move {
                    let out = ctx.work.join("out").join(&file);
                    std::fs::create_dir_all(ctx.work.join("out"))?;
                    convert::pdf::merge(&ctx, &inputs, &out).await?;
                    Ok(vec![out])
                });
            }
            PdfRequest::Split { id, input, ranges } => {
                let input = PathBuf::from(input);
                let dest = Self::destination(&app, &input);
                self.spawn(app, id, dest, None, move |ctx| async move {
                    let out_dir = ctx.work.join("out");
                    std::fs::create_dir_all(&out_dir)?;
                    let stem = fsutil::stem_of(&input, pdf);
                    convert::pdf::split(&ctx, &input, &ranges, &out_dir, &stem).await?;
                    Ok(fsutil::entries(&out_dir))
                });
            }
            PdfRequest::Arrange { id, input, pages } => {
                let input = PathBuf::from(input);
                let dest = Self::destination(&app, &input);
                self.spawn(app, id, dest, None, move |ctx| async move {
                    let out_dir = ctx.work.join("out");
                    std::fs::create_dir_all(&out_dir)?;
                    let out = out_dir.join(format!("{} (rearranged).pdf", fsutil::stem_of(&input, pdf)));
                    convert::pdf::arrange(&ctx, &input, &pages, &out).await?;
                    Ok(vec![out])
                });
            }
        }
    }

    fn spawn<F, Fut>(&self, app: AppHandle, id: String, dest: PathBuf, history: Option<(HistoryEntry, u64)>, work: F)
    where
        F: FnOnce(Arc<Ctx>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Vec<PathBuf>>> + Send + 'static,
    {
        let token = CancellationToken::new();
        self.cancels.lock().unwrap().insert(id.clone(), token.clone());
        let _ = app.emit("job-update", JobUpdate::new(&id, "queued"));

        let sem = self.sem.clone();
        let engines = self.engines.clone();
        let cancels = self.cancels.clone();
        tauri::async_runtime::spawn(async move {
            let permit = tokio::select! {
                p = sem.acquire_owned() => p.ok(),
                _ = token.cancelled() => None,
            };
            if permit.is_none() {
                cancels.lock().unwrap().remove(&id);
                let _ = app.emit("job-update", JobUpdate::new(&id, "canceled"));
                return;
            }
            let _ = app.emit("job-update", JobUpdate::new(&id, "running"));

            let work_dir = jobs_root().join(&id);
            let _ = std::fs::remove_dir_all(&work_dir);
            let _ = std::fs::create_dir_all(&work_dir);

            let last = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
            let (app2, id2) = (app.clone(), id.clone());
            let progress: ProgressFn = Arc::new(move |p| {
                let mut t = last.lock().unwrap();
                if t.elapsed() >= Duration::from_millis(120) {
                    *t = Instant::now();
                    let mut u = JobUpdate::new(&id2, "running");
                    u.progress = p;
                    let _ = app2.emit("job-update", u);
                }
            });
            let ctx = Arc::new(Ctx::new(engines, work_dir.clone(), token.clone(), progress));
            let result = work(ctx.clone()).await;

            let update = match result {
                Ok(files) => {
                    let mut outputs = Vec::new();
                    let mut error = None;
                    for f in &files {
                        match fsutil::move_into(f, &dest) {
                            Ok(p) => outputs.push(p),
                            Err(e) => {
                                error = Some(format!("{e:#}"));
                                break;
                            }
                        }
                    }
                    match error {
                        Some(e) => {
                            let mut u = JobUpdate::new(&id, "failed");
                            u.message = Some(e);
                            u
                        }
                        None => {
                            let mut u = JobUpdate::new(&id, "done");
                            u.progress = Some(1.0);
                            if let (Some((entry, in_size)), Some(main)) = (history, outputs.first()) {
                                let out_size = fsutil::dir_size(main);
                                u.message = Some(format!("{} to {}", human_size(in_size), human_size(out_size)));
                                app.state::<Store>().add_history(entry.with_output(main));
                            }
                            u.outputs = outputs.iter().map(|p| p.to_string_lossy().to_string()).collect();
                            u
                        }
                    }
                }
                Err(e) if is_canceled(&e) => JobUpdate::new(&id, "canceled"),
                Err(e) => {
                    let mut u = JobUpdate::new(&id, "failed");
                    u.message = Some(e.to_string());
                    u.log = Some(ctx.log_text());
                    u
                }
            };
            let _ = std::fs::remove_dir_all(&work_dir);
            drop(permit);
            cancels.lock().unwrap().remove(&id);
            let _ = app.emit("job-update", update);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_names() {
        assert_eq!(human_size(500), "500 bytes");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(5 * 1024 * 1024 + 1), "5.0 MB");
        assert_eq!(pdf_name("  report ", "Merged"), "report.pdf");
        assert_eq!(pdf_name("a/b.PDF", "Merged"), "ab.PDF");
        assert_eq!(pdf_name("", "Merged"), "Merged.pdf");
    }
}
