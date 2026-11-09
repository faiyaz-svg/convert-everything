//! Where the bundled engine programs live.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Engines {
    pub root: PathBuf,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub name: String,
    pub title: String,
    pub version: String,
    pub license: String,
    pub source: String,
    pub found: bool,
}

impl Engines {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Engines { root: root.into() }
    }

    /// `CE_ENGINES_DIR` if set, else `src-tauri/engines` (used by tests and `tauri dev`).
    pub fn from_env_or_dev() -> Self {
        match std::env::var_os("CE_ENGINES_DIR") {
            Some(p) => Self::new(p),
            None => Self::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("engines")),
        }
    }

    fn p(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn ffmpeg(&self) -> PathBuf { self.p("ffmpeg/bin/ffmpeg.exe") }
    pub fn ffprobe(&self) -> PathBuf { self.p("ffmpeg/bin/ffprobe.exe") }
    pub fn magick(&self) -> PathBuf { self.p("imagemagick/magick.exe") }
    pub fn gs(&self) -> PathBuf { self.p("ghostscript/bin/gswin64c.exe") }
    pub fn gs_bin(&self) -> PathBuf { self.p("ghostscript/bin") }
    pub fn soffice(&self) -> PathBuf { self.p("libreoffice/program/soffice.exe") }
    pub fn pandoc(&self) -> PathBuf { self.p("pandoc/pandoc.exe") }
    pub fn ebook_convert(&self) -> PathBuf { self.p("calibre/ebook-convert.exe") }
    pub fn sevenzip(&self) -> PathBuf { self.p("7zip/7z.exe") }
    pub fn qpdf(&self) -> PathBuf { self.p("qpdf/bin/qpdf.exe") }
    pub fn resvg(&self) -> PathBuf { self.p("resvg/resvg.exe") }
    pub fn vtracer(&self) -> PathBuf { self.p("vtracer/vtracer.exe") }
    pub fn python(&self) -> PathBuf { self.p("python/python.exe") }
    pub fn licenses(&self) -> PathBuf { self.p("licenses") }

    fn main_exe(&self, name: &str) -> Option<PathBuf> {
        Some(match name {
            "ffmpeg" => self.ffmpeg(),
            "imagemagick" => self.magick(),
            "ghostscript" => self.gs(),
            "libreoffice" => self.soffice(),
            "pandoc" => self.pandoc(),
            "calibre" => self.ebook_convert(),
            "7zip" => self.sevenzip(),
            "qpdf" => self.qpdf(),
            "resvg" => self.resvg(),
            "vtracer" => self.vtracer(),
            "python" => self.python(),
            _ => return None,
        })
    }

    /// Each engine with its version (from versions.json) and whether its program exists.
    pub fn status(&self) -> Vec<EngineStatus> {
        let versions: Value = std::fs::read_to_string(self.p("versions.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        let names = ["ffmpeg", "imagemagick", "ghostscript", "libreoffice", "pandoc", "calibre", "7zip", "qpdf", "resvg", "vtracer", "python"];
        names
            .iter()
            .map(|n| {
                let v = &versions[n];
                let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
                EngineStatus {
                    name: n.to_string(),
                    title: if s("title").is_empty() { n.to_string() } else { s("title") },
                    version: s("version"),
                    license: s("license"),
                    source: s("source"),
                    found: self.main_exe(n).is_some_and(|p| p.exists()),
                }
            })
            .collect()
    }
}
