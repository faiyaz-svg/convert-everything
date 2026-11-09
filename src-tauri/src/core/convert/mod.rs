//! Runs one conversion and returns what it produced.

pub mod archive;
pub mod data;
pub mod ebook;
pub mod font;
pub mod image;
pub mod media;
pub mod office;
pub mod pandoc;
pub mod pdf;

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};

use super::formats::{Format, by_id};
use super::fsutil;
use super::options::Opts;
use super::routes::{self, Handler};
use super::run::Ctx;

/// Converts `input` (in format `from`) to `to`. Results are written to `ctx.work/out`
/// and returned, main result first. The caller moves them to their final folder.
pub async fn convert(ctx: &Ctx, input: &Path, from: &'static Format, to: &'static Format, opts: &Opts) -> Result<Vec<PathBuf>> {
    let route = routes::find(from.id, to.id).ok_or_else(|| anyhow!("Can't convert {} to {}.", from.name, to.name))?;
    if !input.exists() {
        bail!("The file is gone. It may have been moved or deleted.");
    }
    let out_dir = ctx.work.join("out");
    std::fs::create_dir_all(&out_dir)?;

    let stem = fsutil::stem_of(input, from);
    let name_stem = if from.id == to.id { format!("{stem} (converted)") } else { stem.clone() };
    let out = match to.exts.first() {
        Some(ext) => out_dir.join(format!("{name_stem}.{ext}")),
        None => out_dir.join(&name_stem),
    };

    match route.handler {
        Handler::Image => image::convert(ctx, input, from, to, opts, &out).await?,
        Handler::SvgRender => image::from_svg(ctx, input, to, opts, &out).await?,
        Handler::Trace => image::trace(ctx, input, opts, &out).await?,
        Handler::Media => media::convert(ctx, input, from, to, opts, &out).await?,
        Handler::Office => office::convert(ctx, input, from, to, &out).await?,
        Handler::OfficeToImages => {
            let pdf = ctx.tmp(&format!("{stem}.pdf"));
            office::convert(ctx, input, from, by_id("pdf").unwrap(), &pdf).await?;
            pdf::render(ctx, &pdf, to, opts, &out_dir, &stem).await?;
        }
        Handler::Pandoc => pandoc::convert(ctx, input, from, to, &stem, &out).await?,
        Handler::PandocToPdf => pandoc::to_pdf(ctx, input, from, &stem, &out).await?,
        Handler::Ebook => ebook::convert(ctx, input, from, to, opts, &out).await?,
        Handler::Archive => archive::convert(ctx, input, from, to, opts, &out).await?,
        Handler::Data => {
            let (input, out, opts) = (input.to_path_buf(), out.clone(), opts.clone());
            let (from_id, to_id) = (from.id, to.id);
            tokio::task::spawn_blocking(move || data::convert(&input, from_id, to_id, &opts, &out)).await??;
        }
        Handler::Font => font::convert(ctx, input, to, &out).await?,
        Handler::PdfRender => pdf::render(ctx, input, to, opts, &out_dir, &stem).await?,
        Handler::PdfCompress => pdf::compress(ctx, input, opts, &out).await?,
    }

    let mut produced = fsutil::entries(&out_dir);
    if produced.is_empty() {
        bail!("The conversion finished but wrote no file.");
    }
    // The main result first, extra folders (such as extracted pictures) after.
    produced.sort_by_key(|p| p.is_dir() && to.id != "folder" && !p.to_string_lossy().ends_with("(pages)"));
    if let Some(main) = produced.first()
        && main.is_file()
        && std::fs::metadata(main).map(|m| m.len()).unwrap_or(0) == 0
    {
        if from.id == "pdf" && to.id == "txt" {
            bail!("This PDF has no text to take out. It may be a scan or made of pictures.");
        }
        bail!("The conversion wrote an empty file.");
    }
    Ok(produced)
}
