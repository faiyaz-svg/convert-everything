//! File naming and moving.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::formats::Format;

/// The file name without the format's extension: "backup.tar.gz" gives "backup".
pub fn stem_of(path: &Path, fmt: &Format) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let lower = name.to_lowercase();
    for ext in fmt.exts {
        let dotted = format!(".{ext}");
        if lower.ends_with(&dotted) && name.len() > dotted.len() {
            return name[..name.len() - dotted.len()].to_string();
        }
    }
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name,
    }
}

/// Splits "photo.jpg" into ("photo", ".jpg") and "backup.tar.gz" into ("backup", ".tar.gz").
fn split_name(name: &str) -> (String, String) {
    let lower = name.to_lowercase();
    for double in [".tar.gz", ".tar.xz", ".tar.bz2"] {
        if lower.ends_with(double) && name.len() > double.len() {
            let at = name.len() - double.len();
            return (name[..at].to_string(), name[at..].to_string());
        }
    }
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// A path in `dir` named `name` that does not exist yet: "photo.jpg", then "photo (1).jpg", ...
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = split_name(name);
    (1..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .unwrap()
}

/// Moves a file or folder into `dest_dir` without replacing anything there.
/// Falls back to copy and delete when the destination is on another drive.
pub fn move_into(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dest_dir).with_context(|| format!("Could not create the folder {}", dest_dir.display()))?;
    let name = src.file_name().context("no file name")?.to_string_lossy().to_string();
    let dest = unique_path(dest_dir, &name);
    if fs::rename(src, &dest).is_err() {
        copy_all(src, &dest).with_context(|| format!("Could not save to {}", dest.display()))?;
        let _ = if src.is_dir() { fs::remove_dir_all(src) } else { fs::remove_file(src) };
    }
    Ok(dest)
}

pub fn copy_all(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_all(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        fs::copy(src, dest)?;
    }
    Ok(())
}

/// Files and folders directly inside `dir`, sorted by name.
pub fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    v.sort();
    v
}

pub fn dir_size(path: &Path) -> u64 {
    if path.is_dir() {
        walkdir::WalkDir::new(path).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
    } else {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::by_id;

    #[test]
    fn stems() {
        assert_eq!(stem_of(Path::new("C:/a/backup.tar.gz"), by_id("tgz").unwrap()), "backup");
        assert_eq!(stem_of(Path::new("C:/a/Photo.JPEG"), by_id("jpg").unwrap()), "Photo");
        assert_eq!(stem_of(Path::new("C:/a/my.notes.md"), by_id("md").unwrap()), "my.notes");
    }

    #[test]
    fn unique_names() {
        let dir = std::env::temp_dir().join("ce-unique-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_path(&dir, "a.tar.gz"), dir.join("a.tar.gz"));
        fs::write(dir.join("a.tar.gz"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "a.tar.gz"), dir.join("a (1).tar.gz"));
        fs::write(dir.join("a (1).tar.gz"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "a.tar.gz"), dir.join("a (2).tar.gz"));
        fs::create_dir_all(dir.join("folder")).unwrap();
        assert_eq!(unique_path(&dir, "folder"), dir.join("folder (1)"));
    }
}
