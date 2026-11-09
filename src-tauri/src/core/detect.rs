//! Works out a file's format from its name and its first bytes.

use std::io::Read;
use std::path::Path;

use super::formats::{self, Category, Format};

/// Formats that are really a wrapper around other formats. When the contents say
/// "zip" but the name says ".docx", the name is more specific, so it wins.
const CONTAINERS: &[&str] = &["zip", "gz", "xz", "bz2", "tar", "mp4", "mov", "m4a", "3gp", "ogg", "xml", "html", "txt"];

pub fn detect(path: &Path) -> Option<&'static Format> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let by_name = formats::by_file_name(&name);
    let head = read_head(path);
    let by_magic = head.as_deref().and_then(sniff);
    match (by_name, by_magic) {
        (Some(n), Some(m)) if n.id == m.id => Some(n),
        (Some(n), Some(m)) => {
            if CONTAINERS.contains(&m.id) || n.category != m.category {
                Some(n)
            } else {
                // A misnamed file, such as a PNG saved as photo.jpg.
                Some(m)
            }
        }
        (Some(n), None) => Some(n),
        (None, Some(m)) => Some(m),
        (None, None) => head.as_deref().and_then(sniff_text),
    }
}

fn read_head(path: &Path) -> Option<Vec<u8>> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; 8192];
    let n = f.read(&mut buf).ok()?;
    buf.truncate(n);
    Some(buf)
}

fn sniff(head: &[u8]) -> Option<&'static Format> {
    let kind = infer::get(head)?;
    let f = formats::by_file_name(&format!("x.{}", kind.extension()))?;
    // infer reports Office files in the old binary format loosely; leave those to the name.
    if matches!(f.category, Category::Document | Category::Spreadsheet | Category::Presentation) && f.exts[0].len() == 3 {
        return None;
    }
    Some(f)
}

fn sniff_text(head: &[u8]) -> Option<&'static Format> {
    let text = std::str::from_utf8(head).ok()?;
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with("<?xml") || t.starts_with("<svg") {
        return formats::by_id(if t.contains("<svg") { "svg" } else { "xml" });
    }
    if t.starts_with('{') || t.starts_with('[') {
        return formats::by_id("json");
    }
    if t.to_lowercase().starts_with("<!doctype html") || t.to_lowercase().starts_with("<html") {
        return formats::by_id("html");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("ce-detect-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::File::create(&p).unwrap().write_all(bytes).unwrap();
        p
    }

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];

    #[test]
    fn trusts_contents_for_misnamed_images() {
        assert_eq!(detect(&temp_file("really-png.jpg", PNG)).unwrap().id, "png");
    }

    #[test]
    fn trusts_name_for_containers() {
        let zip = [b'P', b'K', 3, 4, 20, 0, 0, 0, 8, 0];
        assert_eq!(detect(&temp_file("report.docx", &zip)).unwrap().id, "docx");
        assert_eq!(detect(&temp_file("book.epub", &zip)).unwrap().id, "epub");
    }

    #[test]
    fn sniffs_files_without_extension() {
        assert_eq!(detect(&temp_file("picture", PNG)).unwrap().id, "png");
        assert_eq!(detect(&temp_file("data", b"  {\"a\": 1}")).unwrap().id, "json");
        assert!(detect(&temp_file("notes", b"hello there")).is_none());
    }

    #[test]
    fn plain_text_by_name() {
        assert_eq!(detect(&temp_file("table.csv", b"a,b\n1,2\n")).unwrap().id, "csv");
    }
}
