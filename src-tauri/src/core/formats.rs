//! Every file format the app knows about.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Image,
    Audio,
    Video,
    Subtitle,
    Document,
    Spreadsheet,
    Presentation,
    Pdf,
    Ebook,
    Archive,
    Data,
    Font,
    Folder,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    /// File extensions, lower case, without the dot. The first one is used for output.
    pub exts: &'static [&'static str],
    pub description: &'static str,
}

use Category::*;

macro_rules! fmt {
    ($id:literal, $name:literal, $cat:ident, [$($ext:literal),*], $desc:literal) => {
        Format { id: $id, name: $name, category: $cat, exts: &[$($ext),*], description: $desc }
    };
}

pub static FORMATS: &[Format] = &[
    // Images
    fmt!("png", "PNG", Image, ["png"], "Lossless image with transparency. Good for screenshots and graphics."),
    fmt!("jpg", "JPG", Image, ["jpg", "jpeg", "jfif", "jpe"], "Lossy photo format that almost every program can open."),
    fmt!("webp", "WebP", Image, ["webp"], "Smaller than JPG and PNG at the same quality. Used on the web."),
    fmt!("heic", "HEIC", Image, ["heic", "heif"], "Photo format used by iPhones. Many Windows programs can't open it."),
    fmt!("avif", "AVIF", Image, ["avif"], "Newer web image format, usually smaller than WebP."),
    fmt!("gif", "GIF", Image, ["gif"], "Old image format with 256 colours. Supports animation."),
    fmt!("bmp", "BMP", Image, ["bmp", "dib"], "Uncompressed Windows bitmap. Large files."),
    fmt!("tiff", "TIFF", Image, ["tiff", "tif"], "Lossless image used for scans and printing. Can hold several pages."),
    fmt!("ico", "ICO", Image, ["ico"], "Windows icon. Holds several sizes of the same picture."),
    fmt!("svg", "SVG", Image, ["svg"], "Vector drawing made of shapes, so it stays sharp at any size."),
    fmt!("psd", "PSD", Image, ["psd"], "Photoshop document. The app reads the flattened picture."),
    fmt!("jxl", "JPEG XL", Image, ["jxl"], "Newer image format that can store JPG files losslessly in less space."),
    fmt!("tga", "TGA", Image, ["tga"], "Targa image, used by older games and 3D tools."),
    fmt!("raw", "Camera RAW", Image, ["cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "raf", "srw", "pef"], "Unprocessed photo from a digital camera."),
    fmt!("eps", "EPS", Image, ["eps", "ps"], "PostScript drawing used in print design."),
    // Audio
    fmt!("mp3", "MP3", Audio, ["mp3"], "Lossy audio that plays everywhere."),
    fmt!("wav", "WAV", Audio, ["wav"], "Uncompressed audio. Large files, full quality."),
    fmt!("flac", "FLAC", Audio, ["flac"], "Lossless compressed audio, about half the size of WAV."),
    fmt!("m4a", "M4A (AAC)", Audio, ["m4a", "aac"], "Lossy audio used by Apple devices and most phones."),
    fmt!("alac", "ALAC (M4A)", Audio, ["m4a"], "Apple's lossless audio, stored in an M4A file."),
    fmt!("ogg", "OGG Vorbis", Audio, ["ogg", "oga"], "Free lossy audio format."),
    fmt!("opus", "Opus", Audio, ["opus"], "Modern lossy audio. Good quality at low bitrates."),
    fmt!("aiff", "AIFF", Audio, ["aiff", "aif", "aifc"], "Uncompressed audio from Apple."),
    fmt!("wma", "WMA", Audio, ["wma"], "Windows Media Audio. The app reads it but does not write it."),
    // Video
    fmt!("mp4", "MP4", Video, ["mp4", "m4v"], "The most widely supported video format."),
    fmt!("mkv", "MKV", Video, ["mkv"], "Matroska video. Holds many audio and subtitle tracks."),
    fmt!("mov", "MOV", Video, ["mov", "qt"], "QuickTime video from Apple devices and cameras."),
    fmt!("webm", "WebM", Video, ["webm"], "Free video format for the web."),
    fmt!("avi", "AVI", Video, ["avi"], "Old Windows video format."),
    fmt!("wmv", "WMV", Video, ["wmv", "asf"], "Windows Media Video."),
    fmt!("flv", "FLV", Video, ["flv"], "Flash video, from older websites."),
    fmt!("mpeg", "MPEG", Video, ["mpg", "mpeg", "vob"], "MPEG-1 and MPEG-2 video, used on DVDs."),
    fmt!("ts", "MPEG-TS", Video, ["ts", "mts", "m2ts"], "Transport stream from TV recordings and camcorders."),
    fmt!("3gp", "3GP", Video, ["3gp", "3g2"], "Video from older mobile phones."),
    // Subtitles
    fmt!("srt", "SRT", Subtitle, ["srt"], "SubRip subtitles. Plain text with times."),
    fmt!("vtt", "WebVTT", Subtitle, ["vtt"], "Subtitles for web video players."),
    fmt!("ass", "ASS", Subtitle, ["ass", "ssa"], "Subtitles with styling and positions."),
    // Documents
    fmt!("docx", "Word (DOCX)", Document, ["docx"], "Microsoft Word document."),
    fmt!("doc", "Word 97 (DOC)", Document, ["doc"], "Older Microsoft Word document."),
    fmt!("odt", "ODT", Document, ["odt"], "OpenDocument text, used by LibreOffice."),
    fmt!("rtf", "RTF", Document, ["rtf"], "Rich Text Format. Simple formatted text."),
    fmt!("txt", "Plain text", Document, ["txt", "text"], "Text with no formatting."),
    fmt!("md", "Markdown", Document, ["md", "markdown"], "Plain text with simple marks for headings, lists and links."),
    fmt!("html", "HTML", Document, ["html", "htm", "xhtml"], "Web page."),
    fmt!("tex", "LaTeX", Document, ["tex", "latex"], "Source for typeset documents and papers."),
    fmt!("rst", "reStructuredText", Document, ["rst"], "Plain text markup used in Python documentation."),
    fmt!("org", "Org", Document, ["org"], "Emacs Org mode notes."),
    fmt!("pdf", "PDF", Pdf, ["pdf"], "Fixed-layout document that looks the same everywhere."),
    // Spreadsheets
    fmt!("xlsx", "Excel (XLSX)", Spreadsheet, ["xlsx", "xlsm"], "Microsoft Excel workbook."),
    fmt!("xls", "Excel 97 (XLS)", Spreadsheet, ["xls"], "Older Microsoft Excel workbook."),
    fmt!("ods", "ODS", Spreadsheet, ["ods"], "OpenDocument spreadsheet, used by LibreOffice."),
    // Presentations
    fmt!("pptx", "PowerPoint (PPTX)", Presentation, ["pptx"], "Microsoft PowerPoint presentation."),
    fmt!("ppt", "PowerPoint 97 (PPT)", Presentation, ["ppt", "pps"], "Older PowerPoint presentation."),
    fmt!("odp", "ODP", Presentation, ["odp"], "OpenDocument presentation, used by LibreOffice."),
    // Ebooks
    fmt!("epub", "EPUB", Ebook, ["epub"], "Standard ebook format. Works on most readers except older Kindles."),
    fmt!("mobi", "MOBI", Ebook, ["mobi", "prc"], "Older Kindle ebook format."),
    fmt!("azw3", "AZW3", Ebook, ["azw3", "azw", "kf8"], "Kindle ebook format."),
    fmt!("fb2", "FB2", Ebook, ["fb2"], "FictionBook, popular in Eastern Europe."),
    fmt!("pdb", "PDB", Ebook, ["pdb"], "Palm ebook format."),
    fmt!("lit", "LIT", Ebook, ["lit"], "Microsoft Reader ebook. The app reads it but does not write it."),
    fmt!("cbz", "CBZ", Ebook, ["cbz"], "Comic book: a ZIP file of page images."),
    fmt!("cbr", "CBR", Ebook, ["cbr"], "Comic book: a RAR file of page images."),
    // Archives
    fmt!("zip", "ZIP", Archive, ["zip"], "The most common archive format."),
    fmt!("7z", "7Z", Archive, ["7z"], "7-Zip archive. Usually smaller than ZIP."),
    fmt!("tar", "TAR", Archive, ["tar"], "Uncompressed archive used on Linux."),
    fmt!("tgz", "TAR.GZ", Archive, ["tar.gz", "tgz"], "TAR archive compressed with gzip."),
    fmt!("txz", "TAR.XZ", Archive, ["tar.xz", "txz"], "TAR archive compressed with xz."),
    fmt!("tbz2", "TAR.BZ2", Archive, ["tar.bz2", "tbz2", "tbz"], "TAR archive compressed with bzip2."),
    fmt!("rar", "RAR", Archive, ["rar"], "RAR archive. The app opens it but can't create it."),
    fmt!("iso", "ISO", Archive, ["iso"], "Disc image."),
    fmt!("cab", "CAB", Archive, ["cab"], "Windows cabinet archive."),
    fmt!("gz", "GZ", Archive, ["gz"], "One file compressed with gzip."),
    fmt!("bz2", "BZ2", Archive, ["bz2"], "One file compressed with bzip2."),
    fmt!("xz", "XZ", Archive, ["xz"], "One file compressed with xz."),
    // Data
    fmt!("csv", "CSV", Data, ["csv"], "Table as plain text with commas between values."),
    fmt!("tsv", "TSV", Data, ["tsv", "tab"], "Table as plain text with tabs between values."),
    fmt!("json", "JSON", Data, ["json"], "Structured data used by web APIs and programs."),
    fmt!("yaml", "YAML", Data, ["yaml", "yml"], "Structured data that is easy to read and edit."),
    fmt!("toml", "TOML", Data, ["toml"], "Settings file format used by many programs."),
    fmt!("xml", "XML", Data, ["xml"], "Structured data with tags."),
    // Fonts
    fmt!("ttf", "TTF", Font, ["ttf"], "TrueType font."),
    fmt!("otf", "OTF", Font, ["otf"], "OpenType font with PostScript outlines."),
    fmt!("woff", "WOFF", Font, ["woff"], "Compressed font for websites."),
    fmt!("woff2", "WOFF2", Font, ["woff2"], "Smaller compressed font for websites."),
    // Output only
    fmt!("folder", "Folder (extract)", Folder, [], "Unpacks the archive into a folder."),
];

pub fn by_id(id: &str) -> Option<&'static Format> {
    FORMATS.iter().find(|f| f.id == id)
}

/// Finds the format for a file name. Checks two-part extensions such as
/// `.tar.gz` before single ones. ALAC never matches by name (`.m4a` is AAC).
pub fn by_file_name(name: &str) -> Option<&'static Format> {
    let lower = name.to_lowercase();
    let mut best: Option<(&'static Format, usize)> = None;
    for f in FORMATS {
        if f.id == "alac" {
            continue;
        }
        for ext in f.exts {
            let dotted = format!(".{ext}");
            if lower.ends_with(&dotted) && best.is_none_or(|(_, len)| ext.len() > len) {
                best = Some((f, ext.len()));
            }
        }
    }
    best.map(|(f, _)| f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = FORMATS.iter().map(|f| f.id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len());
    }

    #[test]
    fn finds_double_extensions() {
        assert_eq!(by_file_name("backup.tar.gz").unwrap().id, "tgz");
        assert_eq!(by_file_name("notes.GZ").unwrap().id, "gz");
        assert_eq!(by_file_name("Photo.JPEG").unwrap().id, "jpg");
        assert_eq!(by_file_name("song.m4a").unwrap().id, "m4a");
        assert!(by_file_name("README").is_none());
    }
}
