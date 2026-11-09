//! Settings and conversion history, saved as JSON in the app's data folders.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// system, light or dark
    pub theme: String,
    /// "source" saves next to the original file, "folder" saves in output_folder.
    pub output_mode: String,
    pub output_folder: String,
    pub parallel: usize,
    pub check_updates: bool,
    pub explorer_menu: bool,
    /// The last target picked for each input format, so the app remembers it.
    pub last_targets: Map<String, Value>,
}

pub fn default_parallel() -> usize {
    std::thread::available_parallelism().map(|n| (n.get() / 2).clamp(1, 8)).unwrap_or(2)
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "system".into(),
            output_mode: "source".into(),
            output_folder: String::new(),
            parallel: default_parallel(),
            check_updates: true,
            explorer_menu: true,
            last_targets: Map::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub time: u64,
    pub input: String,
    pub output: String,
    pub from: String,
    pub to: String,
}

impl HistoryEntry {
    pub fn new(input: &Path, from: &str, to: &str) -> Self {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        HistoryEntry { time, input: input.to_string_lossy().to_string(), output: String::new(), from: from.into(), to: to.into() }
    }

    pub fn with_output(mut self, output: &Path) -> Self {
        self.output = output.to_string_lossy().to_string();
        self
    }
}

const HISTORY_MAX: usize = 100;

pub struct Store {
    settings_path: PathBuf,
    history_path: PathBuf,
    settings: Mutex<Settings>,
    history: Mutex<Vec<HistoryEntry>>,
}

fn load<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save<T: Serialize>(path: &Path, value: &T) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(value) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

impl Store {
    pub fn open(config_dir: &Path, data_dir: &Path) -> Self {
        let settings_path = config_dir.join("settings.json");
        let history_path = data_dir.join("history.json");
        Store {
            settings: Mutex::new(load(&settings_path)),
            history: Mutex::new(load(&history_path)),
            settings_path,
            history_path,
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set_settings(&self, s: Settings) {
        save(&self.settings_path, &s);
        *self.settings.lock().unwrap() = s;
    }

    pub fn history(&self) -> Vec<HistoryEntry> {
        self.history.lock().unwrap().clone()
    }

    pub fn add_history(&self, e: HistoryEntry) {
        let mut h = self.history.lock().unwrap();
        h.insert(0, e);
        h.truncate(HISTORY_MAX);
        save(&self.history_path, &*h);
    }

    pub fn clear_history(&self) {
        let mut h = self.history.lock().unwrap();
        h.clear();
        save(&self.history_path, &*h);
    }
}
