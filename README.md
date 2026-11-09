# Convert Everything

A free Windows app that converts files between formats. Every conversion runs on your computer. Nothing is uploaded, and it works without an internet connection.

Drop in any file or folder. The app works out what each file is, sorts it into a tab, and lists what it can become. Pick a format and press Convert.

## What it converts

| Tab | Formats |
|---|---|
| Images | PNG, JPG, WebP, HEIC (iPhone photos), AVIF, GIF, BMP, TIFF, ICO, SVG, PSD, JPEG XL, TGA, camera RAW, EPS |
| Audio | MP3, WAV, FLAC, M4A, ALAC, OGG, Opus, AIFF, WMA |
| Video | MP4, MKV, MOV, WebM, AVI, WMV, FLV, MPEG, MPEG-TS, 3GP, video to GIF, sound only; SRT, WebVTT and ASS subtitles |
| Documents | Word (DOCX, DOC), ODT, RTF, plain text, Markdown, HTML, LaTeX, reStructuredText, Org, PDF; Excel (XLSX, XLS), ODS; PowerPoint (PPTX, PPT), ODP |
| Ebooks | EPUB, MOBI, AZW3, FB2, PDB, LIT, CBZ, CBR |
| Archives | ZIP, 7Z, TAR, TAR.GZ, TAR.XZ, TAR.BZ2, RAR, ISO, CAB, GZ, BZ2, XZ |
| Data | CSV, TSV, JSON, YAML, TOML, XML, Excel |
| Fonts | TTF, OTF, WOFF, WOFF2 |

Some formats can be read but not written: HEIC, camera RAW, PSD, WMA, LIT, RAR, ISO and CAB.

It can also:

- make images, video, audio and PDFs smaller, or resize them (pick the same format, then More options)
- trim audio and video
- merge, split, reorder and rotate PDFs, put pictures into a PDF, and save PDF pages as pictures
- add "Convert with Convert Everything" to the File Explorer right-click menu

## Download

Get the installer from the [Releases page](https://github.com/faiyaz-svg/convert-everything/releases). It is large (about 700 MB) because every converter is included, so nothing has to be downloaded later.

The installer is not signed yet, so Windows shows "Windows protected your PC" the first time. Click **More info**, then **Run anyway**. The app installs for your user account only and does not need administrator rights.

Windows 10 and 11, 64-bit.

## Known limits

- PDF to Word keeps the text, but the layout may shift. Scanned PDFs have no text to keep.
- RAR files can be opened but not created; the RAR format is not free.
- On Windows 11 the right-click entry is under "Show more options".

## How it works

The app is built with [Tauri](https://tauri.app) (Rust) and [Svelte](https://svelte.dev). The converting is done by open-source programs that ship inside the app: FFmpeg, ImageMagick, Ghostscript, LibreOffice, Pandoc, Calibre, 7-Zip, qpdf, resvg, VTracer and fontTools. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for versions and licenses.

## Building it yourself

You need Windows 10 or 11, [Node.js](https://nodejs.org) 20 or later, [Rust](https://rustup.rs), the Visual Studio C++ Build Tools, and Python 3 (used once to download fontTools).

```
npm install
npm run engines      # downloads and unpacks the converters (about 1.8 GB)
npm run tauri dev    # runs the app
npm run release      # builds the installer in src-tauri/target/release/bundle/nsis
```

Tests:

```
npm run test:rust      # unit tests
npm run test:convert   # runs every conversion through the real converters
```

`CLAUDE.md` describes the code layout, `DESIGN.md` the look, and `STYLE.md` the rules for text in the app.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
