//! The Tauri side: commands the window calls, and app start-up.

pub mod explorer;
pub mod jobs;
pub mod store;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::core::detect;
use crate::core::engines::{EngineStatus, Engines};
use crate::core::run::Ctx;
use crate::core::{convert, routes};

use jobs::{ConvertRequest, Jobs, PdfRequest};
use store::{HistoryEntry, Settings, Store};

/// Files passed on the command line (from the Explorer menu) that the window has not taken yet.
#[derive(Default)]
pub struct LaunchFiles(Mutex<Vec<String>>);

const MAX_FILES: usize = 5000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbedFile {
    path: String,
    name: String,
    size: u64,
    format: Option<&'static str>,
}

#[tauri::command]
fn get_registry() -> Value {
    routes::registry_json()
}

/// Detects the format of each file. Folders are opened and every file inside is added.
#[tauri::command]
async fn probe_files(paths: Vec<String>) -> Vec<ProbedFile> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut files: Vec<PathBuf> = Vec::new();
        for p in paths {
            let p = PathBuf::from(p);
            if p.is_dir() {
                for e in walkdir::WalkDir::new(&p).sort_by_file_name().into_iter().flatten() {
                    if e.file_type().is_file() && !e.file_name().to_string_lossy().starts_with('.') {
                        files.push(e.into_path());
                    }
                    if files.len() >= MAX_FILES {
                        break;
                    }
                }
            } else if p.is_file() {
                files.push(p);
            }
            if files.len() >= MAX_FILES {
                break;
            }
        }
        files
            .into_iter()
            .map(|p| ProbedFile {
                name: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                size: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
                format: detect::detect(&p).map(|f| f.id),
                path: p.to_string_lossy().to_string(),
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
fn start_conversions(app: AppHandle, jobs: State<'_, Jobs>, requests: Vec<ConvertRequest>) {
    for r in requests {
        jobs.start_convert(app.clone(), r);
    }
}

#[tauri::command]
fn start_pdf_tool(app: AppHandle, jobs: State<'_, Jobs>, request: PdfRequest) {
    jobs.start_pdf(app, request);
}

#[tauri::command]
fn cancel_job(jobs: State<'_, Jobs>, id: String) {
    jobs.cancel(&id);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PdfInfo {
    pages: u32,
    thumbnails: Vec<String>,
}

/// Page count and small page pictures for the PDF tools.
#[tauri::command]
async fn pdf_info(jobs: State<'_, Jobs>, path: String, thumbnails: bool) -> Result<PdfInfo, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let work = std::env::temp_dir().join("convert-everything").join(format!("info-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let ctx = Ctx::new(jobs.engines.clone(), work.clone(), CancellationToken::new(), Arc::new(|_| {}));
    let input = PathBuf::from(&path);
    let result = async {
        let pages = convert::pdf::page_count(&ctx, &input).await?;
        let thumbs = if thumbnails { convert::pdf::thumbnails(&ctx, &input).await? } else { vec![] };
        anyhow::Ok(PdfInfo { pages, thumbnails: thumbs })
    }
    .await;
    let _ = std::fs::remove_dir_all(&work);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(store: State<'_, Store>) -> Settings {
    let mut s = store.settings();
    s.explorer_menu = explorer::is_enabled();
    s
}

#[tauri::command]
fn save_settings(store: State<'_, Store>, jobs: State<'_, Jobs>, settings: Settings) -> Result<(), String> {
    let before = store.settings();
    jobs.set_parallel(settings.parallel);
    if settings.explorer_menu != explorer::is_enabled() || settings.explorer_menu != before.explorer_menu {
        explorer::set_enabled(settings.explorer_menu).map_err(|e| format!("Could not change the File Explorer menu: {e}"))?;
    }
    store.set_settings(settings);
    Ok(())
}

#[tauri::command]
fn get_history(store: State<'_, Store>) -> Vec<HistoryEntry> {
    store.history()
}

#[tauri::command]
fn clear_history(store: State<'_, Store>) {
    store.clear_history();
}

#[tauri::command]
fn engine_status(jobs: State<'_, Jobs>) -> Vec<EngineStatus> {
    jobs.engines.status()
}

#[tauri::command]
fn licenses_folder(jobs: State<'_, Jobs>) -> String {
    jobs.engines.licenses().to_string_lossy().to_string()
}

#[tauri::command]
fn take_launch_files(files: State<'_, LaunchFiles>) -> Vec<String> {
    std::mem::take(&mut *files.0.lock().unwrap())
}

fn file_args(args: impl IntoIterator<Item = String>) -> Vec<String> {
    args.into_iter().skip(1).filter(|a| !a.starts_with('-') && std::path::Path::new(a).exists()).collect()
}

fn find_engines(app: &AppHandle) -> Engines {
    if let Ok(dir) = app.path().resource_dir() {
        let bundled = dir.join("engines");
        if bundled.join("versions.json").exists() {
            return Engines::new(bundled);
        }
    }
    Engines::from_env_or_dev()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // A second copy was started (for example from the Explorer menu): hand its files to this one.
            let files = file_args(argv);
            if !files.is_empty() {
                app.state::<LaunchFiles>().0.lock().unwrap().extend(files);
                let _ = app.emit("launch-files", ());
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(LaunchFiles(Mutex::new(file_args(std::env::args()))))
        .setup(|app| {
            let handle = app.handle();
            let config = handle.path().app_config_dir()?;
            let data = handle.path().app_data_dir()?;
            let store = Store::open(&config, &data);
            let settings = store.settings();
            // Keep the menu entry in step with the setting and pointing at this copy of
            // the app. Installed builds only, so `tauri dev` never registers a debug build.
            if !cfg!(debug_assertions) {
                let _ = explorer::set_enabled(settings.explorer_menu);
            }
            let engines = Arc::new(find_engines(handle));
            app.manage(Jobs::new(engines, settings.parallel));
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_registry,
            probe_files,
            start_conversions,
            start_pdf_tool,
            cancel_job,
            pdf_info,
            get_settings,
            save_settings,
            get_history,
            clear_history,
            engine_status,
            licenses_folder,
            take_launch_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Convert Everything");
}
