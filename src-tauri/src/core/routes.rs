//! Which conversions exist, which engine does each one, and what options it takes.
//!
//! Rules are checked in order and the first rule that covers a (from, to) pair wins.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Serialize;
use serde_json::{Value, json};

use super::formats::{FORMATS, Format};
use super::options::OptionSpec;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Handler {
    Image,
    SvgRender,
    Trace,
    Media,
    Office,
    OfficeToImages,
    Pandoc,
    PandocToPdf,
    Ebook,
    Archive,
    Data,
    Font,
    PdfRender,
    PdfCompress,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub from: &'static str,
    pub to: &'static str,
    pub handler: Handler,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
    pub option_set: String,
}

struct Rule {
    from: &'static [&'static str],
    to: &'static [&'static str],
    handler: Handler,
    /// Allow from == to (compress, resize or re-encode).
    same: bool,
    note: Option<&'static str>,
}

const RASTER_IN: &[&str] = &["png", "jpg", "webp", "heic", "avif", "gif", "bmp", "tiff", "ico", "psd", "jxl", "tga", "raw", "eps"];
const RASTER_OUT: &[&str] = &["png", "jpg", "webp", "avif", "gif", "bmp", "tiff", "ico", "jxl", "pdf"];
const RASTER_SAME: &[&str] = &["png", "jpg", "webp", "avif", "gif", "tiff", "jxl"];
const AUDIO_IN: &[&str] = &["mp3", "wav", "flac", "m4a", "ogg", "opus", "aiff", "wma"];
const AUDIO_OUT: &[&str] = &["mp3", "wav", "flac", "m4a", "alac", "ogg", "opus", "aiff"];
const VIDEO_IN: &[&str] = &["mp4", "mkv", "mov", "webm", "avi", "wmv", "flv", "mpeg", "ts", "3gp"];
const VIDEO_OUT: &[&str] = &["mp4", "mkv", "mov", "webm", "avi", "wmv", "flv", "mpeg", "ts", "3gp"];
const SUBS: &[&str] = &["srt", "vtt", "ass"];
const WORD_IN: &[&str] = &["docx", "doc", "odt", "rtf", "txt", "html"];
const WORD_OUT: &[&str] = &["docx", "doc", "odt", "rtf", "txt", "html", "pdf"];
const MARKUP: &[&str] = &["md", "rst", "org", "tex"];
const MARKUP_OUT: &[&str] = &["md", "html", "docx", "odt", "rst", "org", "tex", "epub", "txt"];
const RICH_IN: &[&str] = &["html", "docx", "odt", "epub", "txt"];
const SHEETS: &[&str] = &["xlsx", "xls", "ods"];
const SHEET_OUT: &[&str] = &["xlsx", "xls", "ods", "csv", "pdf", "html"];
const SLIDES: &[&str] = &["pptx", "ppt", "odp"];
const SLIDE_OUT: &[&str] = &["pptx", "ppt", "odp", "pdf"];
const EBOOK_IN: &[&str] = &["epub", "mobi", "azw3", "fb2", "pdb", "lit", "docx", "odt", "rtf", "txt", "html", "md"];
const EBOOK_OUT: &[&str] = &["epub", "mobi", "azw3", "fb2", "pdb", "pdf", "docx", "txt", "rtf"];
/// Comic books are pictures only, so they become picture-based formats.
const COMICS: &[&str] = &["cbz", "cbr"];
const COMIC_OUT: &[&str] = &["epub", "mobi", "azw3", "pdf"];
const ARCHIVE_IN: &[&str] = &["zip", "7z", "tar", "tgz", "txz", "tbz2", "rar", "iso", "cab", "gz", "bz2", "xz", "cbz", "cbr"];
const ARCHIVE_OUT: &[&str] = &["zip", "7z", "tar", "tgz", "txz", "tbz2", "folder"];
const DATA_IN: &[&str] = &["csv", "tsv", "json", "yaml", "toml", "xml", "xlsx", "xls", "ods"];
const DATA_OUT: &[&str] = &["csv", "tsv", "json", "yaml", "toml", "xml", "xlsx"];
const FONTS: &[&str] = &["ttf", "otf", "woff", "woff2"];

const LAYOUT_NOTE: &str = "Keeps the text, but the layout may shift. PDFs made from scans have no text to keep.";

fn rules() -> Vec<Rule> {
    use Handler::*;
    let r = |from, to, handler, same, note| Rule { from, to, handler, same, note };
    vec![
        r(&["svg"], &["pdf"], Office, false, None),
        r(RASTER_IN, RASTER_OUT, Image, true, None),
        r(&["svg"], RASTER_OUT, SvgRender, false, None),
        r(RASTER_IN, &["svg"], Trace, false, Some("Traces the picture into shapes. Works best on logos and drawings, not photos.")),
        r(VIDEO_IN, VIDEO_OUT, Media, true, None),
        r(VIDEO_IN, AUDIO_OUT, Media, false, Some("Saves only the sound.")),
        r(VIDEO_IN, &["gif"], Media, false, Some("GIFs get large quickly. Trim the video or lower the frame rate for a smaller file.")),
        r(&["gif"], &["mp4", "webm", "mkv", "mov"], Media, false, None),
        r(AUDIO_IN, AUDIO_OUT, Media, true, None),
        r(SUBS, SUBS, Media, false, None),
        r(&["pdf"], &["png", "jpg", "tiff"], PdfRender, false, Some("Each page becomes an image. Several pages are saved in a folder.")),
        r(&["pdf"], &["pdf"], PdfCompress, true, None),
        r(&["pdf"], &["docx", "odt"], Office, false, Some(LAYOUT_NOTE)),
        r(&["pdf"], &["epub", "mobi", "azw3", "txt"], Ebook, false, Some("Works best on PDFs that are mostly text.")),
        r(SLIDES, &["png", "jpg"], OfficeToImages, false, Some("Each slide becomes an image. Several slides are saved in a folder.")),
        r(SLIDES, SLIDE_OUT, Office, false, None),
        r(SHEETS, SHEET_OUT, Office, false, None),
        r(&["csv"], &["xls", "ods", "pdf", "html"], Office, false, None),
        r(MARKUP, MARKUP_OUT, Pandoc, false, None),
        r(RICH_IN, MARKUP, Pandoc, false, None),
        r(&["docx", "odt"], &["html"], Pandoc, false, None),
        // LibreOffice's plain-text export can loop forever on some Word files.
        r(&["docx", "odt", "html"], &["txt"], Pandoc, false, None),
        r(&["html"], &["docx", "odt"], Pandoc, false, None),
        r(MARKUP, &["pdf"], PandocToPdf, false, Some("Uses simple styling. Headings, lists, tables and images are kept.")),
        r(WORD_IN, WORD_OUT, Office, false, None),
        r(EBOOK_IN, EBOOK_OUT, Ebook, false, None),
        r(COMICS, COMIC_OUT, Ebook, false, None),
        r(ARCHIVE_IN, ARCHIVE_OUT, Archive, false, None),
        r(DATA_IN, DATA_OUT, Data, false, Some("Nested data becomes columns such as address.city when saved as a table.")),
        r(FONTS, FONTS, Font, false, None),
    ]
}

fn option_set_key(handler: Handler, from: &str, to: &str) -> String {
    use Handler::*;
    let video = |f: &str| VIDEO_IN.contains(&f);
    match handler {
        Image => format!("image:{to}"),
        SvgRender => format!("svg:{to}"),
        Trace => "trace".into(),
        Media if SUBS.contains(&to) => "none".into(),
        Media if to == "gif" => "gif".into(),
        Media if AUDIO_OUT.contains(&to) => format!("audio:{to}"),
        Media if video(from) || from == "gif" => format!("video:{to}"),
        Media => "none".into(),
        PdfRender | OfficeToImages => format!("pdfrender:{to}"),
        PdfCompress => "pdfcompress".into(),
        Ebook if to == "pdf" => "ebook:pdf".into(),
        Archive => format!("archive:{to}"),
        Data => format!("data:{from}:{to}"),
        Office | Pandoc | PandocToPdf | Ebook | Font => "none".into(),
    }
}

pub struct Registry {
    pub routes: Vec<Route>,
    pub option_sets: BTreeMap<String, Vec<OptionSpec>>,
}

pub fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(|| {
        let rules = rules();
        let mut routes = Vec::new();
        let mut option_sets = BTreeMap::new();
        for from in FORMATS.iter().map(|f| f.id) {
            for to in FORMATS.iter().map(|f| f.id) {
                let Some(rule) = rules.iter().find(|r| {
                    r.from.contains(&from) && r.to.contains(&to) && (from != to || r.same)
                }) else {
                    continue;
                };
                if from == to && rule.handler == Handler::Image && !RASTER_SAME.contains(&from) {
                    continue;
                }
                let key = option_set_key(rule.handler, from, to);
                option_sets.entry(key.clone()).or_insert_with(|| build_options(&key));
                routes.push(Route { from, to, handler: rule.handler, note: rule.note, option_set: key });
            }
        }
        Registry { routes, option_sets }
    })
}

pub fn find(from: &str, to: &str) -> Option<&'static Route> {
    registry().routes.iter().find(|r| r.from == from && r.to == to)
}

pub fn targets(from: &str) -> impl Iterator<Item = &'static Route> {
    registry().routes.iter().filter(move |r| r.from == from)
}

/// What the UI needs to know: formats, routes and option sets.
pub fn registry_json() -> Value {
    let reg = registry();
    let formats: Vec<Value> = FORMATS
        .iter()
        .map(|f: &Format| {
            let mut v = serde_json::to_value(f).unwrap();
            v["canRead"] = json!(reg.routes.iter().any(|r| r.from == f.id));
            v["canWrite"] = json!(reg.routes.iter().any(|r| r.to == f.id));
            v
        })
        .collect();
    json!({ "formats": formats, "routes": reg.routes, "optionSets": reg.option_sets })
}

// ---------------------------------------------------------------- options

const TRIM_HELP: &str = "Times as seconds, minutes:seconds or hours:minutes:seconds. Leave empty to keep the whole file.";

fn trim_options() -> Vec<OptionSpec> {
    vec![
        OptionSpec::text("trimStart", "Start at", "0:00").help(TRIM_HELP),
        OptionSpec::text("trimEnd", "End at", "end of file"),
    ]
}

fn image_options(to: &str, with_size: bool) -> Vec<OptionSpec> {
    let mut v = Vec::new();
    if ["jpg", "webp", "avif", "jxl"].contains(&to) {
        let q = if to == "avif" { 60 } else { 85 };
        v.push(OptionSpec::number("quality", "Quality", json!(q), 1.0, 100.0, Some("%")).help("Lower numbers make smaller files."));
    }
    if with_size && to != "ico" {
        v.push(OptionSpec::number("maxWidth", "Maximum width", Value::Null, 1.0, 50000.0, Some("px")).help("Leave empty to keep the size. Pictures are made smaller, never larger."));
        v.push(OptionSpec::number("maxHeight", "Maximum height", Value::Null, 1.0, 50000.0, Some("px")));
    }
    if ["jpg", "webp"].contains(&to) {
        v.push(OptionSpec::number("targetKb", "Target file size", Value::Null, 1.0, 1_000_000.0, Some("KB")).help("Lowers the quality until the file fits. Leave empty to use the quality above."));
    }
    if !["ico", "pdf", "bmp"].contains(&to) {
        v.push(OptionSpec::toggle("strip", "Remove metadata such as camera and location", false));
    }
    v
}

fn video_codecs(to: &str) -> Option<(Vec<(&'static str, &'static str)>, &'static str)> {
    let h264 = ("h264", "H.264 (plays everywhere)");
    let h265 = ("h265", "H.265 (smaller, needs newer devices)");
    let av1 = ("av1", "AV1 (smallest, slow to make)");
    let vp9 = ("vp9", "VP9");
    let copy = ("copy", "Keep the current video (fastest, no re-encode)");
    let prores = ("prores", "ProRes (for video editing, very large)");
    match to {
        "mp4" => Some((vec![h264, h265, av1, copy], "h264")),
        "mkv" => Some((vec![h264, h265, vp9, av1, copy], "h264")),
        "mov" => Some((vec![h264, h265, prores, copy], "h264")),
        "webm" => Some((vec![vp9, av1], "vp9")),
        "ts" => Some((vec![h264, h265, copy], "h264")),
        _ => None,
    }
}

fn video_options(to: &str) -> Vec<OptionSpec> {
    let mut v = Vec::new();
    if let Some((codecs, default)) = video_codecs(to) {
        v.push(OptionSpec::select("codec", "Video codec", &codecs, default));
    }
    v.push(OptionSpec::select("quality", "Quality", &[("high", "High (larger file)"), ("medium", "Medium"), ("low", "Low (small file)")], "medium"));
    v.push(OptionSpec::number("targetMb", "Target file size", Value::Null, 1.0, 100_000.0, Some("MB")).help("Picks a bitrate to land near this size. Leave empty to use the quality setting."));
    v.push(OptionSpec::select("height", "Resolution", &[("keep", "Keep"), ("2160", "4K (2160p)"), ("1440", "1440p"), ("1080", "1080p"), ("720", "720p"), ("480", "480p"), ("360", "360p")], "keep").help("Videos are made smaller, never larger."));
    v.push(OptionSpec::select("fps", "Frame rate", &[("keep", "Keep"), ("60", "60"), ("30", "30"), ("24", "24")], "keep"));
    v.push(OptionSpec::select("speed", "Encoding speed", &[("fast", "Fast (larger file)"), ("medium", "Balanced"), ("slow", "Slow (smaller file)")], "medium"));
    v.push(OptionSpec::select("audio", "Sound", &[("keep", "Keep"), ("remove", "Remove")], "keep"));
    v.extend(trim_options());
    v
}

fn audio_options(to: &str) -> Vec<OptionSpec> {
    let mut v = Vec::new();
    if ["mp3", "m4a", "ogg", "opus"].contains(&to) {
        let default = if to == "opus" { "128" } else { "192" };
        v.push(OptionSpec::select("bitrate", "Bitrate", &[("64", "64 kbps"), ("96", "96 kbps"), ("128", "128 kbps"), ("160", "160 kbps"), ("192", "192 kbps"), ("256", "256 kbps"), ("320", "320 kbps")], default).help("Higher numbers sound better and make larger files."));
    }
    v.push(OptionSpec::select("sampleRate", "Sample rate", &[("keep", "Keep"), ("44100", "44.1 kHz"), ("48000", "48 kHz")], "keep"));
    v.push(OptionSpec::select("channels", "Channels", &[("keep", "Keep"), ("1", "Mono"), ("2", "Stereo")], "keep"));
    v.push(OptionSpec::toggle("normalize", "Even out the loudness", false));
    v.extend(trim_options());
    v
}

fn build_options(key: &str) -> Vec<OptionSpec> {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        ["image", to] => image_options(to, true),
        ["svg", to] => {
            let mut v = vec![OptionSpec::number("width", "Width", Value::Null, 1.0, 50000.0, Some("px")).help("Leave empty to use the size set in the SVG.")];
            v.extend(image_options(to, false));
            v
        }
        ["trace"] => vec![
            OptionSpec::select("colors", "Colours", &[("color", "Full colour"), ("binary", "Black and white")], "color"),
            OptionSpec::select("detail", "Detail", &[("high", "High"), ("medium", "Medium"), ("low", "Low (fewer shapes)")], "medium"),
        ],
        ["video", to] => video_options(to),
        ["audio", to] => audio_options(to),
        ["gif"] => {
            let mut v = vec![
                OptionSpec::select("fps", "Frame rate", &[("10", "10"), ("15", "15"), ("20", "20"), ("25", "25")], "15"),
                OptionSpec::select("width", "Width", &[("320", "320 px"), ("480", "480 px"), ("640", "640 px"), ("800", "800 px"), ("keep", "Keep")], "480"),
            ];
            v.extend(trim_options());
            v
        }
        ["pdfrender", to] => {
            let mut v = vec![
                OptionSpec::select("dpi", "Resolution", &[("72", "72 dpi (screen)"), ("150", "150 dpi"), ("300", "300 dpi (print)"), ("600", "600 dpi")], "150"),
                OptionSpec::text("pages", "Pages", "all, or 1-3, 5").help("Leave empty for every page."),
            ];
            if *to == "jpg" {
                v.push(OptionSpec::number("quality", "Quality", json!(90), 1.0, 100.0, Some("%")));
            }
            v
        }
        ["pdfcompress"] => vec![OptionSpec::select(
            "level",
            "Compression",
            &[("screen", "Smallest file (images at 72 dpi)"), ("ebook", "Medium (images at 150 dpi)"), ("printer", "High quality (images at 300 dpi)")],
            "ebook",
        )],
        ["ebook", "pdf"] => vec![OptionSpec::select("paper", "Paper size", &[("letter", "Letter"), ("a4", "A4")], "letter")],
        ["archive", to] => {
            let mut v = Vec::new();
            if *to != "tar" && *to != "folder" {
                v.push(OptionSpec::select("level", "Compression", &[("0", "None (fastest)"), ("1", "Fast"), ("5", "Normal"), ("9", "Maximum (slowest)")], "5"));
            }
            if *to == "zip" || *to == "7z" {
                v.push(OptionSpec::text("password", "Password for the new archive", "none").help("Leave empty for no password. 7Z also hides the file names."));
            }
            v.push(OptionSpec::text("inPassword", "Password of the original archive", "only if it has one"));
            v
        }
        ["data", from, to] => {
            let mut v = Vec::new();
            if SHEETS.contains(from) {
                v.push(OptionSpec::number("sheet", "Sheet number", json!(1), 1.0, 1000.0, None));
            }
            if *from == "csv" {
                v.push(OptionSpec::select("delimiter", "Separator in the CSV", &[("auto", "Detect"), (",", "Comma"), (";", "Semicolon")], "auto"));
            }
            if (*from == "csv" || *from == "tsv") && *to != "tsv" && *to != "csv" {
                v.push(OptionSpec::toggle("types", "Read numbers and true/false as values, not text", true));
            }
            if *to == "json" {
                v.push(OptionSpec::toggle("pretty", "Indent the output so it is easy to read", true));
            }
            if *to == "csv" {
                v.push(OptionSpec::select("outDelimiter", "Separator", &[(",", "Comma"), (";", "Semicolon")], ","));
            }
            v
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::by_id;

    #[test]
    fn every_route_uses_known_formats_and_option_sets() {
        let reg = registry();
        assert!(reg.routes.len() > 300, "only {} routes", reg.routes.len());
        for r in &reg.routes {
            assert!(by_id(r.from).is_some(), "{}", r.from);
            assert!(by_id(r.to).is_some(), "{}", r.to);
            assert!(reg.option_sets.contains_key(&r.option_set), "{}", r.option_set);
        }
    }

    #[test]
    fn picks_the_expected_engine() {
        assert_eq!(find("docx", "pdf").unwrap().handler, Handler::Office);
        assert_eq!(find("docx", "md").unwrap().handler, Handler::Pandoc);
        assert_eq!(find("docx", "txt").unwrap().handler, Handler::Pandoc);
        assert_eq!(find("md", "pdf").unwrap().handler, Handler::PandocToPdf);
        assert_eq!(find("docx", "epub").unwrap().handler, Handler::Ebook);
        assert_eq!(find("csv", "xlsx").unwrap().handler, Handler::Data);
        assert_eq!(find("xlsx", "csv").unwrap().handler, Handler::Office);
        assert_eq!(find("heic", "jpg").unwrap().handler, Handler::Image);
        assert_eq!(find("svg", "pdf").unwrap().handler, Handler::Office);
        assert_eq!(find("mov", "mp3").unwrap().handler, Handler::Media);
        assert_eq!(find("pdf", "pdf").unwrap().handler, Handler::PdfCompress);
        assert_eq!(find("rar", "folder").unwrap().handler, Handler::Archive);
        assert!(find("png", "png").is_some());
        assert!(find("ico", "ico").is_none());
        assert!(find("zip", "zip").is_none());
        assert!(find("mp3", "rar").is_none());
        assert!(find("wma", "mp3").is_some());
        assert!(find("wma", "wma").is_none());
    }

    /// Writes the registry for the browser preview of the UI (src/lib/mock/registry.json).
    /// Run with: cargo test write_registry_for_ui_preview -- --ignored
    #[test]
    #[ignore]
    fn write_registry_for_ui_preview() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/mock/registry.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string(&registry_json()).unwrap()).unwrap();
    }

    #[test]
    fn nothing_writes_read_only_formats() {
        for r in &registry().routes {
            assert!(!["wma", "lit", "rar", "iso", "cab", "raw", "psd", "heic"].contains(&r.to) || r.from == r.to, "{} -> {}", r.from, r.to);
        }
    }
}
