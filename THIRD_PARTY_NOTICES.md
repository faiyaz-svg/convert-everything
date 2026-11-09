# Third-party software

Convert Everything is licensed under the GNU General Public License, version 3 or later (see `LICENSE`). The installer includes the programs below. Each runs as a separate program; the app starts it to convert a file. Their license files are installed in `engines/licenses`, and the exact downloads are pinned with SHA-256 hashes in `scripts/engines.lock.json`.

| Program | Version | License | Source code |
|---|---|---|---|
| FFmpeg (full shared build by Gyan Doshi) | 9.0.2 | GPL-3.0-or-later | https://ffmpeg.org/download.html, build scripts at https://www.gyan.dev/ffmpeg/builds/ |
| ImageMagick | 7.1.2-35 | ImageMagick License | https://github.com/ImageMagick/ImageMagick |
| Ghostscript | 10.08.0 | AGPL-3.0-or-later | https://github.com/ArtifexSoftware/ghostpdl |
| LibreOffice | 26.8.2.2 | MPL-2.0 | https://www.libreoffice.org/about-us/source-code/ |
| Pandoc | 3.13 | GPL-2.0-or-later | https://github.com/jgm/pandoc |
| Calibre | 9.18.0 | GPL-3.0 | https://github.com/kovidgoyal/calibre |
| 7-Zip | 26.06 | LGPL-2.1-or-later, with the unRAR restriction | https://github.com/ip7z/7zip |
| qpdf | 12.4.2 | Apache-2.0 | https://github.com/qpdf/qpdf |
| resvg | 0.49.0 | Apache-2.0 or MIT | https://github.com/linebender/resvg |
| VTracer | 1.0.0-alpha.4 | MIT | https://github.com/visioncortex/vtracer |
| Python (embeddable) | 3.14.10 | PSF-2.0 | https://www.python.org/downloads/source/ |
| fontTools | 4.68.0 | MIT | https://github.com/fonttools/fonttools |
| Brotli | 1.2.1 | MIT | https://github.com/google/brotli |

Some files were removed from the LibreOffice, Calibre, ImageMagick and Ghostscript packages to save space (spelling dictionaries, translations, help files and programs the app does not use). The removed paths are listed in `scripts/engines.lock.json`. Nothing else was changed.

The app itself is built with Tauri, Svelte and Rust crates under MIT, Apache-2.0 and similar licenses. They are listed in `src-tauri/Cargo.lock` and `package-lock.json`.

The unRAR code in 7-Zip may be used to open RAR files but not to create them or to recreate the RAR compression method. Convert Everything only opens RAR files.
